use crate::tracking::{Frame, Point, Tracker};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct ObservedHand {
    side: String,
    points: Vec<Point>,
}
#[derive(Deserialize)]
pub struct Input {
    #[serde(flatten)]
    frame: Frame,
    #[serde(default)]
    hands: Vec<ObservedHand>,
    #[serde(default)]
    scroll_clutch: bool,
    #[serde(default = "sensitivity")]
    sensitivity: f64,
    #[serde(default)]
    click_method: String,
    #[serde(default = "pointer_gain")]
    pointer_gain: f64,
    #[serde(default = "smoothness")]
    smoothing: f64,
}
fn smoothness() -> f64 {
    0.65
}
fn pointer_gain() -> f64 {
    4.
}
fn sensitivity() -> f64 {
    0.32
}
#[derive(Default)]
pub struct Engine {
    right: Tracker,
    left: Tracker,
    motion: Motion,
    learned: crate::learned::Library,
}
impl Engine {
    pub fn reset_motion(&mut self) {
        self.right = Tracker::default();
        self.left = Tracker::default();
        self.motion = Motion::default();
        self.learned.rearm();
    }
    pub fn command(&mut self, c: Value) -> Value {
        self.learned.command(c)
    }
    pub fn update(&mut self, mut input: Input) -> Value {
        // Ambiguous chirality is never assigned by screen position. Duplicate labels
        // suppress that side, rather than transferring control to the other hand.
        let take = |side: &str| {
            let mut matching = input.hands.iter().filter(|h| h.side == side);
            let first = matching.next();
            if matching.next().is_some() {
                None
            } else {
                first.map(|h| h.points.clone())
            }
        };
        let right = take("right");
        let left = take("left");
        let time = input.frame.time;
        let aspect = input.frame.width / input.frame.height;
        input.frame.hand = left.clone();
        let left_result = self.left.update(Frame {
            time,
            width: input.frame.width,
            height: input.frame.height,
            hand: left.clone(),
            face: None,
            threshold: input.frame.threshold,
        });
        input.frame.hand = right.clone();
        let right_result = self.right.update(input.frame);
        let learned = self
            .learned
            .update(time, right.as_deref(), left.as_deref(), aspect);
        let mut control = self.motion.update(
            time,
            right.as_deref(),
            aspect,
            left_result.holding && left_result.state == "held",
            MotionSettings {
                clutch_mode: input.scroll_clutch,
                sensitivity: input.sensitivity,
                pinch_method: input.click_method == "pinch",
                pointer_gain: input.pointer_gain,
                smoothing: input.smoothing,
            },
        );
        if learned["training"] == true
            || (learned.get("match").is_some() && learned["action"] != "none")
        {
            control["click"] = json!(false);
            control["scroll"] = json!(0.);
            control["delta"] = json!([0., 0.]);
        }
        let mut result = serde_json::to_value(&right_result).unwrap();
        result["left"] = serde_json::to_value(left_result).unwrap();
        result["right"] = serde_json::to_value(right_result).unwrap();
        result["control"] = control;
        result["learned"] = learned;
        result
    }
}
struct MotionSettings {
    smoothing: f64,
    clutch_mode: bool,
    sensitivity: f64,
    pinch_method: bool,
    pointer_gain: f64,
}
impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            smoothing: 0.65,
            clutch_mode: false,
            sensitivity: 0.32,
            pinch_method: false,
            pointer_gain: 4.,
        }
    }
}
#[derive(Default)]
struct Motion {
    last: Option<f64>,
    center: Option<[f64; 2]>,
    stable_since: Option<f64>,
    pointer: Option<[f64; 2]>,
    motion_anchor: Option<[f64; 2]>,
    filtered: Option<[f64; 2]>,
    click: crate::click::ClickDetector,
    scroll_y: Option<f64>,
    scrolling_since: Option<f64>,
}
fn valid(p: Point) -> bool {
    p.confidence.is_finite()
        && p.confidence >= 0.5
        && p.x.is_finite()
        && p.y.is_finite()
        && (0.0..=1.0).contains(&p.x)
        && (0.0..=1.0).contains(&p.y)
}
fn dist(a: Point, b: Point, aspect: f64) -> f64 {
    ((a.x - b.x) * aspect).hypot(a.y - b.y)
}
impl Motion {
    fn reset(&mut self) {
        *self = Self::default();
    }
    fn update(
        &mut self,
        now: f64,
        points: Option<&[Point]>,
        aspect: f64,
        left_held: bool,
        settings: MotionSettings,
    ) -> Value {
        let MotionSettings {
            smoothing,
            clutch_mode,
            sensitivity,
            pinch_method,
            pointer_gain,
        } = settings;
        let lost = || json!({"mode":"lost","reason":"Pravá ruka chybí nebo nemá spolehlivé body","click":false,"scroll":0});
        let Some(p) =
            points.filter(|p| p.len() == 21 && [0, 5, 8, 9, 17].iter().all(|&i| valid(p[i])))
        else {
            self.reset();
            return lost();
        };
        if !now.is_finite() || !aspect.is_finite() || aspect <= 0. {
            self.reset();
            return lost();
        }
        let scale = dist(p[5], p[17], aspect);
        if scale < 0.025 {
            self.reset();
            return lost();
        }
        let center = [
            (p[0].x + p[5].x + p[9].x + p[17].x) / 4.,
            (p[0].y + p[5].y + p[9].y + p[17].y) / 4.,
        ];
        let dt = self.last.map_or(0., |last| now - last);
        let jump = self
            .center
            .is_some_and(|c| ((center[0] - c[0]) * aspect).hypot(center[1] - c[1]) > 0.18);
        if dt <= 0. || dt > 0.25 || jump {
            self.reset();
        }
        self.last = Some(now);
        self.center = Some(center);
        let stable = *self.stable_since.get_or_insert(now);
        if now - stable < 0.3 {
            return json!({"mode":"acquiring","reason":"Potvrzuji pravou ruku (0,3 s)","click":false,"scroll":0});
        }
        let extension = |base: usize, tip: usize| dist(p[base], p[tip], aspect) / scale;
        let index = extension(5, 8);
        let middle = extension(9, 12);
        let ring = extension(13, 16);
        let little = extension(17, 20);
        let fingers_visible = [12, 16, 20].iter().all(|&i| valid(p[i]));
        let two_fingers =
            fingers_visible && index > 0.95 && middle > 0.95 && ring < 1.05 && little < 1.05;
        let scroll_pose = if clutch_mode {
            left_held && index > 0.65
        } else {
            two_fingers
        };
        if scroll_pose {
            self.filtered = Some(center);
            self.motion_anchor = Some(center);
            self.click = Default::default();
            let since = *self.scrolling_since.get_or_insert(now);
            let y = (p[8].y + p[12].y) / 2.;
            let delta = if now - since >= 0.18 {
                self.scroll_y
                    .map_or(0., |previous| ((y - previous) * 1200.).clamp(-60., 60.))
            } else {
                0.
            };
            self.scroll_y = Some(y);
            return json!({"mode":"scroll","reason":if clutch_mode {"Levá drží sevření · pravá scrolluje"} else {"Dva prsty · pohyb nahoru/dolů scrolluje"},"click":false,"scroll":delta,"extension":index});
        }
        self.scroll_y = None;
        self.scrolling_since = None;
        let pointing = fingers_visible && middle < 1.05 && ring < 1.05 && little < 1.05;
        let sensitivity = if sensitivity.is_finite() {
            sensitivity.clamp(0.2, 0.55)
        } else {
            0.32
        };
        let pinch = if valid(p[4]) {
            Some(dist(p[4], p[8], aspect) / scale)
        } else {
            None
        };
        let click = self
            .click
            .update(now, index, pointing, pinch, sensitivity, pinch_method);
        // A local hand movement drives relative cursor velocity, like a trackpad.
        // Reacquisition establishes a new anchor without warping the system cursor.
        let previous_filtered = self.filtered.unwrap_or(center);
        let smoothing = if smoothing.is_finite() {
            smoothing.clamp(0., 1.)
        } else {
            0.65
        };
        let speed = (center[0] - previous_filtered[0]).hypot(center[1] - previous_filtered[1])
            / dt.max(0.01);
        let tau = (0.015 + smoothing * 0.12) / (1. + speed * 12.);
        let alpha = 1. - (-dt.max(0.01) / tau).exp();
        let center = [
            previous_filtered[0] + alpha * (center[0] - previous_filtered[0]),
            previous_filtered[1] + alpha * (center[1] - previous_filtered[1]),
        ];
        self.filtered = Some(center);
        let anchor = self.motion_anchor.unwrap_or(center);
        let dx = anchor[0] - center[0];
        let dy = anchor[1] - center[1];
        let travel = dx.hypot(dy);
        let gain = if pointer_gain.is_finite() {
            pointer_gain.clamp(1., 8.)
        } else {
            4.
        };
        let acceleration = 1. + (travel / dt.max(0.01) / 0.5).clamp(0., 1.5);
        let mut delta = [0., 0.];
        if travel >= 0.0015 {
            delta = [
                (dx * gain * acceleration).clamp(-0.18, 0.18),
                (dy * gain * acceleration).clamp(-0.18, 0.18),
            ];
            self.motion_anchor = Some(center);
        } else if self.motion_anchor.is_none() {
            self.motion_anchor = Some(center);
        }
        if click.stage == "pressing" || click.click {
            delta = [0., 0.];
            self.motion_anchor = Some(center);
        }
        let previous = self.pointer.unwrap_or([0.5, 0.5]);
        let pointer = [
            (previous[0] + delta[0]).clamp(0., 1.),
            (previous[1] + delta[1]).clamp(0., 1.),
        ];
        self.pointer = Some(pointer);
        let reason = if !pinch_method && !fingers_visible {
            "Klik čeká: ukaž prostředník, prsteník a malík"
        } else {
            click.reason
        };
        json!({"mode":"pointer","reason":reason,"pointer":pointer,"delta":delta,"click":click.click,"scroll":0,"extension":index,"armed":click.armed,"click_feedback":click,"pinch":pinch,"left_clutch":left_held})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pose(index: f64, middle: f64) -> Vec<Point> {
        let mut p = vec![
            Point {
                x: 0.5,
                y: 0.4,
                confidence: 0.95
            };
            21
        ];
        p[5].x = 0.4;
        p[17].x = 0.6;
        p[8] = Point {
            y: 0.4 + index * 0.2,
            ..p[5]
        };
        p[12] = Point {
            y: 0.4 + middle * 0.2,
            ..p[9]
        };
        p
    }
    fn step(m: &mut Motion, t: f64, p: Option<&[Point]>) -> Value {
        m.update(t, p, 1., false, MotionSettings::default())
    }
    fn prime(m: &mut Motion, p: &[Point]) {
        for t in [0., 0.1, 0.2, 0.31, 0.42, 0.54] {
            step(m, t, Some(p));
        }
    }
    #[test]
    fn push_click_requires_pose_dwell_and_rearm() {
        let mut m = Motion::default();
        let extended = pose(1.4, 0.3);
        let pushed = pose(0.7, 0.3);
        prime(&mut m, &extended);
        assert_eq!(step(&mut m, 0.6, Some(&pushed))["click"], false);
        assert_eq!(step(&mut m, 0.67, Some(&pushed))["click"], true);
        assert_eq!(step(&mut m, 0.7, Some(&pushed))["click"], false);
        for t in [0.8, 0.9, 1., 1.1, 1.2] {
            step(&mut m, t, Some(&extended));
        }
        assert_eq!(step(&mut m, 1.25, Some(&pushed))["click"], false);
        for t in [1.3, 1.4, 1.55, 1.65] {
            step(&mut m, t, Some(&extended));
        }
        assert_eq!(step(&mut m, 1.7, Some(&pushed))["click"], false);
        assert_eq!(step(&mut m, 1.77, Some(&pushed))["click"], true);
    }
    #[test]
    fn loss_jump_and_missing_joints_cannot_click() {
        let mut m = Motion::default();
        let p = pose(1.4, 0.3);
        prime(&mut m, &p);
        step(&mut m, 0.6, None);
        assert_eq!(step(&mut m, 0.7, Some(&pose(0.7, 0.3)))["click"], false);
        prime(&mut m, &p);
        let mut jump = pose(0.7, 0.3);
        for point in &mut jump {
            point.x += 0.25;
        }
        assert_eq!(step(&mut m, 0.6, Some(&jump))["mode"], "acquiring");
        let mut bad = p;
        bad[8].confidence = 0.1;
        assert_eq!(step(&mut m, 0.7, Some(&bad))["mode"], "lost");
    }
    #[test]
    fn scrolling_freezes_pointer_and_never_clicks() {
        let mut m = Motion::default();
        let mut p = pose(1.4, 1.4);
        prime(&mut m, &p);
        for i in [8, 12] {
            p[i].y += 0.02;
        }
        let o = step(&mut m, 0.65, Some(&p));
        assert_eq!(o["mode"], "scroll");
        assert!(o["scroll"].as_f64().unwrap() > 0.);
        assert!(o.get("pointer").is_none());
        assert_eq!(o["click"], false);
    }
    #[test]
    fn left_hand_alone_and_duplicate_right_never_drive_cursor() {
        let mut engine = Engine::default();
        for sides in [vec!["left"], vec!["right", "right"], vec!["unknown"]] {
            let input = json!({"time":1.,"width":1000.,"height":1000.,"hands":sides.iter().map(|side|json!({"side":side,"points":pose(1.4,0.3)})).collect::<Vec<_>>()});
            assert_eq!(
                engine.update(serde_json::from_value(input).unwrap())["control"]["mode"],
                "lost"
            );
        }
    }
    #[test]
    fn both_hands_keep_independent_grabs_when_observation_order_changes() {
        let mut engine = Engine::default();
        let mut open = pose(1.4, 1.4);
        open[4].x = 0.2;
        open[4].y = 0.6;
        let mut closed = open.clone();
        closed[4].x = 0.5;
        closed[4].y = 0.6;
        closed[8] = closed[4];
        closed[12] = closed[4];
        let mut update = |time: f64, left: Vec<Point>, reverse: bool| {
            let mut hands = vec![
                json!({"side":"left","points":left}),
                json!({"side":"right","points":open}),
            ];
            if reverse {
                hands.reverse();
            }
            engine.update(
                serde_json::from_value(
                    json!({"time":time,"width":1000.,"height":1000.,"hands":hands}),
                )
                .unwrap(),
            )
        };
        update(0., open.clone(), false);
        update(0.11, open.clone(), true);
        update(0.2, closed.clone(), false);
        let output = update(0.36, closed, true);
        assert_eq!(output["left"]["event"], "grab");
        assert_eq!(output["left"]["holding"], true);
        assert_eq!(output["right"]["state"], "ready");
        assert_eq!(output["right"]["holding"], false);
    }
    #[test]
    fn relative_motion_has_no_initial_warp_and_accelerates_small_movements() {
        let mut slow = Motion::default();
        let mut fast = Motion::default();
        let p = pose(1.4, 0.3);
        prime(&mut slow, &p);
        prime(&mut fast, &p);
        let initial = step(&mut slow, 0.64, Some(&p));
        assert_eq!(initial["delta"], json!([0., 0.]));
        let shifted = |amount: f64| {
            let mut q = p.clone();
            for point in &mut q {
                point.x -= amount;
            }
            q
        };
        let small = step(&mut slow, 0.74, Some(&shifted(0.005)));
        let quick = step(&mut fast, 0.64, Some(&shifted(0.04)));
        let small_gain = small["delta"][0].as_f64().unwrap() / 0.005;
        let quick_gain = quick["delta"][0].as_f64().unwrap() / 0.04;
        assert!(quick_gain > small_gain);
        step(&mut fast, 0.7, None);
        assert_eq!(
            step(&mut fast, 0.8, Some(&shifted(0.08)))["mode"],
            "acquiring"
        );
        for t in [0.9, 1., 1.11] {
            step(&mut fast, t, Some(&shifted(0.08)));
        }
        assert_eq!(
            step(&mut fast, 1.2, Some(&shifted(0.08)))["delta"],
            json!([0., 0.])
        );
    }
    #[test]
    fn smoothing_reduces_resting_jitter_without_accumulating_a_jump() {
        let p = pose(1.4, 0.3);
        let mut raw = Motion::default();
        let mut smooth = Motion::default();
        prime(&mut raw, &p);
        prime(&mut smooth, &p);
        let mut travel = [0., 0.];
        for i in 1..=40 {
            let mut noisy = p.clone();
            for point in &mut noisy {
                point.x += if i % 2 == 0 { 0.0025 } else { -0.0025 };
            }
            for (index, motion) in [&mut raw, &mut smooth].into_iter().enumerate() {
                let result = motion.update(
                    0.54 + i as f64 * 0.08,
                    Some(&noisy),
                    1.,
                    false,
                    MotionSettings {
                        smoothing: index as f64,
                        ..Default::default()
                    },
                );
                travel[index] += result["delta"][0].as_f64().unwrap_or(0.).abs();
            }
        }
        assert!(travel[1] < travel[0] * 0.7, "{travel:?}");
    }
    #[test]
    fn left_clutch_switches_right_index_into_scroll() {
        let mut m = Motion::default();
        let p = pose(1.4, 0.3);
        prime(&mut m, &p);
        assert_eq!(
            m.update(
                0.65,
                Some(&p),
                1.,
                true,
                MotionSettings {
                    clutch_mode: true,
                    ..Default::default()
                }
            )["mode"],
            "scroll"
        );
        assert_eq!(
            m.update(
                0.75,
                Some(&p),
                1.,
                false,
                MotionSettings {
                    clutch_mode: true,
                    ..Default::default()
                }
            )["mode"],
            "pointer"
        );
    }
}

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
}
fn sensitivity() -> f64 {
    0.32
}
#[derive(Default)]
pub struct Engine {
    right: Tracker,
    left: Tracker,
    motion: Motion,
}
impl Engine {
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
            hand: left,
            face: None,
            threshold: input.frame.threshold,
        });
        input.frame.hand = right.clone();
        let right_result = self.right.update(input.frame);
        let control = self.motion.update(
            time,
            right.as_deref(),
            aspect,
            input.scroll_clutch,
            left_result.holding && left_result.state == "held",
            input.sensitivity,
        );
        let mut result = serde_json::to_value(&right_result).unwrap();
        result["left"] = serde_json::to_value(left_result).unwrap();
        result["right"] = serde_json::to_value(right_result).unwrap();
        result["control"] = control;
        result
    }
}
#[derive(Default)]
struct Motion {
    last: Option<f64>,
    center: Option<[f64; 2]>,
    stable_since: Option<f64>,
    pointer: Option<[f64; 2]>,
    extended_since: Option<f64>,
    baseline: f64,
    armed: bool,
    cooldown: f64,
    push_since: Option<f64>,
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
        clutch_mode: bool,
        left_held: bool,
        sensitivity: f64,
    ) -> Value {
        let lost = || json!({"mode":"lost","reason":"Pravá ruka chybí nebo nemá spolehlivé body","click":false,"scroll":0});
        let Some(p) = points.filter(|p| {
            p.len() == 21
                && [0, 5, 6, 8, 9, 10, 12, 13, 16, 17, 20]
                    .iter()
                    .all(|&i| valid(p[i]))
        }) else {
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
        let two_fingers = index > 0.95 && middle > 0.95 && ring < 0.85 && little < 0.85;
        let scroll_pose = if clutch_mode {
            left_held && index > 0.65
        } else {
            two_fingers
        };
        if scroll_pose {
            self.armed = false;
            self.push_since = None;
            self.extended_since = None;
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
        // Restrict click to a single pointing index. Shortening relative to its
        // recent extended pose is a monocular push proxy, not a depth estimate.
        let pointing = middle < 0.85 && ring < 0.85 && little < 0.85;
        let sensitivity = if sensitivity.is_finite() {
            sensitivity.clamp(0.2, 0.55)
        } else {
            0.32
        };
        let mut click = false;
        if pointing && index > 1.05 {
            self.push_since = None;
            let since = *self.extended_since.get_or_insert(now);
            if now - since >= 0.22 && now >= self.cooldown {
                self.armed = true;
                self.baseline = index;
            }
        } else {
            self.extended_since = None;
            if pointing && self.armed && index < self.baseline * (1. - sensitivity) && index > 0.3 {
                let since = *self.push_since.get_or_insert(now);
                if now - since >= 0.06 {
                    click = true;
                    self.cooldown = now + 0.7;
                    self.armed = false;
                    self.push_since = None;
                }
            } else {
                self.armed = false;
                self.push_since = None;
            }
        }
        if !pointing {
            self.armed = false;
            self.push_since = None;
        }
        let target = [
            ((1. - center[0] - 0.15) / 0.7).clamp(0., 1.),
            ((1. - center[1] - 0.15) / 0.7).clamp(0., 1.),
        ];
        let alpha = 1. - (-dt.clamp(0., 0.25) / 0.07).exp();
        let pointer = self.pointer.map_or(target, |previous| {
            [
                previous[0] + alpha * (target[0] - previous[0]),
                previous[1] + alpha * (target[1] - previous[1]),
            ]
        });
        self.pointer = Some(pointer);
        json!({"mode":"pointer","reason":if click {"KLIK · zkrácení ukazováku"} else if self.armed {"Klik připraven · zatlač ukazovák ke kameře"} else {"Pravá dlaň řídí kurzor · pro klik natáhni jen ukazovák"},"pointer":pointer,"click":click,"scroll":0,"extension":index,"armed":self.armed,"left_clutch":left_held})
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
        m.update(t, p, 1., false, false, 0.32)
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
    fn left_clutch_switches_right_index_into_scroll() {
        let mut m = Motion::default();
        let p = pose(1.4, 0.3);
        prime(&mut m, &p);
        assert_eq!(
            m.update(0.65, Some(&p), 1., true, true, 0.32)["mode"],
            "scroll"
        );
        assert_eq!(
            m.update(0.75, Some(&p), 1., true, false, 0.32)["mode"],
            "pointer"
        );
    }
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub confidence: f64,
}
#[derive(Debug, Default, Deserialize)]
pub struct Face {
    pub confidence: f64,
    pub left_eye: Vec<Point>,
    pub right_eye: Vec<Point>,
    pub left_pupil: Option<Point>,
    pub right_pupil: Option<Point>,
}
#[derive(Debug, Deserialize)]
pub struct Frame {
    pub time: f64,
    pub width: f64,
    pub height: f64,
    pub hand: Option<Vec<Point>>,
    pub face: Option<Face>,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
}
fn default_threshold() -> f64 {
    0.35
}
#[derive(Debug, Serialize)]
pub struct EyeSignal {
    pub x: f64,
    pub y: f64,
    pub quality: f64,
    pub left_openness: f64,
    pub right_openness: f64,
}
#[derive(Debug, Serialize)]
pub struct Output {
    pub state: &'static str,
    pub event: Option<&'static str>,
    pub reason: &'static str,
    pub hand_confidence: f64,
    pub missing_joints: Vec<&'static str>,
    pub pinch_index: Option<f64>,
    pub pinch_middle: Option<f64>,
    pub pinch_span: Option<f64>,
    pub center: Option<[f64; 2]>,
    pub grab_origin: Option<[f64; 2]>,
    pub drag_delta: Option<[f64; 2]>,
    pub eye_signal: Option<EyeSignal>,
    pub holding: bool,
}
#[derive(Default)]
pub struct Tracker {
    holding: bool,
    ready: bool,
    pending: Option<f64>,
    last_seen: Option<f64>,
    last_time: Option<f64>,
    last_center: Option<[f64; 2]>,
    origin: Option<[f64; 2]>,
}
struct Hand {
    confidence: f64,
    index: f64,
    middle: f64,
    span: f64,
    center: [f64; 2],
}
fn good(p: Point) -> bool {
    p.x.is_finite()
        && p.y.is_finite()
        && (0.0..=1.0).contains(&p.x)
        && (0.0..=1.0).contains(&p.y)
        && p.confidence >= 0.5
}
fn distance(a: Point, b: Point, aspect: f64) -> f64 {
    ((a.x - b.x) * aspect).hypot(a.y - b.y)
}
fn hand(points: &[Point], aspect: f64) -> Option<Hand> {
    if points.len() != 21 || ![0, 4, 5, 8, 12, 17].iter().all(|i| good(points[*i])) {
        return None;
    }
    let scale = distance(points[5], points[17], aspect);
    if scale < 0.025 {
        return None;
    }
    let index = distance(points[4], points[8], aspect) / scale;
    let middle = distance(points[4], points[12], aspect) / scale;
    let span = index
        .max(middle)
        .max(distance(points[8], points[12], aspect) / scale);
    Some(Hand {
        confidence: [0, 4, 5, 8, 12, 17]
            .iter()
            .map(|i| points[*i].confidence)
            .fold(1., f64::min),
        index,
        middle,
        span,
        center: [
            (points[4].x + points[8].x + points[12].x) / 3.,
            (points[4].y + points[8].y + points[12].y) / 3.,
        ],
    })
}
fn eye(points: &[Point], pupil: Point, width: f64, height: f64) -> Option<(f64, f64, f64)> {
    if points.len() < 4 || !good(pupil) || points.iter().any(|p| !good(*p)) {
        return None;
    }
    let pixel = |p: Point| [p.x * width, p.y * height];
    let mut a = pixel(points[0]);
    let mut b = a;
    let mut longest = 0.;
    for p in points {
        for q in points {
            let p = pixel(*p);
            let q = pixel(*q);
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            if d > longest {
                longest = d;
                a = p;
                b = q;
            }
        }
    }
    if longest < 10. {
        return None;
    }
    if a[0] > b[0] {
        std::mem::swap(&mut a, &mut b);
    }
    let axis = [(b[0] - a[0]) / longest, (b[1] - a[1]) / longest];
    let normal = [-axis[1], axis[0]];
    let project = |p: [f64; 2], v: [f64; 2]| (p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1];
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for p in points {
        let y = project(pixel(*p), normal);
        low = low.min(y);
        high = high.max(y);
    }
    let openness = (high - low) / longest;
    if !(0.14..=0.8).contains(&openness) {
        return None;
    }
    let x = project(pixel(pupil), axis) / longest;
    let y = (project(pixel(pupil), normal) - low) / (high - low);
    if !(0.05..=0.95).contains(&x) || !(0.05..=0.95).contains(&y) {
        return None;
    }
    Some((x, y, openness))
}
fn eyes(face: &Face, width: f64, height: f64) -> Option<EyeSignal> {
    if !face.confidence.is_finite() || face.confidence < 0.5 {
        return None;
    }
    let l = eye(&face.left_eye, face.left_pupil?, width, height)?;
    let r = eye(&face.right_eye, face.right_pupil?, width, height)?;
    if (l.0 - r.0).abs() > 0.25 || (l.1 - r.1).abs() > 0.4 {
        return None;
    }
    Some(EyeSignal {
        x: (l.0 + r.0) / 2.,
        y: (l.1 + r.1) / 2.,
        quality: face.confidence.min(1.),
        left_openness: l.2,
        right_openness: r.2,
    })
}
impl Tracker {
    pub fn update(&mut self, frame: Frame) -> Output {
        let now = frame.time;
        let required = [
            (0, "wrist"),
            (4, "thumb_tip"),
            (5, "index_base"),
            (8, "index_tip"),
            (12, "middle_tip"),
            (17, "little_base"),
        ];
        let raw = frame.hand.as_deref().filter(|p| p.len() == 21);
        let missing_joints = raw
            .map(|points| {
                required
                    .iter()
                    .filter(|(i, _)| !good(points[*i]))
                    .map(|(_, name)| *name)
                    .collect()
            })
            .unwrap_or_default();
        let raw_quality = raw.map_or(0., |points| {
            required
                .iter()
                .map(|(i, _)| {
                    if points[*i].confidence.is_finite() {
                        points[*i].confidence.clamp(0., 1.)
                    } else {
                        0.
                    }
                })
                .fold(1., f64::min)
        });

        let geometry = frame.width.is_finite()
            && frame.height.is_finite()
            && frame.width > 0.
            && frame.height > 0.;
        let hand = if geometry {
            frame
                .hand
                .as_deref()
                .and_then(|h| hand(h, frame.width / frame.height))
        } else {
            None
        };
        let eye_signal = if geometry {
            frame
                .face
                .as_ref()
                .and_then(|f| eyes(f, frame.width, frame.height))
        } else {
            None
        };
        let mut event = None;
        let reason;
        let time_valid = now.is_finite() && self.last_time.is_none_or(|last| now >= last);
        if !time_valid {
            event = self.holding.then_some("cancel");
            *self = Self::default();
            reason = "invalid_time";
        } else {
            self.last_time = Some(now);
            if self.last_seen.is_some_and(|last| now - last > 0.30) {
                event = self.holding.then_some("cancel");
                self.holding = false;
                self.ready = false;
                self.pending = None;
                self.origin = None;
                self.last_center = None;
            }
            if let Some(h) = &hand {
                let jumped = self.holding
                    && self.last_center.is_some_and(|last| {
                        ((h.center[0] - last[0]) * frame.width / frame.height)
                            .hypot(h.center[1] - last[1])
                            > 0.30
                    });
                self.last_seen = Some(now);
                self.last_center = Some(h.center);
                if jumped {
                    self.holding = false;
                    self.ready = false;
                    self.pending = None;
                    self.origin = None;
                    event = Some("cancel");
                    reason = "hand_jump";
                } else {
                    let close = if frame.threshold.is_finite() {
                        frame.threshold.clamp(0.15, 0.65)
                    } else {
                        default_threshold()
                    };
                    let open = close * 1.6;
                    let condition = if self.holding || !self.ready {
                        h.span >= open
                    } else {
                        h.span <= close
                    };
                    if condition {
                        let start = *self.pending.get_or_insert(now);
                        let duration = if self.holding || !self.ready {
                            0.10
                        } else {
                            0.15
                        };
                        if now - start + 1e-9 >= duration {
                            self.pending = None;
                            if self.holding {
                                self.holding = false;
                                self.origin = None;
                                self.ready = true;
                                event = Some("release");
                            } else if !self.ready {
                                self.ready = true;
                            } else {
                                self.holding = true;
                                self.origin = Some(h.center);
                                event = Some("grab");
                            }
                        }
                    } else {
                        self.pending = None;
                    }
                    reason = if self.holding {
                        if self.pending.is_some() {
                            "confirm_release"
                        } else {
                            "holding"
                        }
                    } else if !self.ready {
                        "show_open_hand"
                    } else if self.pending.is_some() {
                        "confirm_grab"
                    } else {
                        "ready"
                    };
                }
            } else {
                self.pending = None;
                self.ready = false;
                reason = if self.holding {
                    "tracking_grace"
                } else {
                    "show_hand"
                };
            }
        }
        let center = hand.as_ref().map(|h| h.center);
        let drag_delta = if self.holding {
            self.origin
                .zip(center)
                .map(|(o, c)| [c[0] - o[0], c[1] - o[1]])
        } else {
            None
        };
        Output {
            state: if hand.is_none() {
                "lost"
            } else if self.holding {
                "held"
            } else if !self.ready {
                "waiting_open"
            } else if self.pending.is_some() {
                "arming"
            } else {
                "ready"
            },
            event,
            reason,
            hand_confidence: hand.as_ref().map_or(raw_quality, |h| h.confidence),
            missing_joints,
            pinch_index: hand.as_ref().map(|h| h.index),
            pinch_middle: hand.as_ref().map(|h| h.middle),
            pinch_span: hand.as_ref().map(|h| h.span),
            center,
            grab_origin: self.origin,
            drag_delta,
            eye_signal,
            holding: self.holding,
        }
    }
}
#[cfg(test)]
#[path = "tracking_tests.rs"]
mod tests;

use serde::Serialize;

#[derive(Default)]
pub struct ClickDetector {
    neutral_since: Option<f64>,
    armed_at: Option<f64>,
    push_since: Option<f64>,
    baseline: f64,
    cooldown: f64,
    release_required: bool,
}
#[derive(Serialize)]
pub struct Feedback {
    pub click: bool,
    pub armed: bool,
    pub progress: f64,
    pub stage: &'static str,
    pub reason: &'static str,
}
impl ClickDetector {
    pub fn update(
        &mut self,
        now: f64,
        index: f64,
        pointing: bool,
        pinch: Option<f64>,
        sensitivity: f64,
        pinch_method: bool,
    ) -> Feedback {
        let mut out = Feedback {
            click: false,
            armed: self.armed_at.is_some(),
            progress: 0.,
            stage: "pose",
            reason: if pinch_method {
                "Oddal palec a ukazovák, ostatní prsty nech volně"
            } else {
                "Natáhni ukazovák, prostředník, prsteník a malík ohni"
            },
        };
        let valid_pose = if pinch_method {
            pinch.is_some()
        } else {
            pointing
        };
        if !valid_pose {
            self.neutral_since = None;
            self.armed_at = None;
            self.push_since = None;
            out.armed = false;
            if pinch_method {
                out.reason = "Ukaž špičku palce i ukazováku";
            }
            return out;
        }
        if now < self.cooldown {
            out.stage = "cooldown";
            out.reason = "Klik potvrzen. Prsty vrať do výchozí polohy";
            out.armed = false;
            return out;
        }
        if let Some(armed_at) = self.armed_at {
            if now - armed_at > 2. {
                self.armed_at = None;
                self.neutral_since = None;
                self.push_since = None;
                out.armed = false;
                out.reason = "Zkus to znovu: vrať prsty do výchozí polohy";
                return out;
            }
            let progress = if pinch_method {
                ((0.55 - pinch.unwrap()) / 0.30).clamp(0., 1.)
            } else {
                ((self.baseline - index) / (self.baseline * sensitivity)).clamp(0., 1.)
            };
            let pressed = if pinch_method {
                pinch.unwrap() < 0.25
            } else {
                index <= self.baseline * (1. - sensitivity)
            };
            out.progress = progress;
            out.stage = "ready";
            out.reason = if pinch_method {
                "Připraveno. Spoj palec a ukazovák"
            } else {
                "Připraveno. Zatlač ukazovák ke kameře; dlaň drž na místě"
            };
            // Once armed, intermediate lengths must not disarm a gradual push.
            // Evaluate relative shortening before any absolute extension gate.
            if pressed {
                let since = *self.push_since.get_or_insert(now);
                out.stage = "pressing";
                out.reason = "Gesto vidím, ještě krátce podrž";
                if now - since >= 0.06 {
                    self.armed_at = None;
                    self.neutral_since = None;
                    self.push_since = None;
                    self.cooldown = now + 0.7;
                    self.release_required = true;
                    out.click = true;
                    out.armed = false;
                    out.progress = 1.;
                    out.stage = "clicked";
                    out.reason = "Klik rozpoznán";
                }
            } else {
                self.push_since = None;
                let neutral = if pinch_method {
                    pinch.unwrap() > 0.55
                } else {
                    index >= self.baseline * 0.94
                };
                if neutral {
                    self.armed_at = Some(now);
                }
            }
            return out;
        }
        let neutral = if pinch_method {
            pinch.unwrap() > 0.55
        } else {
            index > 0.85 && (!self.release_required || index >= self.baseline * 0.9)
        };
        if neutral {
            self.release_required = false;
            let since = *self.neutral_since.get_or_insert(now);
            out.stage = "arming";
            out.reason = "Správná poloha. Chvilku ji podrž";
            out.progress = ((now - since) / 0.22).clamp(0., 1.);
            if now - since >= 0.22 {
                self.armed_at = Some(now);
                self.baseline = index;
                out.armed = true;
                out.progress = 0.;
                out.stage = "ready";
                out.reason = if pinch_method {
                    "Připraveno. Spoj palec a ukazovák"
                } else {
                    "Připraveno. Zatlač ukazovák ke kameře"
                };
            }
        } else {
            self.neutral_since = None;
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn step(d: &mut ClickDetector, t: f64, index: f64) -> Feedback {
        d.update(t, index, true, None, 0.32, false)
    }
    #[test]
    fn gradual_push_keeps_armed_through_intermediate_lengths() {
        let mut d = ClickDetector::default();
        step(&mut d, 0., 1.4);
        assert!(step(&mut d, 0.23, 1.4).armed);
        for (t, index) in [(0.3, 1.2), (0.4, 1.05), (0.5, 0.98)] {
            assert!(step(&mut d, t, index).armed);
        }
        assert!(!step(&mut d, 0.6, 0.9).click);
        assert!(step(&mut d, 0.67, 0.9).click);
        assert!(!step(&mut d, 0.8, 0.9).click);
        assert!(!step(&mut d, 1.5, 0.9).click);
    }
    #[test]
    fn relative_push_can_click_while_finger_is_still_long() {
        let mut d = ClickDetector::default();
        step(&mut d, 0., 2.);
        step(&mut d, 0.23, 2.);
        step(&mut d, 0.3, 1.3);
        assert!(step(&mut d, 0.37, 1.3).click);
    }
    #[test]
    fn pose_loss_and_long_incomplete_push_require_rearming() {
        let mut d = ClickDetector::default();
        step(&mut d, 0., 1.4);
        step(&mut d, 0.23, 1.4);
        assert!(!d.update(0.3, 1.4, false, None, 0.32, false).armed);
        assert!(!step(&mut d, 0.4, 0.7).click);
        step(&mut d, 0.5, 1.4);
        step(&mut d, 0.73, 1.4);
        step(&mut d, 1., 1.1);
        assert!(!step(&mut d, 3., 0.7).armed);
    }
    #[test]
    fn pinch_requires_opening_and_confirmation_then_release_to_repeat() {
        let mut d = ClickDetector::default();
        let mut step = |t, ratio| d.update(t, 0.5, false, Some(ratio), 0.32, true);
        assert!(!step(0., 0.1).click);
        assert!(!step(1., 0.1).armed);
        step(1.1, 0.8);
        assert!(step(1.34, 0.8).armed);
        assert!(!step(1.4, 0.1).click);
        assert!(step(1.47, 0.1).click);
        assert!(!step(2.3, 0.1).click);
        assert!(!step(2.4, 0.1).armed);
        step(2.5, 0.8);
        assert!(step(2.74, 0.8).armed);
    }
}

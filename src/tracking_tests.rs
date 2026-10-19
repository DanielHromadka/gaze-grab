use super::*;
fn point(x: f64, y: f64) -> Point {
    Point {
        x,
        y,
        confidence: 0.95,
    }
}
fn open() -> Vec<Point> {
    let mut p = vec![Point::default(); 21];
    p[0] = point(0.45, 0.30);
    p[5] = point(0.35, 0.45);
    p[17] = point(0.55, 0.45);
    p[4] = point(0.25, 0.55);
    p[8] = point(0.38, 0.80);
    p[12] = point(0.49, 0.83);
    p
}
fn closed() -> Vec<Point> {
    let mut p = open();
    p[4] = point(0.45, 0.65);
    p[8] = point(0.46, 0.65);
    p[12] = point(0.45, 0.66);
    p
}
fn frame(time: f64, hand: Option<Vec<Point>>) -> Frame {
    Frame {
        time,
        width: 1000.,
        height: 1000.,
        hand,
        face: None,
        threshold: 0.35,
    }
}
fn held() -> Tracker {
    let mut tracker = Tracker::default();
    tracker.update(frame(0., Some(open())));
    assert_eq!(tracker.update(frame(0.11, Some(open()))).state, "ready");
    assert_eq!(tracker.update(frame(0.16, Some(closed()))).state, "arming");
    tracker.update(frame(0.24, Some(closed())));
    assert_eq!(
        tracker.update(frame(0.32, Some(closed()))).event,
        Some("grab")
    );
    tracker
}
#[test]
fn startup_closed_requires_open_hand_and_dwell() {
    let mut t = Tracker::default();
    for time in [0., 0.1, 0.2, 0.3] {
        let o = t.update(frame(time, Some(closed())));
        assert_eq!(o.state, "waiting_open");
        assert!(o.event.is_none());
    }
    t.update(frame(0.35, Some(open())));
    assert_eq!(t.update(frame(0.46, Some(open()))).state, "ready");
    t.update(frame(0.5, Some(closed())));
    assert!(t.update(frame(0.6, Some(closed()))).event.is_none());
    assert_eq!(t.update(frame(0.66, Some(closed()))).event, Some("grab"));
    assert!(t.update(frame(0.7, Some(closed()))).event.is_none());
}
#[test]
fn two_finger_pinch_is_not_three_finger_grab() {
    let mut t = Tracker::default();
    t.update(frame(0., Some(open())));
    t.update(frame(0.11, Some(open())));
    let mut p = closed();
    p[12] = point(0.49, 0.83);
    for time in [0.16, 0.25, 0.35, 0.45] {
        let o = t.update(frame(time, Some(p.clone())));
        assert_eq!(o.state, "ready");
        assert!(!o.holding);
    }
}
#[test]
fn hysteresis_and_release_confirmation_prevent_chatter() {
    let mut t = held();
    let mut jitter = closed();
    jitter[8] = point(0.53, 0.65);
    for time in [0.36, 0.40, 0.44] {
        let o = t.update(frame(time, Some(jitter.clone())));
        assert!(o.holding);
        assert!(o.event.is_none());
    }
    assert!(t.update(frame(0.50, Some(open()))).holding);
    assert!(t.update(frame(0.56, Some(open()))).holding);
    assert_eq!(t.update(frame(0.61, Some(open()))).event, Some("release"));
    assert!(t.update(frame(0.66, Some(open()))).event.is_none());
}
#[test]
fn lost_tracking_cancels_instead_of_releasing_and_cannot_regrab_closed() {
    let mut t = held();
    let grace = t.update(frame(0.37, None));
    assert!(grace.holding && grace.event.is_none());
    let lost = t.update(frame(0.67, None));
    assert_eq!(lost.event, Some("cancel"));
    assert!(!lost.holding);
    assert!(t.update(frame(0.90, None)).event.is_none());
    assert_eq!(t.update(frame(0.91, Some(closed()))).state, "waiting_open");
    t.update(frame(1., Some(open())));
    assert_eq!(t.update(frame(1.11, Some(open()))).state, "ready");
    let mut t = held();
    let mut missing = closed();
    missing[12].confidence = 0.1;
    let lost = t.update(frame(0.68, Some(missing)));
    assert_eq!(lost.event, Some("cancel"));
    assert!(lost.missing_joints.contains(&"middle_tip"));
    assert!((lost.hand_confidence - 0.1).abs() < 1e-9);
}
#[test]
fn large_jumps_and_backward_time_cancel_held_state() {
    let mut t = held();
    let mut p = closed();
    for p in &mut p {
        p.x += 0.35;
    }
    assert_eq!(t.update(frame(0.36, Some(p))).event, Some("cancel"));
    let mut t = held();
    let o = t.update(frame(0.1, Some(closed())));
    assert_eq!(o.event, Some("cancel"));
    assert!(!o.holding);
}
#[test]
fn pinch_ratios_respect_camera_aspect_and_hand_scale() {
    let p = closed();
    let h = hand(&p, 1.).unwrap();
    let mut wide = p.clone();
    for p in &mut wide {
        p.x /= 2.;
    }
    assert!((hand(&wide, 2.).unwrap().span - h.span).abs() < 1e-9);
    let mut small = p;
    for p in &mut small {
        p.x *= 0.5;
        p.y *= 0.5;
    }
    assert!((hand(&small, 1.).unwrap().span - h.span).abs() < 1e-9);
}
fn face() -> Face {
    let region = |x| {
        vec![
            point(x - 0.03, 0.55),
            point(x, 0.559),
            point(x + 0.03, 0.55),
            point(x, 0.541),
        ]
    };
    Face {
        confidence: 0.95,
        left_eye: region(0.3),
        right_eye: region(0.7),
        left_pupil: Some(point(0.306, 0.55)),
        right_pupil: Some(point(0.706, 0.55)),
    }
}
#[test]
fn eye_signal_is_roll_invariant_and_rejects_blinks_and_disagreement() {
    let mut f = face();
    let original = eyes(&f, 1000., 1000.).unwrap();
    assert!((original.x - 0.6).abs() < 1e-9);
    let rotate = |p: &mut Point| {
        let x = p.x - 0.5;
        let y = p.y - 0.5;
        p.x = 0.5 + x * 0.4_f64.cos() - y * 0.4_f64.sin();
        p.y = 0.5 + x * 0.4_f64.sin() + y * 0.4_f64.cos();
    };
    for p in f.left_eye.iter_mut().chain(f.right_eye.iter_mut()) {
        rotate(p);
    }
    rotate(f.left_pupil.as_mut().unwrap());
    rotate(f.right_pupil.as_mut().unwrap());
    let rotated = eyes(&f, 1000., 1000.).unwrap();
    assert!((rotated.x - original.x).abs() < 1e-9);
    assert!((rotated.y - original.y).abs() < 1e-9);
    let mut blink = face();
    for p in &mut blink.left_eye {
        p.y = 0.55 + (p.y - 0.55) * 0.1;
    }
    assert!(eyes(&blink, 1000., 1000.).is_none());
    let mut mismatch = face();
    mismatch.right_pupil = Some(point(0.68, 0.55));
    assert!(eyes(&mismatch, 1000., 1000.).is_none());
    let mut missing = face();
    missing.left_pupil = None;
    assert!(eyes(&missing, 1000., 1000.).is_none());
}

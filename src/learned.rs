//! Local few-shot template learning over normalized Vision landmarks. No images
//! are retained. Static poses use averaged shape; motions use temporal alignment.
use crate::tracking::Point;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::VecDeque, path::PathBuf};

const STEPS: usize = 16;
#[derive(Clone, Serialize, Deserialize)]
pub struct Gesture {
    id: u64,
    name: String,
    hands: String,
    kind: String,
    action: String,
    samples: Vec<Vec<Vec<f64>>>,
    enabled: bool,
}
#[derive(Clone)]
struct Observation {
    time: f64,
    shape: Vec<f64>,
    wrists: Vec<[f64; 2]>,
    scales: Vec<f64>,
}
#[derive(Default)]
struct Draft {
    name: String,
    hands: String,
    kind: String,
    action: String,
    samples: Vec<Vec<Vec<f64>>>,
    capture: Option<f64>,
    frames: Vec<Observation>,
}
pub struct Library {
    path: Option<PathBuf>,
    storage_error: Option<String>,
    gestures: Vec<Gesture>,
    draft: Option<Draft>,
    history: VecDeque<Observation>,
    candidate: Option<(u64, f64)>,
    latched: Option<u64>,
    absent_since: Option<f64>,
    last: Option<f64>,
    message: String,
}
impl Default for Library {
    fn default() -> Self {
        let path = if cfg!(test) {
            None
        } else {
            std::env::var_os("HOME").map(|home| {
                PathBuf::from(home).join("Library/Application Support/Gaze Grab/gestures.json")
            })
        };
        let mut library = Self::empty(path);
        if let Some(path) = &library.path {
            match std::fs::read(path) {
                Ok(bytes) => match serde_json::from_slice::<Vec<Gesture>>(&bytes) {
                    Ok(gestures) if gestures.len() <= 32 && gestures.iter().all(valid_gesture) => {
                        library.gestures = gestures
                    }
                    _ => library.message = "Soubor gest nelze načíst. Nebyl přepsán.".into(),
                },
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => library.message = format!("Gesta nelze načíst: {error}"),
            }
        }
        if !library.message.is_empty() {
            library.storage_error = Some(library.message.clone());
        }
        library
    }
}
fn valid_action(action: &str) -> bool {
    matches!(
        action,
        "click" | "double_click" | "right_click" | "scroll_up" | "scroll_down" | "pause" | "none"
    )
}
fn valid_hands(hands: &str) -> bool {
    matches!(hands, "left" | "right" | "both")
}
fn valid_gesture(g: &Gesture) -> bool {
    !g.name.trim().is_empty()
        && g.name.chars().count() <= 60
        && valid_hands(&g.hands)
        && valid_action(&g.action)
        && matches!(g.kind.as_str(), "pose" | "motion")
        && (3..=8).contains(&g.samples.len())
        && g.samples.iter().all(|sample| {
            sample.len() == STEPS
                && sample.iter().all(|row| {
                    !row.is_empty()
                        && row.len() <= 100
                        && row.iter().all(|v| v.is_finite() && v.abs() < 100.)
                })
        })
}
fn good(p: Point) -> bool {
    p.x.is_finite()
        && p.y.is_finite()
        && p.confidence.is_finite()
        && p.confidence >= 0.3
        && (0.0..=1.).contains(&p.x)
        && (0.0..=1.).contains(&p.y)
}
fn observation(
    now: f64,
    right: Option<&[Point]>,
    left: Option<&[Point]>,
    hands: &str,
    aspect: f64,
) -> Option<Observation> {
    if !now.is_finite() || !aspect.is_finite() || aspect <= 0. {
        return None;
    }
    let mut shape = Vec::new();
    let mut wrists = Vec::new();
    let mut scales = Vec::new();
    for points in match hands {
        "left" => vec![left?],
        "right" => vec![right?],
        "both" => vec![left?, right?],
        _ => return None,
    } {
        const JOINTS: [usize; 14] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 16, 17, 20];
        if points.len() != 21 || !JOINTS.iter().all(|&i| good(points[i])) {
            return None;
        }
        let scale = ((points[5].x - points[17].x) * aspect).hypot(points[5].y - points[17].y);
        if scale < 0.025 {
            return None;
        }
        let wrist = [points[0].x * aspect, points[0].y];
        for i in JOINTS {
            shape.push((points[i].x * aspect - wrist[0]) / scale * 0.35);
            shape.push((points[i].y - wrist[1]) / scale * 0.35);
        }
        wrists.push(wrist);
        scales.push(scale);
    }
    if wrists.len() == 2 {
        let scale = (scales[0] + scales[1]) / 2.;
        shape.push((wrists[1][0] - wrists[0][0]) / scale * 0.12);
        shape.push((wrists[1][1] - wrists[0][1]) / scale * 0.12);
    }
    Some(Observation {
        time: now,
        shape,
        wrists,
        scales,
    })
}
fn sequence(frames: &[Observation], kind: &str) -> Option<Vec<Vec<f64>>> {
    let first = frames.first()?;
    let last = frames.last()?;
    if frames.len() < if kind == "pose" { 5 } else { 10 }
        || last.time - first.time < if kind == "pose" { 0.35 } else { 0.7 }
    {
        return None;
    }
    let rows: Vec<Vec<f64>> = frames
        .iter()
        .map(|f| {
            let mut row = f.shape.clone();
            if kind == "motion" {
                for (i, wrist) in f.wrists.iter().enumerate() {
                    row.push((wrist[0] - first.wrists[i][0]) / first.scales[i] * 2.0);
                    row.push((wrist[1] - first.wrists[i][1]) / first.scales[i] * 2.0);
                }
            }
            row
        })
        .collect();
    if kind == "pose" {
        let mut mean = vec![0.; rows[0].len()];
        for row in &rows {
            for (m, v) in mean.iter_mut().zip(row) {
                *m += v / rows.len() as f64;
            }
        }
        return Some(vec![mean; STEPS]);
    }
    Some(
        (0..STEPS)
            .map(|i| {
                let t = first.time + (last.time - first.time) * i as f64 / (STEPS - 1) as f64;
                let upper = frames.partition_point(|f| f.time < t).min(frames.len() - 1);
                let lower = upper.saturating_sub(1);
                let factor = if upper == lower {
                    0.
                } else {
                    (t - frames[lower].time) / (frames[upper].time - frames[lower].time).max(0.001)
                };
                rows[lower]
                    .iter()
                    .zip(&rows[upper])
                    .map(|(a, b)| a + (b - a) * factor)
                    .collect()
            })
            .collect(),
    )
}
fn rms(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return f64::INFINITY;
    }
    (a.iter().zip(b).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / a.len() as f64).sqrt()
}
fn distance(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }
    let mut previous = vec![f64::INFINITY; b.len() + 1];
    previous[0] = 0.;
    for row in a {
        let mut current = vec![f64::INFINITY; b.len() + 1];
        for (j, other) in b.iter().enumerate() {
            current[j + 1] = rms(row, other) + previous[j].min(previous[j + 1]).min(current[j]);
        }
        previous = current;
    }
    previous[b.len()] / a.len().max(b.len()) as f64
}
impl Library {
    pub fn rearm(&mut self) {
        self.history.clear();
        self.candidate = None;
        self.latched = None;
        self.absent_since = None;
    }
    fn empty(path: Option<PathBuf>) -> Self {
        Self {
            path,
            storage_error: None,
            gestures: Vec::new(),
            draft: None,
            history: VecDeque::new(),
            candidate: None,
            latched: None,
            absent_since: None,
            last: None,
            message: String::new(),
        }
    }
    fn persist(&self) -> Result<(), String> {
        if let Some(error) = &self.storage_error {
            return Err(error.clone());
        }
        let Some(path) = &self.path else {
            return Err("Chybí složka pro ukládání gest".into());
        };
        let data = serde_json::to_vec_pretty(&self.gestures).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let temp = path.with_extension("json.tmp");
        std::fs::write(&temp, data)
            .and_then(|()| std::fs::rename(&temp, path))
            .map_err(|e| format!("Gesta nelze uložit: {e}"))
    }
    pub fn command(&mut self, command: Value) -> Value {
        let result = self.perform(command);
        if let Err(error) = &result {
            self.message = error.clone();
        }
        json!({"ok":result.is_ok(),"message":self.message,"gestures":self.gestures.iter().map(|g|json!({"id":g.id,"name":g.name,"hands":g.hands,"kind":g.kind,"action":g.action,"enabled":g.enabled})).collect::<Vec<_>>()})
    }
    fn perform(&mut self, c: Value) -> Result<(), String> {
        match c["command"].as_str().unwrap_or("") {
            "list" => {}
            "begin" => {
                let name = c["name"].as_str().unwrap_or("").trim().to_owned();
                let hands = c["hands"].as_str().unwrap_or("right").to_owned();
                let kind = c["kind"].as_str().unwrap_or("pose").to_owned();
                let action = c["action"].as_str().unwrap_or("none").to_owned();
                if name.is_empty() || name.chars().count() > 60 {
                    return Err("Zadej název do 60 znaků".into());
                }
                if !valid_hands(&hands)
                    || !valid_action(&action)
                    || !matches!(kind.as_str(), "pose" | "motion")
                {
                    return Err("Neplatné nastavení gesta".into());
                }
                self.draft = Some(Draft {
                    name,
                    hands,
                    kind,
                    action,
                    ..Default::default()
                });
                self.history.clear();
                self.message = "Připrav gesto a nahraj tři ukázky.".into();
            }
            "record" => {
                let draft = self.draft.as_mut().ok_or("Nejdřív připrav nové gesto")?;
                if draft.samples.len() >= 3 {
                    return Err("Tři ukázky jsou hotové. Ulož gesto nebo začni znovu.".into());
                }
                if draft.capture.is_some() {
                    return Err("Nahrávání už běží".into());
                }
                draft.capture = Some(self.last.unwrap_or(0.) + 2.);
                draft.frames.clear();
                self.message = "Za dvě sekundy začíná dvousekundová ukázka.".into();
            }
            "cancel" => {
                self.draft = None;
                self.history.clear();
                self.message = "Učení ukončeno.".into();
            }
            "save" => {
                let draft = self.draft.as_ref().ok_or("Chybí rozpracované gesto")?;
                if draft.samples.len() != 3 {
                    return Err("Nahraj nejdřív tři platné ukázky".into());
                }
                if self.gestures.len() >= 32 {
                    return Err("Lze uložit nejvýše 32 gest".into());
                }
                if self.gestures.iter().any(|g| g.name == draft.name) {
                    return Err(
                        "Tento název už existuje. Pro přeučení původní gesto nejprve smaž.".into(),
                    );
                }
                let g = Gesture {
                    id: self.gestures.iter().map(|g| g.id).max().unwrap_or(0) + 1,
                    name: draft.name.clone(),
                    hands: draft.hands.clone(),
                    kind: draft.kind.clone(),
                    action: draft.action.clone(),
                    samples: draft.samples.clone(),
                    enabled: true,
                };
                let variation = distance(&g.samples[0], &g.samples[1])
                    .max(distance(&g.samples[0], &g.samples[2]));
                if variation > 0.2 {
                    return Err(
                        "Ukázky se moc liší. Začni znovu a třikrát zopakuj stejné gesto.".into(),
                    );
                }
                if self.gestures.iter().any(|other| {
                    other.hands == g.hands
                        && other.kind == g.kind
                        && other
                            .samples
                            .iter()
                            .any(|s| g.samples.iter().any(|sample| distance(s, sample) < 0.045))
                }) {
                    return Err(
                        "Gesto je příliš podobné již uloženému. Použij odlišnější tvar nebo pohyb."
                            .into(),
                    );
                }
                self.gestures.push(g);
                if let Err(e) = self.persist() {
                    self.gestures.pop();
                    return Err(e);
                }
                self.draft = None;
                self.history.clear();
                self.latched = None;
                self.message = "Gesto uloženo. Teď ho můžeš vyzkoušet.".into();
            }
            "delete" | "assign" => {
                let id = c["id"].as_u64().ok_or("Vyber gesto")?;
                let index = self
                    .gestures
                    .iter()
                    .position(|g| g.id == id)
                    .ok_or("Gesto nenalezeno")?;
                let previous = self.gestures.clone();
                if c["command"] == "delete" {
                    self.gestures.remove(index);
                } else {
                    let action = c["action"].as_str().ok_or("Vyber akci")?;
                    if !valid_action(action) {
                        return Err("Neplatná akce".into());
                    }
                    self.gestures[index].action = action.to_owned();
                    self.gestures[index].enabled = c["enabled"].as_bool().unwrap_or(true);
                }
                if let Err(e) = self.persist() {
                    self.gestures = previous;
                    return Err(e);
                }
                self.history.clear();
                self.candidate = None;
                self.latched = None;
                self.message = "Změny uloženy.".into();
            }
            _ => return Err("Neznámý příkaz".into()),
        }
        Ok(())
    }
    pub fn update(
        &mut self,
        now: f64,
        right: Option<&[Point]>,
        left: Option<&[Point]>,
        aspect: f64,
    ) -> Value {
        let discontinuity = !now.is_finite()
            || self
                .last
                .is_some_and(|last| now <= last || now - last > 0.4);
        if discontinuity {
            self.history.clear();
            self.candidate = None;
            self.latched = None;
            self.absent_since = None;
            if let Some(d) = &mut self.draft {
                if d.capture.take().is_some() {
                    d.frames.clear();
                    self.message = "Nahrávání přerušeno. Zkus ukázku znovu.".into();
                }
            }
        }
        self.last = Some(now);
        if let Some(d) = &mut self.draft {
            let visible = observation(now, right, left, &d.hands, aspect);
            let mut progress = 0.;
            let mut recording = false;
            if let Some(start) = d.capture {
                if now < start {
                    self.message = format!("Připrav se: {:.0}…", (start - now).ceil());
                } else if now - start <= 2. {
                    recording = true;
                    progress = (now - start) / 2.;
                    if let Some(frame) = visible {
                        d.frames.push(frame);
                        self.message = "Nahrávám. Zopakuj gesto, pózu drž po celou dobu.".into();
                    } else {
                        self.message = "Ruce nejsou čitelné. Ukaž všechny prsty v záběru.".into();
                    }
                } else {
                    let sample = sequence(&d.frames, &d.kind);
                    if d.frames.len() >= 15
                        && !d.frames.windows(2).any(|w| w[1].time - w[0].time > 0.3)
                        && d.frames
                            .last()
                            .zip(d.frames.first())
                            .is_some_and(|(last, first)| last.time - first.time >= 1.4)
                    {
                        if let Some(sample) = sample {
                            d.samples.push(sample);
                            self.message = format!(
                                "Ukázka {} ze 3 přijata. Vrať ruku do výchozí polohy.",
                                d.samples.len()
                            );
                        }
                    } else {
                        self.message = "Málo platných snímků. Zkus stejnou ukázku znovu.".into();
                    }
                    d.capture = None;
                    d.frames.clear();
                }
            }
            return json!({"training":true,"recording":recording,"busy":d.capture.is_some(),"samples":d.samples.len(),"progress":progress,"message":self.message});
        }
        // Store observations independently for each scope so one missing hand
        // cannot suppress recognition of a gesture taught with the other hand.
        // The history includes all scopes, identified by descriptor dimensions.
        for hands in ["left", "right", "both"] {
            if let Some(mut o) = observation(now, right, left, hands, aspect) {
                // Tag the scope explicitly (left and right have equal dimensions).
                o.shape.insert(
                    0,
                    match hands {
                        "left" => -1.,
                        "right" => 1.,
                        _ => 0.,
                    },
                );
                self.history.push_back(o);
            }
        }
        while self.history.front().is_some_and(|o| now - o.time > 2.2) {
            self.history.pop_front();
        }
        let mut matches = Vec::new();
        for g in self.gestures.iter().filter(|g| g.enabled) {
            let tag = match g.hands.as_str() {
                "left" => -1.,
                "right" => 1.,
                _ => 0.,
            };
            let current = observation(now, right, left, &g.hands, aspect);
            if current.is_none() {
                continue;
            }
            let mut frames: Vec<_> = self
                .history
                .iter()
                .filter(|o| {
                    o.shape[0] == tag && now - o.time <= if g.kind == "pose" { 0.85 } else { 2. }
                })
                .cloned()
                .collect();
            for o in &mut frames {
                o.shape.remove(0);
            }
            if frames.windows(2).any(|w| w[1].time - w[0].time > 0.3) {
                continue;
            }
            let Some(seq) = sequence(&frames, &g.kind) else {
                continue;
            };
            let score = g
                .samples
                .iter()
                .map(|s| distance(s, &seq))
                .fold(f64::INFINITY, f64::min);
            let variation =
                distance(&g.samples[0], &g.samples[1]).max(distance(&g.samples[0], &g.samples[2]));
            let threshold = (variation * 1.8 + 0.07).clamp(0.09, 0.2);
            if score < threshold {
                matches.push((g.id, score, g.name.clone(), g.action.clone(), threshold));
            }
        }
        matches.sort_by(|a, b| a.1.total_cmp(&b.1));
        let best = matches
            .first()
            .filter(|best| matches.get(1).is_none_or(|second| second.1 - best.1 > 0.02));
        let mut event = None;
        if let Some((id, score, name, action, threshold)) = best {
            self.absent_since = None;
            if self.candidate.is_none_or(|(candidate, _)| candidate != *id) {
                self.candidate = Some((*id, now));
            }
            if now - self.candidate.unwrap().1 >= 0.25 && self.latched.is_none() {
                self.latched = Some(*id);
                event = Some(json!({"id":id,"name":name,"action":action}));
            }
            return json!({"training":false,"match":name,"action":action,"score":(1.-score/threshold).clamp(0.,1.),"event":event,"message":format!("Rozpoznáno: {name}"),"count":self.gestures.len()});
        }
        self.candidate = None;
        let absent = *self.absent_since.get_or_insert(now);
        if now - absent >= 0.4 {
            self.latched = None;
        }
        json!({"training":false,"event":null,"message":if matches.len()>1 {"Nejednoznačné gesto. Zkus odlišnější polohu."}else{"Čekám na naučené gesto"},"count":self.gestures.len()})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hand(x: f64, scale: f64, bent: bool) -> Vec<Point> {
        (0..21)
            .map(|i| {
                let finger = if i == 0 { 0. } else { ((i - 1) / 4) as f64 };
                let joint = if i == 0 { 0. } else { ((i - 1) % 4 + 1) as f64 };
                Point {
                    x: x + (finger - 2.) * scale / 4.,
                    y: 0.3 + joint * scale * if bent { 0.1 } else { 0.4 },
                    confidence: 0.95,
                }
            })
            .collect()
    }
    fn library() -> Library {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        Library::empty(Some(std::env::temp_dir().join(format!(
            "gaze-gestures-{}-{n}/gestures.json",
            std::process::id()
        ))))
    }
    fn teach(l: &mut Library, hands: &str, kind: &str) {
        assert_eq!(l.command(json!({"command":"begin","name":"Moje gesto","hands":hands,"kind":kind,"action":"click"}))["ok"],true);
        for _ in 0..3 {
            let start = l.last.unwrap_or(0.);
            l.command(json!({"command":"record"}));
            for i in 1..=55 {
                let t = i as f64 * 0.08;
                let x = 0.4
                    + if kind == "motion" {
                        ((t - 2.) / 2.).clamp(0., 1.) * 0.12
                    } else {
                        0.
                    };
                let r = hand(x, 0.2, false);
                let left = hand(x + 0.3, 0.2, true);
                l.update(start + t, Some(&r), Some(&left), 1.);
            }
        }
        assert_eq!(l.draft.as_ref().unwrap().samples.len(), 3);
        let saved = l.command(json!({"command":"save"}));
        assert_eq!(saved["ok"], true, "{saved}");
    }
    #[test]
    fn learns_pose_persists_assignment_and_fires_once_until_release() {
        let mut l = library();
        teach(&mut l, "right", "pose");
        let p = hand(0.65, 0.15, false);
        let mut events = 0;
        let start = l.last.unwrap();
        for i in 1..=40 {
            events += usize::from(
                l.update(start + i as f64 * 0.08, Some(&p), None, 1.)["event"].is_object(),
            );
        }
        assert_eq!(events, 1);
        let id = l.gestures[0].id;
        assert_eq!(
            l.command(json!({"command":"assign","id":id,"action":"pause","enabled":false}))["ok"],
            true
        );
        let reloaded: Vec<Gesture> =
            serde_json::from_slice(&std::fs::read(l.path.as_ref().unwrap()).unwrap()).unwrap();
        assert_eq!(reloaded[0].action, "pause");
        assert!(!reloaded[0].enabled);
        assert_eq!(l.command(json!({"command":"delete","id":id}))["ok"], true);
        assert!(l.gestures.is_empty());
        std::fs::remove_dir_all(l.path.unwrap().parent().unwrap()).unwrap();
    }
    #[test]
    fn two_hand_gesture_requires_both_correct_hands() {
        let mut l = library();
        teach(&mut l, "both", "pose");
        let start = l.last.unwrap();
        for i in 1..=20 {
            assert!(l.update(
                start + i as f64 * 0.08,
                Some(&hand(0.4, 0.2, false)),
                None,
                1.
            )["event"]
                .is_null());
        }
        let start = l.last.unwrap();
        let mut seen = false;
        for i in 1..=30 {
            seen |= l.update(
                start + i as f64 * 0.08,
                Some(&hand(0.4, 0.2, false)),
                Some(&hand(0.7, 0.2, true)),
                1.,
            )["event"]
                .is_object();
        }
        assert!(seen);
        std::fs::remove_dir_all(l.path.unwrap().parent().unwrap()).unwrap();
    }
    #[test]
    fn motion_alignment_distinguishes_reverse_path_and_scale() {
        let frames = |reverse: bool, scale: f64| {
            (0..26)
                .map(|i| {
                    let x = 0.4 + (if reverse { 25 - i } else { i }) as f64 / 25. * scale * 0.7;
                    observation(
                        i as f64 * 0.08,
                        Some(&hand(x, scale, false)),
                        None,
                        "right",
                        1.,
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>()
        };
        let a = sequence(&frames(false, 0.2), "motion").unwrap();
        let b = sequence(&frames(false, 0.12), "motion").unwrap();
        let c = sequence(&frames(true, 0.2), "motion").unwrap();
        assert!(distance(&a, &b) < 0.01);
        assert!(distance(&a, &c) > 0.09);
    }
    #[test]
    fn missing_landmarks_and_interrupted_recording_cannot_create_template() {
        let mut l = library();
        l.command(json!({"command":"begin","name":"Test","hands":"right","kind":"pose"}));
        l.update(1., None, None, 1.);
        l.command(json!({"command":"record"}));
        l.update(5., Some(&hand(0.4, 0.2, false)), None, 1.);
        assert!(l.draft.as_ref().unwrap().capture.is_none());
        assert!(l.draft.as_ref().unwrap().samples.is_empty());
        assert_eq!(l.command(json!({"command":"save"}))["ok"], false);
        let mut bad = hand(0.4, 0.2, false);
        bad[8].confidence = 0.1;
        assert!(observation(1., Some(&bad), None, "right", 1.).is_none());
    }
    #[test]
    fn recorded_motion_is_recognized_live_but_reverse_is_rejected() {
        let mut l = library();
        teach(&mut l, "right", "motion");
        let start = l.last.unwrap();
        let mut events = 0;
        for i in 1..=60 {
            let t = i as f64 * 0.08;
            let x = 0.4 + ((t - 2.) / 2.).clamp(0., 1.) * 0.12;
            events += usize::from(
                l.update(start + t, Some(&hand(x, 0.2, false)), None, 1.)["event"].is_object(),
            );
        }
        assert_eq!(events, 1);
        l.rearm();
        let start = l.last.unwrap_or(0.) + 1.;
        let mut events = 0;
        for i in 1..=60 {
            let t = i as f64 * 0.08;
            let x = 0.52 - ((t - 2.) / 2.).clamp(0., 1.) * 0.12;
            events += usize::from(
                l.update(start + t, Some(&hand(x, 0.2, false)), None, 1.)["event"].is_object(),
            );
        }
        assert_eq!(events, 0);
        std::fs::remove_dir_all(l.path.unwrap().parent().unwrap()).unwrap();
    }
    #[test]
    fn ambiguous_templates_do_not_emit_an_action() {
        let mut l = library();
        teach(&mut l, "right", "pose");
        let mut duplicate = l.gestures[0].clone();
        duplicate.id = 2;
        duplicate.name = "Podobné".into();
        l.gestures.push(duplicate);
        let start = l.last.unwrap();
        let p = hand(0.4, 0.2, false);
        for i in 1..=30 {
            let result = l.update(start + i as f64 * 0.08, Some(&p), None, 1.);
            assert!(result["event"].is_null());
        }
        std::fs::remove_dir_all(l.path.unwrap().parent().unwrap()).unwrap();
    }
    #[test]
    fn malformed_and_unavailable_storage_fail_without_mutating_models() {
        let mut l = Library::empty(None);
        let gesture = Gesture {
            id: 1,
            name: "Test".into(),
            hands: "right".into(),
            kind: "pose".into(),
            action: "click".into(),
            samples: vec![vec![vec![0.]; STEPS]; 3],
            enabled: true,
        };
        l.gestures.push(gesture);
        assert_eq!(
            l.command(json!({"command":"assign","id":1,"action":"scroll_up"}))["ok"],
            false
        );
        assert_eq!(l.gestures[0].action, "click");
        assert_eq!(
            l.command(json!({"command":"assign","id":1,"action":"run_shell"}))["ok"],
            false
        );
    }
}

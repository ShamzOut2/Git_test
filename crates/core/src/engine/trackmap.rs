//! Track outline recorder. iRacing does not expose track geometry, so the
//! outline is reconstructed from the player's own driving: heading (YawNorth)
//! integrated over distance travelled (LapDist) for one clean lap, then the
//! small closing error is spread over the lap. Maps are cached per TrackID.

use crate::model::{surface, SessionInfo, Telemetry};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const BINS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackMap {
    pub track_id: i64,
    pub name: String,
    /// Normalised outline, `BINS` points indexed by lap fraction, x/y in 0..1.
    pub points: Vec<[f32; 2]>,
    pub source: String,
}

pub struct Recorder {
    dir: Option<PathBuf>,
    /// Outline recorded from the player's driving (or set by the simulator).
    pub map: Option<TrackMap>,
    /// Outline from iRacing's official track map, when downloaded.
    pub official: Option<TrackMap>,
    track_id: i64,
    track_name: String,
    bins: Vec<Option<[f64; 2]>>,
    pos: [f64; 2],
    active: bool,
    last_dist: f32,
    last_pct: f32,
    /// Incremented whenever `map` changes so the topic can be re-sent.
    pub revision: u64,
}

impl Recorder {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self {
            dir,
            map: None,
            official: None,
            track_id: 0,
            track_name: String::new(),
            bins: vec![None; BINS],
            pos: [0.0; 2],
            active: false,
            last_dist: 0.0,
            last_pct: 0.0,
            revision: 0,
        }
    }

    fn path(&self, id: i64) -> Option<PathBuf> {
        self.dir.as_ref().map(|d| d.join(format!("{id}.json")))
    }

    pub fn load(&mut self, track_id: i64, name: &str) {
        self.track_id = track_id;
        self.track_name = name.to_string();
        self.active = false;
        self.map = self.path(track_id).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok());
        if self.official.as_ref().is_some_and(|m| m.track_id != track_id) {
            self.official = None;
        }
        self.revision += 1;
    }

    pub fn set_official(&mut self, map: TrackMap) {
        self.official = Some(map);
        self.revision += 1;
    }

    /// The outline to draw: iRacing's official map when preferred and available, else the
    /// recorded one (or the official one while nothing has been recorded).
    pub fn shown(&self, prefer_official: bool) -> Option<&TrackMap> {
        let official = self.official.as_ref().filter(|m| m.track_id == self.track_id);
        if prefer_official {
            official.or(self.map.as_ref())
        } else {
            self.map.as_ref().or(official)
        }
    }

    /// Install a map generated elsewhere (e.g. the mock simulator or an import).
    pub fn set(&mut self, map: TrackMap) {
        self.map = Some(map);
        self.revision += 1;
    }

    pub fn clear(&mut self) {
        if let Some(p) = self.path(self.track_id) {
            let _ = std::fs::remove_file(p);
        }
        self.map = None;
        self.active = false;
        self.revision += 1;
    }

    /// 0..1 while recording, 1 when a map exists.
    pub fn progress(&self) -> f32 {
        if self.map.is_some() {
            1.0
        } else if self.active {
            self.bins.iter().filter(|b| b.is_some()).count() as f32 / BINS as f32
        } else {
            0.0
        }
    }

    pub fn recording(&self) -> bool {
        self.map.is_none() && self.active
    }

    pub fn update(&mut self, t: &Telemetry, info: &SessionInfo) {
        if self.map.is_some() || info.track_id == 0 {
            return;
        }
        let me = t.player();
        let clean = t.is_on_track && !t.on_pit_road && t.speed > 5.0 && me.is_some_and(|c| c.surface == surface::ON_TRACK);
        if !clean {
            self.active = false;
            return;
        }
        let pct = t.lap_dist_pct;
        let crossed = self.last_pct > 0.9 && pct < 0.1;
        if crossed {
            if self.active && self.bins.iter().filter(|b| b.is_some()).count() > BINS * 9 / 10 {
                self.finish();
                return;
            }
            self.bins.iter_mut().for_each(|b| *b = None);
            self.pos = [0.0; 2];
            self.active = true;
        } else if self.active {
            let ds = (t.lap_dist - self.last_dist) as f64;
            if !(0.0..=60.0).contains(&ds) {
                // Teleport / reset to pits: abandon this lap.
                self.active = false;
            } else {
                let yaw = t.yaw_north as f64;
                self.pos[0] += ds * yaw.cos();
                self.pos[1] += ds * yaw.sin();
                let bin = ((pct.clamp(0.0, 0.9999)) * BINS as f32) as usize;
                self.bins[bin] = Some(self.pos);
            }
        }
        self.last_pct = pct;
        self.last_dist = t.lap_dist;
    }

    fn finish(&mut self) {
        // Close the loop: the end should coincide with the start (origin).
        let mut pts: Vec<[f64; 2]> = fill_gaps(&self.bins);
        let err = self.pos;
        let n = pts.len() as f64;
        for (i, p) in pts.iter_mut().enumerate() {
            let k = (i as f64 + 1.0) / n;
            p[0] -= err[0] * k;
            p[1] -= err[1] * k;
        }
        let map = TrackMap { track_id: self.track_id, name: self.track_name.clone(), points: normalise(&pts), source: "recorded".into() };
        if let Some(p) = self.path(self.track_id) {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, serde_json::to_string(&map).unwrap_or_default());
        }
        self.set(map);
        self.active = false;
    }
}

impl Recorder {
    /// Heading (radians, same convention as YawNorth) of the track at a lap fraction,
    /// from the recorded outline. Lets the compass follow any car, not only the player's.
    pub fn heading_at(&self, pct: f32) -> Option<f32> {
        let map = self.map.as_ref().filter(|m| m.source == "recorded" || m.source == "mock")?;
        let pts = &map.points;
        let n = pts.len();
        if n < 8 || !(0.0..=1.0).contains(&pct) {
            return None;
        }
        let i = (pct * n as f32) as usize;
        let (a, b) = (pts[(i + n - 2) % n], pts[(i + 2) % n]);
        // Outline y is flipped for screen space.
        let (dx, dy) = (b[0] - a[0], -(b[1] - a[1]));
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        // Recorded outlines are integrated from YawNorth itself; the simulator's circuit uses maths angles.
        Some(if map.source == "mock" { std::f32::consts::FRAC_PI_2 - dy.atan2(dx) } else { dy.atan2(dx) })
    }
}

fn fill_gaps(bins: &[Option<[f64; 2]>]) -> Vec<[f64; 2]> {
    let n = bins.len();
    let known: Vec<usize> = (0..n).filter(|&i| bins[i].is_some()).collect();
    if known.is_empty() {
        return vec![[0.0; 2]; n];
    }
    (0..n)
        .map(|i| {
            if let Some(p) = bins[i] {
                return p;
            }
            let next = known.iter().copied().find(|&k| k > i).unwrap_or(known[0] + n);
            let prev = known.iter().rev().copied().find(|&k| k < i).map(|k| k as isize).unwrap_or(known[known.len() - 1] as isize - n as isize);
            let (a, b) = (bins[(prev.rem_euclid(n as isize)) as usize].unwrap(), bins[next % n].unwrap());
            let f = (i as f64 - prev as f64) / (next as f64 - prev as f64);
            [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
        })
        .collect()
}

/// Scale into a unit box preserving aspect ratio, centred. Y is flipped for screen space.
pub fn normalise(pts: &[[f64; 2]]) -> Vec<[f32; 2]> {
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in pts {
        minx = minx.min(p[0]);
        maxx = maxx.max(p[0]);
        miny = miny.min(p[1]);
        maxy = maxy.max(p[1]);
    }
    let span = (maxx - minx).max(maxy - miny).max(1e-9);
    let (ox, oy) = ((span - (maxx - minx)) / 2.0, (span - (maxy - miny)) / 2.0);
    pts.iter()
        .map(|p| {
            let x = (p[0] - minx + ox) / span;
            let y = 1.0 - (p[1] - miny + oy) / span;
            [(x * 1e4).round() as f32 / 1e4, (y * 1e4).round() as f32 / 1e4]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CarTel;

    #[test]
    fn records_a_circle() {
        let info = SessionInfo { track_id: 7, ..Default::default() };
        let mut r = Recorder::new(None);
        r.load(7, "Circle");
        let mut t = Telemetry { is_on_track: true, speed: 50.0, player_idx: 0, ..Default::default() };
        t.cars[0] = CarTel { surface: surface::ON_TRACK, pct: 0.0, ..Default::default() };
        let len = 1000.0f32;
        let steps = 4000;
        for s in 0..=(steps * 2 + 10) {
            let f = (s % steps) as f32 / steps as f32;
            t.lap_dist_pct = f;
            t.lap_dist = f * len;
            t.yaw_north = f * std::f32::consts::TAU;
            r.update(&t, &info);
        }
        let m = r.map.clone().expect("map recorded");
        assert_eq!(m.points.len(), BINS);
        // Circle: every point is ~0.5 from the centre.
        for p in &m.points {
            let d = ((p[0] - 0.5).powi(2) + (p[1] - 0.5).powi(2)).sqrt();
            assert!((d - 0.5).abs() < 0.02, "{d}");
        }
        // The outline gives back the heading the car had at each point.
        for f in [0.1f32, 0.3, 0.6, 0.85] {
            let want = f * std::f32::consts::TAU;
            let got = r.heading_at(f).unwrap();
            let diff = (got - want).rem_euclid(std::f32::consts::TAU);
            assert!(diff.min(std::f32::consts::TAU - diff) < 0.05, "{f}: {got} vs {want}");
        }
    }
}

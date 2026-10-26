//! Derived race state, recomputed every telemetry tick (60 Hz).

pub mod battle;
pub mod fuel;
pub mod trackmap;
pub mod tyres;
pub mod weather;

use crate::config::Config;
use crate::league::{ClassKey, Resolver};
use crate::model::{flags, surface, SessionDef, SessionInfo, Telemetry, MAX_CARS};
use std::collections::VecDeque;
use std::path::PathBuf;

/// Session states (irsdk_SessionState).
pub const STATE_RACING: i32 = 4;
pub const STATE_CHECKERED: i32 = 5;

/// Per-car data that only changes with the session info string or config.
#[derive(Debug, Clone)]
pub struct Static {
    pub idx: usize,
    pub class: ClassKey,
    pub name: String,
    pub abbrev: String,
    pub team: String,
    pub number: String,
    pub car: String,
    pub car_id: i64,
    pub user_id: i64,
    pub irating: i32,
    pub lic: String,
    pub lic_color: u32,
    pub club: String,
    pub hidden: bool,
}

#[derive(Debug, Clone)]
pub struct LapRec {
    pub time: f32,
    pub wet: Option<bool>,
    pub at: f64,
    pub pit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CarHist {
    pub seen: bool,
    pub last_progress: f64,
    pub was_on_pit: bool,
    pub pit_stops: i32,
    pub last_pit_lap: i32,
    pub stint_start_lap: i32,
    pub start_class_pos: i32,
    pub last_lap_seen: f32,
    pub pit_this_lap: bool,
    pub laps: VecDeque<LapRec>,
}

impl CarHist {
    /// Median of the last few non-pit laps; a robust pace estimate.
    pub fn pace(&self) -> Option<f32> {
        let mut v: Vec<f32> = self.laps.iter().rev().filter(|l| !l.pit && l.time > 0.0).take(5).map(|l| l.time).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.total_cmp(b));
        Some(v[v.len() / 2])
    }
    pub fn last(&self) -> Option<&LapRec> {
        self.laps.back()
    }
}

/// One entry of the computed running order.
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub idx: usize,
    pub pos: i32,
    pub class_pos: i32,
    /// Gap to class leader; `None` with `gap_laps > 0` means laps down.
    pub gap: Option<f32>,
    pub gap_laps: i32,
    pub interval: Option<f32>,
    pub interval_laps: i32,
}

#[derive(Debug, Clone)]
pub struct ClassSummary {
    pub key: ClassKey,
    pub count: usize,
    pub sof: i32,
    pub best_lap: Option<(usize, f32)>,
}

pub struct Engine {
    pub tel: Telemetry,
    pub info: SessionInfo,
    pub connected: bool,
    pub source: &'static str,
    pub cfg: Config,
    pub resolver: Resolver,
    pub statics: Vec<Option<Static>>,
    pub hist: Vec<CarHist>,
    pub order: Vec<Row>,
    pub classes: Vec<ClassSummary>,
    pub fuel: fuel::FuelTracker,
    pub weather: weather::WeatherTracker,
    pub battle: battle::BattleTracker,
    pub map: trackmap::Recorder,
    last_session: (i64, i32),
    last_state: i32,
}

impl Engine {
    pub fn new(cfg: Config, data_dir: Option<PathBuf>) -> Self {
        let mut e = Self {
            tel: Telemetry::default(),
            info: SessionInfo::default(),
            connected: false,
            source: "none",
            resolver: Resolver::default(),
            cfg,
            statics: vec![None; MAX_CARS],
            hist: vec![CarHist::default(); MAX_CARS],
            order: vec![],
            classes: vec![],
            fuel: Default::default(),
            weather: Default::default(),
            battle: Default::default(),
            map: trackmap::Recorder::new(data_dir.map(|d| d.join("trackmaps"))),
            last_session: (-1, -1),
            last_state: 0,
        };
        e.set_config(e.cfg.clone());
        e
    }

    pub fn set_config(&mut self, cfg: Config) {
        let league = cfg.active_league.as_ref().and_then(|id| cfg.leagues.iter().find(|l| &l.id == id)).cloned();
        self.resolver = Resolver::new(league);
        self.cfg = cfg;
        self.rebuild_statics();
    }

    pub fn set_session(&mut self, info: SessionInfo) {
        let track_changed = info.track_id != self.info.track_id;
        self.info = info;
        if track_changed {
            self.map.load(self.info.track_id, &self.info.track_display_name);
        }
        self.rebuild_statics();
    }

    fn rebuild_statics(&mut self) {
        let mut statics = vec![None; MAX_CARS];
        for d in &self.info.drivers {
            if d.car_idx >= MAX_CARS || d.is_spectator || d.is_pace_car || d.car_idx as i32 == self.info.pace_car_idx {
                continue;
            }
            let (name, team, hidden) = self.resolver.display(d);
            statics[d.car_idx] = Some(Static {
                idx: d.car_idx,
                class: self.resolver.class_of(d),
                abbrev: if d.abbrev.is_empty() { name.clone() } else { d.abbrev.clone() },
                name,
                team,
                number: d.car_number.clone(),
                car: if d.car_short.is_empty() { d.car_name.clone() } else { d.car_short.clone() },
                car_id: d.car_id,
                user_id: d.user_id,
                irating: d.irating,
                lic: d.lic_string.clone(),
                lic_color: d.lic_color,
                club: d.club.clone(),
                hidden,
            });
        }
        self.statics = statics;
    }

    /// Forget everything about the current session (simulator closed).
    pub fn reset(&mut self) {
        self.connected = false;
        self.tel = Telemetry::default();
        self.set_session(SessionInfo::default());
        self.hist = vec![CarHist::default(); MAX_CARS];
        self.order.clear();
        self.classes.clear();
        self.battle = Default::default();
        self.last_session = (-1, -1);
    }

    pub fn session_def(&self) -> Option<&SessionDef> {
        self.info.session(self.tel.session_num)
    }

    pub fn is_race(&self) -> bool {
        self.session_def().is_some_and(|s| s.is_race())
    }

    pub fn under_caution(&self) -> bool {
        self.tel.session_flags & (flags::CAUTION | flags::CAUTION_WAVING) != 0
    }

    pub fn static_of(&self, idx: usize) -> Option<&Static> {
        self.statics.get(idx).and_then(|s| s.as_ref())
    }

    pub fn progress(&self, idx: usize) -> f64 {
        let c = &self.tel.cars[idx];
        if c.in_world() {
            c.progress()
        } else {
            self.hist[idx].last_progress
        }
    }

    /// Reference lap time used to convert track distance into time.
    pub fn ref_lap(&self, idx: usize) -> f32 {
        if let Some(p) = self.hist.get(idx).and_then(|h| h.pace()) {
            return p;
        }
        let est = self.info.driver(idx).map(|d| d.class_est_lap).unwrap_or(0.0);
        if est > 0.0 {
            est
        } else if self.info.est_lap_time > 0.0 {
            self.info.est_lap_time
        } else {
            100.0
        }
    }

    /// Time from `behind` to `ahead` in race terms. Returns (seconds, whole laps).
    pub fn time_between(&self, behind: usize, ahead: usize) -> (Option<f32>, i32) {
        let dp = self.progress(ahead) - self.progress(behind);
        if dp >= 1.0 {
            return (None, dp.floor() as i32);
        }
        if dp < 0.0 {
            return (Some(0.0), 0);
        }
        let (a, b) = (&self.tel.cars[ahead], &self.tel.cars[behind]);
        let l = self.ref_lap(behind);
        let t = if a.est_time > 0.0 || b.est_time > 0.0 {
            let mut t = a.est_time - b.est_time;
            if a.pct < b.pct {
                t += l;
            }
            t
        } else {
            dp as f32 * l
        };
        (Some(t.clamp(0.0, l)), 0)
    }

    /// On-track gap from the focus car to `idx` (positive = `idx` is ahead on track).
    pub fn relative_gap(&self, focus: usize, idx: usize) -> f32 {
        let (f, c) = (&self.tel.cars[focus], &self.tel.cars[idx]);
        let mut dp = c.pct - f.pct;
        if dp > 0.5 {
            dp -= 1.0;
        } else if dp < -0.5 {
            dp += 1.0;
        }
        let l = self.ref_lap(focus);
        if f.est_time <= 0.0 && c.est_time <= 0.0 {
            return dp * l;
        }
        let mut t = c.est_time - f.est_time;
        if dp > 0.0 && t < 0.0 {
            t += l;
        } else if dp < 0.0 && t > 0.0 {
            t -= l;
        }
        t.clamp(-l / 2.0, l / 2.0)
    }

    /// Advance all derived state after the source has written a new telemetry row.
    pub fn tick(&mut self) {
        let key = (self.info.subsession_id, self.tel.session_num);
        if key != self.last_session {
            self.last_session = key;
            self.hist = vec![CarHist::default(); MAX_CARS];
            self.battle = Default::default();
            self.fuel.new_session();
        }
        self.update_hist();
        self.compute_order();
        if self.tel.session_state == STATE_RACING && self.last_state != STATE_RACING && self.is_race() {
            for r in &self.order {
                self.hist[r.idx].start_class_pos = r.class_pos;
            }
        }
        self.last_state = self.tel.session_state;
        let caution = self.under_caution();
        self.fuel.update(&self.tel, caution, &self.cfg.fuel);
        self.weather.update(&self.tel);
        self.map.update(&self.tel, &self.info);
        battle::update(self);
    }

    fn update_hist(&mut self) {
        let now = self.tel.session_time;
        for i in 0..MAX_CARS {
            let c = &self.tel.cars[i];
            let wet = self.info.tire_is_wet(c.tire_compound);
            let h = &mut self.hist[i];
            if !c.in_world() {
                continue;
            }
            h.seen = true;
            h.last_progress = c.progress();
            if c.on_pit_road {
                h.pit_this_lap = true;
            }
            if c.on_pit_road && !h.was_on_pit {
                h.pit_stops += 1;
                h.last_pit_lap = c.lap;
            }
            if !c.on_pit_road && h.was_on_pit {
                h.stint_start_lap = c.lap;
            }
            h.was_on_pit = c.on_pit_road;
            if c.last_lap > 0.0 && (c.last_lap - h.last_lap_seen).abs() > 1e-4 {
                h.last_lap_seen = c.last_lap;
                h.laps.push_back(LapRec { time: c.last_lap, wet, at: now, pit: h.pit_this_lap });
                h.pit_this_lap = c.on_pit_road;
                if h.laps.len() > 30 {
                    h.laps.pop_front();
                }
            }
        }
    }

    fn compute_order(&mut self) {
        let t = &self.tel;
        let race = self.is_race();
        let results = self.session_def().map(|s| s.results.as_slice()).unwrap_or(&[]);
        let result_pos = |idx: usize| results.iter().find(|r| r.car_idx == idx).map(|r| r.position).unwrap_or(0);
        let best = |idx: usize| {
            let b = t.cars[idx].best_lap;
            if b > 0.0 {
                return b;
            }
            results.iter().find(|r| r.car_idx == idx).map(|r| r.fastest_time).filter(|&v| v > 0.0).unwrap_or(f32::MAX)
        };
        let mut idxs: Vec<usize> = (0..MAX_CARS)
            .filter(|&i| self.statics[i].as_ref().is_some_and(|s| !s.hidden))
            .filter(|&i| t.cars[i].in_world() || self.hist[i].seen || result_pos(i) > 0)
            .collect();
        let num = |i: usize| self.statics[i].as_ref().and_then(|s| s.number.parse::<i32>().ok()).unwrap_or(9999);
        let pos_or_max = |p: i32| if p > 0 { p } else { i32::MAX };
        if race && t.session_state == STATE_RACING {
            idxs.sort_by(|&a, &b| self.progress(b).total_cmp(&self.progress(a)));
        } else if race {
            idxs.sort_by_key(|&i| {
                let p = if t.cars[i].position > 0 { t.cars[i].position } else { result_pos(i) };
                (pos_or_max(p), num(i))
            });
        } else {
            idxs.sort_by(|&a, &b| best(a).total_cmp(&best(b)).then(num(a).cmp(&num(b))));
        }

        let mut rows: Vec<Row> = Vec::with_capacity(idxs.len());
        let mut class_ids: Vec<String> = vec![];
        let mut class_count: Vec<i32> = vec![];
        let mut leader_of: Vec<usize> = vec![];
        let mut prev_of: Vec<usize> = vec![];
        for (n, &idx) in idxs.iter().enumerate() {
            let cid = &self.statics[idx].as_ref().unwrap().class.id;
            let ci = match class_ids.iter().position(|c| c == cid) {
                Some(ci) => ci,
                None => {
                    class_ids.push(cid.clone());
                    class_count.push(0);
                    leader_of.push(idx);
                    prev_of.push(idx);
                    class_ids.len() - 1
                }
            };
            class_count[ci] += 1;
            let mut row = Row { idx, pos: n as i32 + 1, class_pos: class_count[ci], ..Default::default() };
            if class_count[ci] > 1 {
                if race {
                    (row.gap, row.gap_laps) = self.time_between(idx, leader_of[ci]);
                    (row.interval, row.interval_laps) = self.time_between(idx, prev_of[ci]);
                } else if best(idx) < f32::MAX {
                    row.gap = Some(best(idx) - best(leader_of[ci]));
                    row.interval = Some(best(idx) - best(prev_of[ci]));
                }
            }
            prev_of[ci] = idx;
            rows.push(row);
        }
        self.order = rows;
        self.compute_classes();
    }

    fn compute_classes(&mut self) {
        let mut out: Vec<ClassSummary> = vec![];
        let mut irs: Vec<Vec<i32>> = vec![];
        for r in &self.order {
            let s = self.statics[r.idx].as_ref().unwrap();
            let i = match out.iter().position(|c| c.key.id == s.class.id) {
                Some(i) => i,
                None => {
                    out.push(ClassSummary { key: s.class.clone(), count: 0, sof: 0, best_lap: None });
                    irs.push(vec![]);
                    out.len() - 1
                }
            };
            out[i].count += 1;
            irs[i].push(s.irating);
            let b = self.tel.cars[r.idx].best_lap;
            if b > 0.0 && out[i].best_lap.is_none_or(|(_, t)| b < t) {
                out[i].best_lap = Some((r.idx, b));
            }
        }
        for (c, ir) in out.iter_mut().zip(&irs) {
            c.sof = sof(ir);
        }
        out.sort_by(|a, b| a.key.order.total_cmp(&b.key.order));
        self.classes = out;
    }

    pub fn row(&self, idx: usize) -> Option<&Row> {
        self.order.iter().find(|r| r.idx == idx)
    }

    pub fn on_track(&self, idx: usize) -> bool {
        self.tel.cars[idx].surface == surface::ON_TRACK
    }
}

/// iRacing's strength-of-field formula.
pub fn sof(irs: &[i32]) -> i32 {
    let v: Vec<f64> = irs.iter().filter(|&&i| i > 0).map(|&i| i as f64).collect();
    if v.is_empty() {
        return 0;
    }
    let k = 1600.0 / std::f64::consts::LN_2;
    let sum: f64 = v.iter().map(|ir| (-ir / k).exp()).sum();
    (k * (v.len() as f64 / sum).ln()).round() as i32
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::CarTel;

    pub fn session_yaml(n: usize, race: bool) -> String {
        let mut s = String::from("WeekendInfo:\n TrackID: 1\n TrackDisplayName: Test\nSessionInfo:\n Sessions:\n - SessionNum: 0\n");
        s += &format!("   SessionType: {}\n   SessionLaps: 20\n", if race { "Race" } else { "Practice" });
        s += "DriverInfo:\n DriverCarIdx: 0\n DriverCarEstLapTime: 100.0\n Drivers:\n";
        for i in 0..n {
            let class = if i % 2 == 0 { 1 } else { 2 };
            s += &format!(" - CarIdx: {i}\n   UserName: Driver {i}\n   CarNumber: \"{i}\"\n   CarClassID: {class}\n   CarClassShortName: C{class}\n   CarClassEstLapTime: 100.0\n   IRating: {}\n", 1500 + i * 100);
        }
        s
    }

    pub fn engine(n: usize, race: bool) -> Engine {
        let mut e = Engine::new(Config::default(), None);
        e.set_session(SessionInfo::parse(&session_yaml(n, race)));
        e.tel.player_idx = 0;
        e.tel.session_state = STATE_RACING;
        e
    }

    pub fn place(e: &mut Engine, idx: usize, lap: i32, pct: f32) {
        e.tel.cars[idx] = CarTel { lap, pct, est_time: pct * 100.0, surface: surface::ON_TRACK, ..Default::default() };
    }

    #[test]
    fn race_order_and_class_gaps() {
        let mut e = engine(4, true);
        place(&mut e, 0, 3, 0.50);
        place(&mut e, 1, 3, 0.60);
        place(&mut e, 2, 3, 0.40);
        place(&mut e, 3, 2, 0.90);
        e.tick();
        let order: Vec<usize> = e.order.iter().map(|r| r.idx).collect();
        assert_eq!(order, vec![1, 0, 2, 3]);
        let r2 = e.row(2).unwrap();
        assert_eq!(r2.class_pos, 2);
        assert!((r2.gap.unwrap() - 10.0).abs() < 1e-3, "{:?}", r2.gap);
        let r3 = e.row(3).unwrap();
        assert_eq!(r3.class_pos, 2);
        assert!((r3.gap.unwrap() - 70.0).abs() < 1e-3, "{:?}", r3.gap);
        place(&mut e, 3, 1, 0.90);
        e.tick();
        assert_eq!(e.row(3).unwrap().gap_laps, 1);
    }

    #[test]
    fn relative_wraps_across_line() {
        let mut e = engine(2, true);
        place(&mut e, 0, 3, 0.95);
        place(&mut e, 1, 4, 0.05);
        assert!((e.relative_gap(0, 1) - 10.0).abs() < 1e-3);
        assert!((e.relative_gap(1, 0) + 10.0).abs() < 1e-3);
    }

    #[test]
    fn strength_of_field() {
        assert_eq!(sof(&[2000, 2000]), 2000);
        let s = sof(&[1000, 3000]);
        assert!(s > 1000 && s < 3000);
    }
}

//! Derived race state, recomputed every telemetry tick (60 Hz).

pub mod battle;
pub mod fuel;
pub mod ratings;
pub mod trackmap;
pub mod tyres;
pub mod weather;

use crate::config::{Config, Corner};
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
    pub country: String,
    pub make: String,
    pub hidden: bool,
    /// iRacing car class (iRating is calculated per iRacing class even when league classes are shown).
    pub ir_class: i64,
    pub lic_class: char,
    /// Expected score against the drivers of the same iRacing class (for the iRating estimate).
    pub ir_expected: f32,
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
    /// Left the pits and has not crossed the line yet.
    pub out_lap: bool,
    /// Took the chequered flag.
    pub finished: bool,
    pub prev_completed: i32,
    pub last_cross_at: f64,
    pub laps_led_overall: i32,
    pub laps_led_class: i32,
    pub pitted: bool,
    pub exit_completed: i32,
}

/// Where a car is in its stint, for the "stint lap" column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stint {
    Pit,
    OutLap,
    /// Lap number within the stint (race lap number before the first stop).
    Lap(i32),
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
    /// Corners of the current track (custom > downloaded > source-provided).
    pub corners: Vec<Corner>,
    pub corners_source: &'static str,
    remote_corners: Option<(i64, Vec<Corner>, &'static str)>,
    /// Session time the leader took the chequered flag.
    checkered_at: Option<f64>,
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
            corners: vec![],
            corners_source: "none",
            remote_corners: None,
            checkered_at: None,
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
        self.refresh_corners();
    }

    /// Corner data fetched or generated outside the engine for `track_id`.
    pub fn set_remote_corners(&mut self, track_id: i64, corners: Vec<Corner>, source: &'static str) {
        self.remote_corners = Some((track_id, corners, source));
        self.refresh_corners();
    }

    fn refresh_corners(&mut self) {
        let id = self.info.track_id;
        if let Some(c) = self.cfg.track_corners.get(&id.to_string()).filter(|c| !c.is_empty()) {
            self.corners = c.clone();
            self.corners_source = "custom";
        } else if let Some((_, c, src)) = self.remote_corners.as_ref().filter(|r| r.0 == id) {
            self.corners = c.clone();
            self.corners_source = src;
        } else {
            self.corners.clear();
            self.corners_source = "none";
        }
        self.map.revision += 1;
    }

    /// Corners per lap, estimated from the track length when no corner data exists.
    pub fn corners_per_lap(&self) -> f32 {
        if !self.corners.is_empty() {
            self.corners.len() as f32
        } else {
            (self.info.track_length_km * 3.0).max(6.0)
        }
    }

    pub fn stint(&self, idx: usize) -> Stint {
        let (c, h) = (&self.tel.cars[idx], &self.hist[idx]);
        if c.on_pit_road {
            Stint::Pit
        } else if h.out_lap {
            Stint::OutLap
        } else if h.pitted {
            Stint::Lap((c.lap_completed - h.exit_completed).max(1))
        } else {
            Stint::Lap(c.lap.max(1))
        }
    }

    /// Incidents of a car this session (live for the player, from the session string otherwise).
    pub fn incidents(&self, idx: usize) -> i32 {
        if idx as i32 == self.tel.player_idx {
            self.tel.incidents
        } else {
            self.info.driver(idx).map(|d| d.incidents).unwrap_or(0)
        }
    }

    /// Other drivers' incident counts are hidden in many sessions; assume visible once any is non-zero.
    pub fn incidents_visible(&self, idx: usize) -> bool {
        idx as i32 == self.tel.player_idx || self.info.drivers.iter().any(|d| d.incidents > 0 && d.car_idx as i32 != self.tel.player_idx)
    }

    pub fn ir_delta(&self, idx: usize) -> Option<f32> {
        if !self.is_race() {
            return None;
        }
        let s = self.static_of(idx)?;
        let group: Vec<usize> = self.order.iter().map(|r| r.idx).filter(|&i| self.static_of(i).is_some_and(|o| o.ir_class == s.ir_class)).collect();
        let pos = group.iter().position(|&i| i == idx)? + 1;
        Some(ratings::ir_change(group.len(), pos, s.ir_expected))
    }

    pub fn sr_delta(&self, idx: usize) -> Option<f32> {
        let s = self.static_of(idx)?;
        if !self.incidents_visible(idx) {
            return None;
        }
        let c = &self.tel.cars[idx];
        let per_lap = self.corners_per_lap();
        let passed = if self.corners.is_empty() { c.pct.max(0.0) * per_lap } else { self.corners.iter().filter(|k| k.end <= c.pct).count() as f32 };
        let corners = c.lap_completed.max(0) as f32 * per_lap + passed;
        Some(ratings::sr_change(s.lic_class, corners, self.incidents(idx)))
    }

    /// League points for the current running position, and the projected championship total.
    /// Best lap this session, from telemetry or (for cars no longer in the world) the results.
    pub fn best_lap(&self, idx: usize) -> Option<f32> {
        let b = self.tel.cars[idx].best_lap;
        if b > 0.0 {
            return Some(b);
        }
        self.result(idx).map(|r| r.fastest_time).filter(|&t| t > 0.0)
    }

    pub fn last_lap(&self, idx: usize) -> Option<f32> {
        let l = self.tel.cars[idx].last_lap;
        if l > 0.0 {
            return Some(l);
        }
        self.result(idx).map(|r| r.last_time).filter(|&t| t > 0.0)
    }

    fn result(&self, idx: usize) -> Option<&crate::model::ResultPos> {
        self.session_def().and_then(|d| d.results.iter().find(|r| r.car_idx == idx))
    }

    /// Qualifying order (car indices) for pole bonuses: carried-over results, else the last qualifying session.
    fn qualify_order(&self) -> Vec<usize> {
        if !self.info.qualify.is_empty() {
            return self.info.qualify.iter().map(|r| r.car_idx).collect();
        }
        let q = self.info.sessions.iter().rfind(|s| s.kind.to_lowercase().contains("qualify") && !s.results.is_empty());
        let mut res: Vec<&crate::model::ResultPos> = q.map(|s| s.results.iter().collect()).unwrap_or_default();
        res.sort_by_key(|r| r.position);
        res.iter().map(|r| r.car_idx).collect()
    }

    /// Cars in the same points group as `idx` (its class, or everyone for overall scoring).
    fn points_group(&self, idx: usize, overall: bool) -> Vec<usize> {
        let class = self.static_of(idx).map(|s| s.class.id.clone());
        self.order.iter().map(|r| r.idx).filter(|&i| overall || self.static_of(i).map(|s| &s.class.id) == class.as_ref()).collect()
    }

    pub fn points(&self, idx: usize) -> (Option<f32>, Option<f32>) {
        let Some(league) = self.resolver.league() else { return (None, None) };
        let (Some(r), Some(s)) = (self.row(idx), self.static_of(idx)) else { return (None, None) };
        if !self.is_race() {
            return (None, None);
        }
        let p = &league.points;
        let overall = p.basis == "overall";
        let pos = if overall { r.pos } else { r.class_pos };
        let fastest = if overall {
            self.classes.iter().filter_map(|c| c.best_lap).min_by(|a, b| a.1.total_cmp(&b.1)).map(|b| b.0)
        } else {
            self.classes.iter().find(|c| c.key.id == s.class.id).and_then(|c| c.best_lap).map(|b| b.0)
        };
        let group = self.points_group(idx, overall);
        let pole = self.qualify_order().into_iter().find(|i| group.contains(i));
        let led = |i: usize| if overall { self.hist[i].laps_led_overall } else { self.hist[i].laps_led_class };
        let most = group.iter().copied().max_by_key(|&i| led(i)).filter(|&i| led(i) > 0);
        let pts = ratings::points(p, pos, fastest == Some(idx)).map(|base| {
            base + if pole == Some(idx) { p.pole_bonus } else { 0.0 }
                + if led(idx) > 0 { p.lap_led_bonus } else { 0.0 }
                + if most == Some(idx) { p.most_laps_led_bonus } else { 0.0 }
                + p.finish_bonus
        });
        let season = league.season_points.iter().find(|e| e.customer_id == s.user_id).map(|e| e.points);
        let total = match (season, pts) {
            (Some(a), Some(b)) => Some(a + b),
            (Some(a), None) => Some(a),
            (None, b) if !league.season_points.is_empty() => b,
            _ => None,
        };
        (pts, total)
    }

    pub fn set_session(&mut self, info: SessionInfo) {
        let track_changed = info.track_id != self.info.track_id;
        self.info = info;
        if track_changed {
            self.map.load(self.info.track_id, &self.info.track_display_name);
        }
        self.rebuild_statics();
        if track_changed {
            self.refresh_corners();
        }
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
                number: self.resolver.car_number(d),
                car: if d.car_short.is_empty() { d.car_name.clone() } else { d.car_short.clone() },
                car_id: d.car_id,
                user_id: d.user_id,
                irating: d.irating,
                lic: d.lic_string.clone(),
                lic_color: d.lic_color,
                club: d.club.clone(),
                country: if override_country(&self.resolver, d.user_id).is_empty() { d.country.clone() } else { override_country(&self.resolver, d.user_id) },
                make: d.make.clone(),
                hidden,
                ir_class: d.class_id,
                lic_class: d.licence().0,
                ir_expected: 0.0,
            });
        }
        // Expected scores per iRacing class for the iRating estimate (iRatings only change with the session string).
        let mut groups: std::collections::BTreeMap<i64, Vec<usize>> = Default::default();
        for s in statics.iter().flatten() {
            groups.entry(s.ir_class).or_default().push(s.idx);
        }
        for idxs in groups.values() {
            let irs: Vec<i32> = idxs.iter().map(|&i| statics[i].as_ref().unwrap().irating).collect();
            for (k, e) in ratings::expected_scores(&irs).into_iter().enumerate() {
                statics[idxs[k]].as_mut().unwrap().ir_expected = e;
            }
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
            self.checkered_at = None;
        }
        self.update_hist();
        self.compute_order();
        if self.is_race() && self.tel.session_state >= STATE_CHECKERED && self.checkered_at.is_none() {
            // The leader just crossed the line under the chequered flag.
            let now = self.tel.session_time;
            self.checkered_at = Some(now);
            if let Some(leader) = self.order.first().map(|r| r.idx) {
                if now - self.hist[leader].last_cross_at < 2.0 {
                    self.hist[leader].finished = true;
                }
            }
        }
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
        let racing = self.is_race() && self.tel.session_state >= STATE_RACING;
        // Running order from the previous tick decides who led the lap that just ended.
        let overall_leader = self.order.first().map(|r| r.idx);
        let class_leaders: Vec<usize> = self.order.iter().filter(|r| r.class_pos == 1).map(|r| r.idx).collect();
        let checkered_at = self.checkered_at;
        for i in 0..MAX_CARS {
            let c = &self.tel.cars[i];
            let wet = self.info.tire_is_wet(c.tire_compound);
            let h = &mut self.hist[i];
            if !c.in_world() {
                continue;
            }
            if h.seen && c.lap_completed > h.prev_completed {
                h.last_cross_at = now;
                if racing {
                    h.laps_led_overall += (overall_leader == Some(i)) as i32;
                    h.laps_led_class += class_leaders.contains(&i) as i32;
                }
                if checkered_at.is_some_and(|t| now >= t - 0.05) {
                    h.finished = true;
                }
            }
            h.prev_completed = c.lap_completed;
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
                h.out_lap = true;
                h.pitted = true;
                h.exit_completed = c.lap_completed;
            }
            if h.out_lap && c.lap_completed > h.exit_completed {
                h.out_lap = false;
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
            let b = self.best_lap(r.idx).unwrap_or(0.0);
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

fn override_country(r: &Resolver, user_id: i64) -> String {
    r.league().and_then(|l| l.driver_overrides.iter().find(|o| o.customer_id == user_id)).map(|o| o.country.trim().to_uppercase()).unwrap_or_default()
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
    fn chequered_flag_laps_led_and_bonus_points() {
        use crate::config::{League, PointsConfig};
        let cfg = Config {
            leagues: vec![League {
                id: "l".into(),
                points: PointsConfig {
                    table: vec![25.0, 18.0, 15.0],
                    basis: "overall".into(),
                    lap_led_bonus: 1.0,
                    most_laps_led_bonus: 2.0,
                    finish_bonus: 0.5,
                    ..Default::default()
                },
                ..Default::default()
            }],
            active_league: Some("l".into()),
            ..Default::default()
        };
        let mut e = engine(3, true);
        e.set_config(cfg);
        let lap = |e: &mut Engine, t: f64, cars: [(i32, f32); 3]| {
            e.tel.session_time = t;
            for (i, (l, p)) in cars.into_iter().enumerate() {
                place(e, i, l, p);
                e.tel.cars[i].lap_completed = l - 1;
            }
            e.tick();
        };
        lap(&mut e, 0.0, [(1, 0.95), (1, 0.90), (1, 0.85)]);
        // Car 0 leads across the line twice.
        lap(&mut e, 10.0, [(2, 0.01), (1, 0.97), (1, 0.93)]);
        lap(&mut e, 110.0, [(3, 0.01), (2, 0.97), (2, 0.93)]);
        assert_eq!(e.hist[0].laps_led_overall, 2);
        assert_eq!(e.points(0).0, Some(25.0 + 1.0 + 2.0 + 0.5));
        assert_eq!(e.points(1).0, Some(18.5));
        // Chequered flag: the leader finishes, the others when they next cross the line.
        e.tel.session_state = STATE_CHECKERED;
        lap(&mut e, 210.0, [(4, 0.01), (2, 0.99), (2, 0.98)]);
        assert!(e.hist[0].finished);
        assert!(!e.hist[1].finished);
        lap(&mut e, 212.0, [(4, 0.03), (3, 0.01), (2, 0.99)]);
        assert!(e.hist[1].finished);
        assert!(!e.hist[2].finished);
    }

    #[test]
    fn strength_of_field() {
        assert_eq!(sof(&[2000, 2000]), 2000);
        let s = sof(&[1000, 3000]);
        assert!(s > 1000 && s < 3000);
    }
}

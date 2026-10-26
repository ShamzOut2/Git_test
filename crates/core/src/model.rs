//! Normalised simulator state. Sources (live iRacing, mock) fill these structs;
//! everything downstream only depends on this module, never on raw SDK vars.

use apex_irsdk::yaml::{self, Yaml};

pub const MAX_CARS: usize = 64;

/// irsdk_TrkLoc
pub mod surface {
    pub const NOT_IN_WORLD: i32 = -1;
    pub const OFF_TRACK: i32 = 0;
    pub const IN_PIT_STALL: i32 = 1;
    pub const APPROACHING_PITS: i32 = 2;
    pub const ON_TRACK: i32 = 3;
}

/// irsdk_Flags
pub mod flags {
    pub const CHECKERED: u32 = 0x0001;
    pub const WHITE: u32 = 0x0002;
    pub const GREEN: u32 = 0x0004;
    pub const YELLOW: u32 = 0x0008;
    pub const RED: u32 = 0x0010;
    pub const BLUE: u32 = 0x0020;
    pub const DEBRIS: u32 = 0x0040;
    pub const YELLOW_WAVING: u32 = 0x0100;
    pub const ONE_LAP_TO_GREEN: u32 = 0x0200;
    pub const CAUTION: u32 = 0x4000;
    pub const CAUTION_WAVING: u32 = 0x8000;
    pub const BLACK: u32 = 0x10000;
    pub const DISQUALIFY: u32 = 0x20000;
    pub const REPAIR: u32 = 0x100000;

    pub fn names(f: u32) -> Vec<&'static str> {
        const ALL: &[(u32, &str)] = &[
            (CHECKERED, "checkered"),
            (WHITE, "white"),
            (GREEN, "green"),
            (YELLOW, "yellow"),
            (YELLOW_WAVING, "yellowWaving"),
            (CAUTION, "caution"),
            (CAUTION_WAVING, "cautionWaving"),
            (RED, "red"),
            (BLUE, "blue"),
            (DEBRIS, "debris"),
            (ONE_LAP_TO_GREEN, "oneLapToGreen"),
            (BLACK, "black"),
            (DISQUALIFY, "disqualify"),
            (REPAIR, "repair"),
        ];
        ALL.iter().filter(|(b, _)| f & b != 0).map(|(_, n)| *n).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CarTel {
    pub lap: i32,
    pub lap_completed: i32,
    pub pct: f32,
    pub est_time: f32,
    pub f2_time: f32,
    pub position: i32,
    pub class_position: i32,
    pub on_pit_road: bool,
    pub surface: i32,
    pub last_lap: f32,
    pub best_lap: f32,
    pub tire_compound: i32,
    pub gear: i32,
    pub rpm: f32,
}

impl CarTel {
    /// Laps completed plus fraction of the current lap.
    pub fn progress(&self) -> f64 {
        self.lap.max(0) as f64 + self.pct.clamp(0.0, 1.0) as f64
    }
    pub fn in_world(&self) -> bool {
        self.surface != surface::NOT_IN_WORLD && self.pct >= 0.0
    }
}

#[derive(Debug, Clone)]
pub struct Telemetry {
    pub tick: i32,
    pub session_time: f64,
    pub session_num: i32,
    pub session_state: i32,
    pub session_flags: u32,
    pub session_time_remain: f64,
    pub session_laps_remain: i32,
    pub time_of_day: f32,
    pub player_idx: i32,
    /// Car the camera is following (used as focus when spectating).
    pub cam_car_idx: i32,

    pub speed: f32,
    pub rpm: f32,
    pub gear: i32,
    pub throttle: f32,
    pub brake: f32,
    /// 0 = pedal released, 1 = pedal fully pressed (already inverted from the SDK's engagement value).
    pub clutch: f32,
    pub steering: f32,
    pub steering_max: f32,
    pub abs_active: bool,

    pub fuel_level: f32,
    pub fuel_level_pct: f32,
    pub fuel_use_per_hour: f32,

    pub lap: i32,
    pub lap_completed: i32,
    pub lap_dist_pct: f32,
    pub lap_dist: f32,
    pub lap_current_time: f32,
    pub lap_last_time: f32,
    pub lap_best_time: f32,
    pub delta_best: f32,
    pub delta_best_ok: bool,

    pub on_pit_road: bool,
    pub is_on_track: bool,
    pub incidents: i32,
    pub yaw_north: f32,
    pub lat_accel: f32,
    pub long_accel: f32,
    pub tire_compound: i32,

    pub track_temp: f32,
    pub air_temp: f32,
    pub track_wetness: i32,
    pub precipitation: f32,
    pub declared_wet: bool,
    pub wind_vel: f32,
    pub wind_dir: f32,
    pub humidity: f32,
    pub fog: f32,
    pub skies: i32,
    pub air_pressure: f32,
    pub display_metric: bool,

    pub cars: Vec<CarTel>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            tick: 0,
            session_time: 0.0,
            session_num: 0,
            session_state: 0,
            session_flags: 0,
            session_time_remain: 0.0,
            session_laps_remain: 0,
            time_of_day: 0.0,
            player_idx: -1,
            cam_car_idx: -1,
            speed: 0.0,
            rpm: 0.0,
            gear: 0,
            throttle: 0.0,
            brake: 0.0,
            clutch: 0.0,
            steering: 0.0,
            steering_max: 0.0,
            abs_active: false,
            fuel_level: 0.0,
            fuel_level_pct: 0.0,
            fuel_use_per_hour: 0.0,
            lap: 0,
            lap_completed: 0,
            lap_dist_pct: 0.0,
            lap_dist: 0.0,
            lap_current_time: 0.0,
            lap_last_time: 0.0,
            lap_best_time: 0.0,
            delta_best: 0.0,
            delta_best_ok: false,
            on_pit_road: false,
            is_on_track: false,
            incidents: 0,
            yaw_north: 0.0,
            lat_accel: 0.0,
            long_accel: 0.0,
            tire_compound: 0,
            track_temp: 0.0,
            air_temp: 0.0,
            track_wetness: 0,
            precipitation: 0.0,
            declared_wet: false,
            wind_vel: 0.0,
            wind_dir: 0.0,
            humidity: 0.0,
            fog: 0.0,
            skies: 0,
            air_pressure: 0.0,
            display_metric: true,
            cars: vec![CarTel { surface: surface::NOT_IN_WORLD, pct: -1.0, ..Default::default() }; MAX_CARS],
        }
    }
}

impl Telemetry {
    pub fn player(&self) -> Option<&CarTel> {
        usize::try_from(self.player_idx).ok().and_then(|i| self.cars.get(i))
    }

    /// The car overlays centre on: the player's car while driving, otherwise the camera car.
    pub fn focus_idx(&self) -> Option<usize> {
        let ok = |i: i32| usize::try_from(i).ok().filter(|&i| self.cars.get(i).is_some_and(|c| c.in_world()));
        ok(self.player_idx).or_else(|| ok(self.cam_car_idx))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Driver {
    pub car_idx: usize,
    pub name: String,
    pub abbrev: String,
    pub initials: String,
    pub user_id: i64,
    pub team_id: i64,
    pub team_name: String,
    pub car_number: String,
    pub car_id: i64,
    pub car_path: String,
    pub car_name: String,
    pub car_short: String,
    pub class_id: i64,
    pub class_name: String,
    pub class_color: u32,
    pub class_est_lap: f32,
    pub irating: i32,
    pub lic_string: String,
    pub lic_color: u32,
    pub club: String,
    pub is_spectator: bool,
    pub is_pace_car: bool,
    pub is_ai: bool,
    pub incidents: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ResultPos {
    pub position: i32,
    pub class_position: i32,
    pub car_idx: usize,
    pub laps_complete: i32,
    pub fastest_time: f32,
    pub last_time: f32,
    pub incidents: i32,
    pub reason_out: String,
}

#[derive(Debug, Clone, Default)]
pub struct SessionDef {
    pub num: i32,
    pub kind: String,
    pub name: String,
    pub laps: Option<i32>,
    pub time: Option<f64>,
    pub results: Vec<ResultPos>,
}

impl SessionDef {
    pub fn is_race(&self) -> bool {
        self.kind.eq_ignore_ascii_case("race")
    }
}

#[derive(Debug, Clone, Default)]
pub struct TireDef {
    pub index: i32,
    pub kind: String,
}

impl TireDef {
    pub fn is_wet(&self) -> bool {
        self.kind.to_ascii_lowercase().contains("wet") || self.kind.to_ascii_lowercase().contains("rain")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    pub track_id: i64,
    pub track_name: String,
    pub track_display_name: String,
    pub track_config: String,
    pub track_length_km: f32,
    pub track_city: String,
    pub track_country: String,
    pub series_id: i64,
    pub subsession_id: i64,
    pub league_id: i64,
    pub event_type: String,
    pub incident_limit: Option<i32>,
    pub sessions: Vec<SessionDef>,
    pub drivers: Vec<Driver>,
    pub player_car_idx: i32,
    pub pace_car_idx: i32,
    pub fuel_max_ltr: f32,
    pub max_fuel_pct: f32,
    pub est_lap_time: f32,
    pub shift_rpm: f32,
    pub redline_rpm: f32,
    pub tires: Vec<TireDef>,
    pub sectors: Vec<f32>,
}

impl SessionInfo {
    pub fn parse(src: &str) -> Self {
        Self::from_yaml(&yaml::parse(src))
    }

    pub fn from_yaml(y: &Yaml) -> Self {
        let wi = y.get("WeekendInfo");
        let di = y.get("DriverInfo");
        let unlimited = |v: &Yaml| v.str().map(|s| s.starts_with("unlimited")).unwrap_or(true);
        let sessions = y
            .path("SessionInfo.Sessions")
            .list()
            .iter()
            .map(|s| SessionDef {
                num: s.get("SessionNum").i32_or(0),
                kind: s.get("SessionType").string(),
                name: s.get("SessionName").string(),
                laps: (!unlimited(s.get("SessionLaps"))).then(|| s.get("SessionLaps").i32_or(0)),
                time: (!unlimited(s.get("SessionTime"))).then(|| s.get("SessionTime").f64().unwrap_or(0.0)),
                results: s
                    .get("ResultsPositions")
                    .list()
                    .iter()
                    .map(|r| ResultPos {
                        position: r.get("Position").i32_or(0),
                        class_position: r.get("ClassPosition").i32_or(-1) + 1,
                        car_idx: r.get("CarIdx").i32_or(0).max(0) as usize,
                        laps_complete: r.get("LapsComplete").i32_or(0),
                        fastest_time: r.get("FastestTime").f32_or(-1.0),
                        last_time: r.get("LastTime").f32_or(-1.0),
                        incidents: r.get("Incidents").i32_or(0),
                        reason_out: r.get("ReasonOutStr").string(),
                    })
                    .collect(),
            })
            .collect();
        let drivers = di
            .get("Drivers")
            .list()
            .iter()
            .map(|d| Driver {
                car_idx: d.get("CarIdx").i32_or(0).max(0) as usize,
                name: d.get("UserName").string(),
                abbrev: d.get("AbbrevName").string(),
                initials: d.get("Initials").string(),
                user_id: d.get("UserID").i64().unwrap_or(-1),
                team_id: d.get("TeamID").i64().unwrap_or(0),
                team_name: d.get("TeamName").string(),
                car_number: d.get("CarNumber").string(),
                car_id: d.get("CarID").i64().unwrap_or(0),
                car_path: d.get("CarPath").string(),
                car_name: d.get("CarScreenName").string(),
                car_short: d.get("CarScreenNameShort").string(),
                class_id: d.get("CarClassID").i64().unwrap_or(0),
                class_name: d.get("CarClassShortName").string(),
                class_color: d.get("CarClassColor").i64().unwrap_or(0xffffff) as u32,
                class_est_lap: d.get("CarClassEstLapTime").f32_or(0.0),
                irating: d.get("IRating").i32_or(0),
                lic_string: d.get("LicString").string(),
                lic_color: d.get("LicColor").i64().unwrap_or(0xffffff) as u32,
                club: d.get("ClubName").string(),
                is_spectator: d.get("IsSpectator").i32_or(0) != 0,
                is_pace_car: d.get("CarIsPaceCar").i32_or(0) != 0,
                is_ai: d.get("CarIsAI").i32_or(0) != 0,
                incidents: d.get("CurDriverIncidentCount").i32_or(0),
            })
            .collect();
        SessionInfo {
            track_id: wi.get("TrackID").i64().unwrap_or(0),
            track_name: wi.get("TrackName").string(),
            track_display_name: wi.get("TrackDisplayName").string(),
            track_config: wi.get("TrackConfigName").string(),
            track_length_km: wi.get("TrackLength").f32_or(0.0),
            track_city: wi.get("TrackCity").string(),
            track_country: wi.get("TrackCountry").string(),
            series_id: wi.get("SeriesID").i64().unwrap_or(0),
            subsession_id: wi.get("SubSessionID").i64().unwrap_or(0),
            league_id: wi.get("LeagueID").i64().unwrap_or(0),
            event_type: wi.get("EventType").string(),
            incident_limit: wi.path("WeekendOptions.IncidentLimit").i64().map(|v| v as i32),
            sessions,
            drivers,
            player_car_idx: di.get("DriverCarIdx").i32_or(-1),
            pace_car_idx: di.get("PaceCarIdx").i32_or(-1),
            fuel_max_ltr: di.get("DriverCarFuelMaxLtr").f32_or(0.0),
            max_fuel_pct: di.get("DriverCarMaxFuelPct").f32_or(1.0),
            est_lap_time: di.get("DriverCarEstLapTime").f32_or(0.0),
            shift_rpm: di.get("DriverCarSLShiftRPM").f32_or(0.0),
            redline_rpm: di.get("DriverCarRedLine").f32_or(0.0),
            tires: di
                .get("DriverTires")
                .list()
                .iter()
                .map(|t| TireDef { index: t.get("TireIndex").i32_or(0), kind: t.get("TireCompoundType").string() })
                .collect(),
            sectors: y.path("SplitTimeInfo.Sectors").list().iter().map(|s| s.get("SectorStartPct").f32_or(0.0)).collect(),
        }
    }

    pub fn session(&self, num: i32) -> Option<&SessionDef> {
        self.sessions.iter().find(|s| s.num == num)
    }

    pub fn driver(&self, car_idx: usize) -> Option<&Driver> {
        self.drivers.iter().find(|d| d.car_idx == car_idx)
    }

    pub fn fuel_capacity(&self) -> f32 {
        self.fuel_max_ltr * if self.max_fuel_pct > 0.0 { self.max_fuel_pct } else { 1.0 }
    }

    pub fn tire_is_wet(&self, compound: i32) -> Option<bool> {
        if self.tires.is_empty() {
            return None;
        }
        self.tires.iter().find(|t| t.index == compound).map(|t| t.is_wet())
    }

    pub fn has_wet_tires(&self) -> bool {
        self.tires.iter().any(|t| t.is_wet())
    }
}

pub fn wetness_label(w: i32) -> &'static str {
    match w {
        1 => "Dry",
        2 => "Mostly dry",
        3 => "Very lightly wet",
        4 => "Lightly wet",
        5 => "Moderately wet",
        6 => "Very wet",
        7 => "Extremely wet",
        _ => "Unknown",
    }
}

pub fn skies_label(s: i32) -> &'static str {
    match s {
        0 => "Clear",
        1 => "Partly cloudy",
        2 => "Mostly cloudy",
        3 => "Overcast",
        _ => "Unknown",
    }
}

pub fn session_state_label(s: i32) -> &'static str {
    match s {
        1 => "Get in car",
        2 => "Warmup",
        3 => "Parade laps",
        4 => "Racing",
        5 => "Checkered",
        6 => "Cool down",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_session_info() {
        let s = SessionInfo::parse(
            "WeekendInfo:\n TrackID: 9\n TrackDisplayName: Road Atlanta\n TrackLength: 4.08 km\n WeekendOptions:\n  IncidentLimit: unlimited\nSessionInfo:\n Sessions:\n - SessionNum: 0\n   SessionLaps: 20\n   SessionTime: unlimited\n   SessionType: Race\nDriverInfo:\n DriverCarIdx: 2\n DriverCarFuelMaxLtr: 100.0\n DriverCarMaxFuelPct: 0.5\n DriverTires:\n - TireIndex: 0\n   TireCompoundType: \"Hard\"\n - TireIndex: 1\n   TireCompoundType: \"Wet\"\n Drivers:\n - CarIdx: 2\n   UserName: Me\n   CarClassColor: 0xff0000\n",
        );
        assert_eq!(s.track_id, 9);
        assert_eq!(s.incident_limit, None);
        assert_eq!(s.sessions[0].laps, Some(20));
        assert_eq!(s.sessions[0].time, None);
        assert!(s.sessions[0].is_race());
        assert_eq!(s.fuel_capacity(), 50.0);
        assert_eq!(s.tire_is_wet(1), Some(true));
        assert_eq!(s.tire_is_wet(0), Some(false));
        assert_eq!(s.driver(2).unwrap().class_color, 0xff0000);
    }
}

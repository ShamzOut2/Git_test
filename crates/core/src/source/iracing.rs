//! Live iRacing source (Windows). Elsewhere it simply reports "disconnected".

use super::{Poll, Source};
use crate::model::{Telemetry, MAX_CARS};
use apex_irsdk::{Row, Var, VarTable};

macro_rules! vars {
    ($($field:ident = $name:literal),* $(,)?) => {
        /// Variable handles resolved once per connection.
        #[derive(Default, Clone, Copy)]
        #[allow(non_snake_case)]
        pub struct Vars { $(pub $field: Option<Var>),* }
        impl Vars {
            pub fn resolve(t: &VarTable) -> Self { Self { $($field: t.get($name)),* } }
        }
    };
}

vars! {
    session_time = "SessionTime", session_num = "SessionNum", session_state = "SessionState",
    session_flags = "SessionFlags", session_time_remain = "SessionTimeRemain",
    session_laps_remain = "SessionLapsRemainEx", time_of_day = "SessionTimeOfDay",
    player_idx = "PlayerCarIdx", cam_car_idx = "CamCarIdx", replay = "IsReplayPlaying",
    speed = "Speed", rpm = "RPM", gear = "Gear", throttle = "Throttle", brake = "Brake", clutch = "Clutch",
    steering = "SteeringWheelAngle", steering_max = "SteeringWheelAngleMax", abs_active = "BrakeABSactive",
    fuel_level = "FuelLevel", fuel_level_pct = "FuelLevelPct", fuel_use_per_hour = "FuelUsePerHour",
    lap = "Lap", lap_completed = "LapCompleted", lap_dist_pct = "LapDistPct", lap_dist = "LapDist",
    lap_current_time = "LapCurrentLapTime", lap_last_time = "LapLastLapTime", lap_best_time = "LapBestLapTime",
    delta_best = "LapDeltaToBestLap", delta_best_ok = "LapDeltaToBestLap_OK",
    on_pit_road = "OnPitRoad", is_on_track = "IsOnTrack", incidents = "PlayerCarMyIncidentCount",
    yaw_north = "YawNorth", lat_accel = "LatAccel", long_accel = "LongAccel", tire_compound = "PlayerTireCompound",
    track_temp = "TrackTempCrew", air_temp = "AirTemp", track_wetness = "TrackWetness", precipitation = "Precipitation",
    declared_wet = "WeatherDeclaredWet", wind_vel = "WindVel", wind_dir = "WindDir", humidity = "RelativeHumidity",
    fog = "FogLevel", skies = "Skies", air_pressure = "AirPressure", display_units = "DisplayUnits", car_left_right = "CarLeftRight",
    car_lap = "CarIdxLap", car_lap_completed = "CarIdxLapCompleted", car_pct = "CarIdxLapDistPct",
    car_est = "CarIdxEstTime", car_f2 = "CarIdxF2Time", car_pos = "CarIdxPosition", car_class_pos = "CarIdxClassPosition",
    car_pit = "CarIdxOnPitRoad", car_surface = "CarIdxTrackSurface", car_last = "CarIdxLastLapTime",
    car_best = "CarIdxBestLapTime", car_tire = "CarIdxTireCompound", car_gear = "CarIdxGear", car_rpm = "CarIdxRPM", car_flags = "CarIdxSessionFlags",
}

/// Copy one telemetry row into the normalised model.
pub fn fill(v: &Vars, r: Row, t: &mut Telemetry) {
    t.session_time = r.f64(v.session_time);
    t.session_num = r.i32(v.session_num);
    t.session_state = r.i32(v.session_state);
    t.session_flags = r.u32(v.session_flags);
    t.session_time_remain = r.f64(v.session_time_remain);
    t.session_laps_remain = r.i32(v.session_laps_remain);
    t.time_of_day = r.f32(v.time_of_day);
    t.player_idx = r.i32(v.player_idx);
    t.cam_car_idx = r.i32(v.cam_car_idx);
    t.is_replay_playing = r.bool(v.replay);
    t.speed = r.f32(v.speed);
    t.rpm = r.f32(v.rpm);
    t.gear = r.i32(v.gear);
    t.throttle = r.f32(v.throttle);
    t.brake = r.f32(v.brake);
    // The SDK reports clutch *engagement* (1 = pedal released); overlays want pedal travel.
    t.clutch = if v.clutch.is_some() { 1.0 - r.f32(v.clutch) } else { 0.0 };
    t.steering = r.f32(v.steering);
    t.steering_max = r.f32(v.steering_max);
    t.abs_active = r.bool(v.abs_active);
    t.fuel_level = r.f32(v.fuel_level);
    t.fuel_level_pct = r.f32(v.fuel_level_pct);
    t.fuel_use_per_hour = r.f32(v.fuel_use_per_hour);
    t.lap = r.i32(v.lap);
    t.lap_completed = r.i32(v.lap_completed);
    t.lap_dist_pct = r.f32(v.lap_dist_pct);
    t.lap_dist = r.f32(v.lap_dist);
    t.lap_current_time = r.f32(v.lap_current_time);
    t.lap_last_time = r.f32(v.lap_last_time);
    t.lap_best_time = r.f32(v.lap_best_time);
    t.delta_best = r.f32(v.delta_best);
    t.delta_best_ok = r.bool(v.delta_best_ok);
    t.on_pit_road = r.bool(v.on_pit_road);
    t.is_on_track = r.bool(v.is_on_track);
    t.incidents = r.i32(v.incidents);
    t.yaw_north = r.f32(v.yaw_north);
    t.lat_accel = r.f32(v.lat_accel);
    t.long_accel = r.f32(v.long_accel);
    t.tire_compound = r.i32(v.tire_compound);
    t.track_temp = r.f32(v.track_temp);
    t.air_temp = r.f32(v.air_temp);
    t.track_wetness = r.i32(v.track_wetness);
    t.precipitation = r.f32(v.precipitation);
    t.declared_wet = r.bool(v.declared_wet);
    t.wind_vel = r.f32(v.wind_vel);
    t.wind_dir = r.f32(v.wind_dir);
    t.humidity = r.f32(v.humidity);
    t.fog = r.f32(v.fog);
    t.skies = r.i32(v.skies);
    t.air_pressure = r.f32(v.air_pressure);
    t.display_metric = v.display_units.is_none() || r.i32(v.display_units) == 1;
    t.car_left_right = r.i32(v.car_left_right);
    for (i, c) in t.cars.iter_mut().enumerate().take(MAX_CARS) {
        c.lap = r.i32_at(v.car_lap, i);
        c.lap_completed = r.i32_at(v.car_lap_completed, i);
        c.pct = r.f32_at(v.car_pct, i);
        c.est_time = r.f32_at(v.car_est, i);
        c.f2_time = r.f32_at(v.car_f2, i);
        c.position = r.i32_at(v.car_pos, i);
        c.class_position = r.i32_at(v.car_class_pos, i);
        c.on_pit_road = r.bool_at(v.car_pit, i);
        c.surface = if v.car_surface.is_some() { r.i32_at(v.car_surface, i) } else { -1 };
        c.last_lap = r.f32_at(v.car_last, i);
        c.best_lap = r.f32_at(v.car_best, i);
        c.tire_compound = r.i32_at(v.car_tire, i);
        c.gear = r.i32_at(v.car_gear, i);
        c.rpm = r.f32_at(v.car_rpm, i);
        c.session_flags = r.i32_at(v.car_flags, i) as u32;
    }
}

pub struct IRacing {
    #[cfg(windows)]
    shm: Option<apex_irsdk::SharedMemory>,
    #[cfg(windows)]
    vars: Vars,
    #[cfg(windows)]
    row: Vec<u8>,
    #[cfg(windows)]
    last_tick: i32,
    #[cfg(windows)]
    last_session_update: i32,
    #[cfg(windows)]
    layout: (i32, i32),
}

impl Default for IRacing {
    fn default() -> Self {
        Self::new()
    }
}

impl IRacing {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            shm: None,
            #[cfg(windows)]
            vars: Vars::default(),
            #[cfg(windows)]
            row: Vec::new(),
            #[cfg(windows)]
            last_tick: -1,
            #[cfg(windows)]
            last_session_update: -1,
            #[cfg(windows)]
            layout: (-1, -1),
        }
    }
}

impl Source for IRacing {
    fn name(&self) -> &'static str {
        "iracing"
    }

    #[cfg(not(windows))]
    fn poll(&mut self, _tel: &mut Telemetry) -> Poll {
        std::thread::sleep(std::time::Duration::from_millis(500));
        Poll::Disconnected
    }

    #[cfg(windows)]
    fn poll(&mut self, tel: &mut Telemetry) -> Poll {
        use apex_irsdk::SharedMemory;
        if self.shm.is_none() {
            self.shm = SharedMemory::open();
            self.last_session_update = -1;
            self.last_tick = -1;
            self.layout = (-1, -1);
            if self.shm.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(500));
                return Poll::Disconnected;
            }
        }
        let shm = self.shm.as_mut().unwrap();
        let got_signal = shm.wait(50);
        let h = shm.header();
        if !h.is_connected() {
            self.last_session_update = -1;
            if !got_signal {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            return Poll::Disconnected;
        }
        if (h.num_vars, h.var_header_offset) != self.layout {
            self.layout = (h.num_vars, h.var_header_offset);
            self.vars = Vars::resolve(shm.vars(&h));
        }
        let Some(tick) = shm.read_row(&mut self.row) else { return Poll::Idle };
        if tick == self.last_tick {
            return Poll::Idle;
        }
        self.last_tick = tick;
        tel.tick = tick;
        fill(&self.vars, Row(&self.row), tel);
        let session_yaml = (h.session_info_update != self.last_session_update).then(|| {
            self.last_session_update = h.session_info_update;
            shm.session_info(&h)
        });
        Poll::Data { session_yaml }
    }
}

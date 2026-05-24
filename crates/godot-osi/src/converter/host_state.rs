//! `OsiHostVehicleState` — a convenience helper for meter-cluster / ADAS HMIs.
//!
//! The converter emits the raw typed snapshots (`OsiHostVehicleData`,
//! `OsiGroundTruth`), but reading e.g. the speed means digging through
//! `vehicle_motion.velocity.{x,y,z}`. This helper binds to the converter, caches
//! the latest of each, and exposes the common ego-vehicle quantities a dashboard
//! needs in a single `ego_state()` `Dictionary` (plus a few scalar getters).
//!
//! Source of each field:
//!   - speed / gear / rpm / pedals / steering / curvature / pose come from
//!     `OsiHostVehicleData` (the host channel);
//!   - turn-signal / brake-light / head-light / assigned-lane come from the ego's
//!     own object in `GroundTruth` (matched via `host_vehicle_id`), and are only
//!     present if the simulator populates them.
//!
//! Missing values degrade gracefully: floats read back `NAN`, light/lane ints
//! `-1`. (The ADAS-function-state list, `vehicle_automated_driving_function`, is
//! intentionally not surfaced here — read it off the snapshot if your simulator
//! provides it.)

use godot::prelude::*;
use osi_types::osi3;

use crate::converter::coords::{self, AxisMapping};
use crate::converter::generated::{OsiGroundTruth, OsiHostVehicleData, OsiVector3d};
use crate::converter::node::OsiConverter;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct OsiHostVehicleState {
    /// Latest converted host-vehicle snapshot, if any.
    host: Option<Gd<OsiHostVehicleData>>,
    /// Latest converted ground-truth snapshot (for ego light/lane state), if any.
    ground_truth: Option<Gd<OsiGroundTruth>>,
    /// Uniform scale forwarded to the coordinate mapping for `position`.
    #[var]
    #[init(val = 1.0)]
    scale: real,
    base: Base<Node>,
}

#[godot_api]
impl OsiHostVehicleState {
    /// Emitted after a new host-vehicle snapshot is cached (i.e. fresh ego state).
    #[signal]
    fn updated();

    /// Connect to `converter`'s snapshot signals so the cached state updates
    /// automatically every converted frame.
    #[func]
    fn bind_converter(&mut self, mut converter: Gd<OsiConverter>) {
        let this = self.to_gd();
        converter.connect(
            "host_vehicle_data_converted",
            &Callable::from_object_method(&this, "on_host_vehicle_data"),
        );
        converter.connect(
            "ground_truth_converted",
            &Callable::from_object_method(&this, "on_ground_truth"),
        );
    }

    /// Cache a new host-vehicle snapshot. (Auto-called once bound.)
    #[func]
    pub fn on_host_vehicle_data(&mut self, snapshot: Gd<OsiHostVehicleData>) {
        self.host = Some(snapshot);
        self.signals().updated().emit();
    }

    /// Cache a new ground-truth snapshot (for ego light/lane state). (Auto-called.)
    #[func]
    pub fn on_ground_truth(&mut self, snapshot: Gd<OsiGroundTruth>) {
        self.ground_truth = Some(snapshot);
    }

    /// Whether a host-vehicle snapshot has been received.
    #[func]
    fn is_ready(&self) -> bool {
        self.host.is_some()
    }

    /// Ego forward speed magnitude in km/h (`NAN` if unavailable).
    #[func]
    fn speed_kph(&self) -> f64 {
        self.speed_mps() * 3.6
    }

    /// Ego speed magnitude in m/s (`NAN` if unavailable).
    #[func]
    fn speed_mps(&self) -> f64 {
        let Some(host) = self.host.as_ref() else {
            return f64::NAN;
        };
        let h = host.bind();
        h.vehicle_motion
            .as_ref()
            .and_then(|m| m.bind().velocity.as_ref().map(vec_len))
            .unwrap_or(f64::NAN)
    }

    /// Transmission gear: >0 forward, 0 neutral, <0 reverse; `0` if no powertrain.
    #[func]
    fn gear(&self) -> i64 {
        let Some(host) = self.host.as_ref() else {
            return 0;
        };
        host.bind()
            .vehicle_powertrain
            .as_ref()
            .map(|p| p.bind().gear_transmission as i64)
            .unwrap_or(0)
    }

    /// All common ego-vehicle quantities in one `Dictionary`. Keys:
    /// `valid` (bool), `speed_mps`, `speed_kph`, `gear`, `rpm`, `throttle`,
    /// `brake`, `clutch`, `steering_angle`, `curvature` (all `NAN`/`0` when
    /// absent), `position` (Vector3, Godot space), `heading` (raw OSI yaw, rad),
    /// and — when the ego object is present in GroundTruth — `indicator`,
    /// `brake_light`, `head_light`, `assigned_lane_id` (`-1` if absent).
    #[func]
    fn ego_state(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(host) = self.host.as_ref() else {
            d.set("valid", false);
            return d;
        };
        let h = host.bind();
        let mapping = AxisMapping {
            scale: self.scale,
            ..Default::default()
        };

        d.set("valid", true);

        // --- vehicle_motion: speed, curvature, pose ---
        let mut speed = f64::NAN;
        let mut curvature = f64::NAN;
        let mut position = Vector3::ZERO;
        let mut has_position = false;
        let mut heading = f64::NAN;
        if let Some(motion) = h.vehicle_motion.as_ref() {
            let m = motion.bind();
            if let Some(v) = m.velocity.as_ref() {
                speed = vec_len(v);
            }
            curvature = m.current_curvature;
            if let Some(p) = m.position.as_ref() {
                position = godot_pos(p, &mapping);
                has_position = true;
            }
            if let Some(o) = m.orientation.as_ref() {
                heading = o.bind().yaw;
            }
        }
        // Fall back to the localization estimate for pose if motion lacked it.
        if let Some(loc) = h.vehicle_localization.as_ref() {
            let l = loc.bind();
            if !has_position {
                if let Some(p) = l.position.as_ref() {
                    position = godot_pos(p, &mapping);
                    has_position = true;
                }
            }
            if heading.is_nan() {
                if let Some(o) = l.orientation.as_ref() {
                    heading = o.bind().yaw;
                }
            }
        }

        d.set("speed_mps", speed);
        d.set("speed_kph", if speed.is_nan() { f64::NAN } else { speed * 3.6 });
        d.set("curvature", curvature);
        d.set("position", position);
        d.set("has_position", has_position);
        d.set("heading", heading);

        // --- powertrain: gear, rpm, throttle/clutch ---
        let mut gear = 0_i64;
        let mut rpm = f64::NAN;
        let mut throttle = f64::NAN;
        let mut clutch = f64::NAN;
        if let Some(pt) = h.vehicle_powertrain.as_ref() {
            let pt = pt.bind();
            gear = pt.gear_transmission as i64;
            throttle = pt.pedal_position_acceleration;
            clutch = pt.pedal_position_clutch;
            if let Some(motor) = pt.motor.iter_shared().next() {
                rpm = motor.bind().rpm;
            }
        }
        d.set("gear", gear);
        d.set("rpm", rpm);
        d.set("throttle", throttle);
        d.set("clutch", clutch);

        // --- brake pedal ---
        let brake = h
            .vehicle_brake_system
            .as_ref()
            .map(|b| b.bind().pedal_position_brake)
            .unwrap_or(f64::NAN);
        d.set("brake", brake);

        // --- steering wheel angle ---
        let steering = h
            .vehicle_steering
            .as_ref()
            .and_then(|s| s.bind().vehicle_steering_wheel.as_ref().map(|w| w.bind().angle))
            .unwrap_or(f64::NAN);
        d.set("steering_angle", steering);

        // --- ego light / lane state from GroundTruth (optional) ---
        let host_id = h.host_vehicle_id.as_ref().map(|i| i.bind().value);
        let (mut indicator, mut brake_light, mut head_light, mut lane_id) = (-1_i64, -1, -1, -1);
        if let (Some(hid), Some(gt)) = (host_id, self.ground_truth.as_ref()) {
            let gt = gt.bind();
            for mo in gt.moving_object.iter_shared() {
                let mo = mo.bind();
                if mo.id.as_ref().map(|i| i.bind().value) != Some(hid) {
                    continue;
                }
                if let Some(vc) = mo.vehicle_classification.as_ref() {
                    if let Some(ls) = vc.bind().light_state.as_ref() {
                        let ls = ls.bind();
                        indicator = ls.indicator_state;
                        brake_light = ls.brake_light_state;
                        head_light = ls.head_light;
                    }
                }
                if let Some(lane) = mo.assigned_lane_id.iter_shared().next() {
                    lane_id = lane.bind().value;
                }
                break;
            }
        }
        d.set("indicator", indicator);
        d.set("brake_light", brake_light);
        d.set("head_light", head_light);
        d.set("assigned_lane_id", lane_id);

        d
    }
}

/// Magnitude of an OSI vector resource (frame-independent, for speed).
fn vec_len(v: &Gd<OsiVector3d>) -> f64 {
    let v = v.bind();
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

/// Convert an OSI position resource into Godot space via the shared mapping.
fn godot_pos(v: &Gd<OsiVector3d>, mapping: &AxisMapping) -> Vector3 {
    let v = v.bind();
    coords::osi_position_to_godot(
        &osi3::Vector3d {
            x: Some(v.x),
            y: Some(v.y),
            z: Some(v.z),
        },
        mapping,
    )
}

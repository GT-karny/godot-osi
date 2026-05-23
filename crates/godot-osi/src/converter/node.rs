//! `OsiConverter` — the converter plugin's core node (REQUIREMENTS.md §5, "B").
//!
//! Each frame it drains the newest raw OSI frame from the shared
//! [`OsiFrameBus`], converts it to a typed Godot `Resource` snapshot via the
//! generated `convert_*` free functions, and emits it as a signal. It does NOT
//! spawn or mutate any other scene nodes — turning a snapshot into scene
//! objects is the job of the optional helper (see `spawn_helper`).
//!
//! Frame policy is "newest-wins", enforced by the bus: a slow consumer simply
//! sees the latest frame and silently drops intermediate ones.

use godot::classes::INode;
use godot::prelude::*;

use crate::converter::generated::{
    convert_ground_truth, convert_host_vehicle_data, OsiGroundTruth, OsiHostVehicleData,
};
use crate::frame_bus::OsiFrameBus;

#[derive(GodotClass)]
#[class(base=Node, init)]
pub struct OsiConverter {
    /// Shared producer/consumer bus. Empty until wired to a source.
    bus: OsiFrameBus,
    /// When false, `process` does not drain the bus (manual `poll` only).
    #[var]
    #[init(val = true)]
    auto_poll: bool,
    base: Base<Node>,
}

#[godot_api]
impl OsiConverter {
    /// Emitted once per converted GroundTruth frame, carrying the typed snapshot.
    #[signal]
    fn ground_truth_converted(snapshot: Gd<OsiGroundTruth>);

    /// Emitted once per converted HostVehicleData frame.
    #[signal]
    fn host_vehicle_data_converted(snapshot: Gd<OsiHostVehicleData>);

    /// Drain whatever is currently on the bus and emit conversions. Safe to
    /// call manually (e.g. from a fixed-step loop) regardless of `auto_poll`.
    #[func]
    pub fn poll(&mut self) {
        if let Some(gt) = self.bus.ground_truth.take() {
            let snapshot = convert_ground_truth(&gt);
            self.signals().ground_truth_converted().emit(&snapshot);
        }
        if let Some(hvd) = self.bus.host_vehicle_data.take() {
            let snapshot = convert_host_vehicle_data(&hvd);
            self.signals().host_vehicle_data_converted().emit(&snapshot);
        }
    }
}

#[godot_api]
impl INode for OsiConverter {
    fn process(&mut self, _delta: f64) {
        if self.auto_poll {
            self.poll();
        }
    }
}

impl OsiConverter {
    /// Share `bus` with this converter (Arc-cloned slots). Used by integration
    /// wiring — the receiver hands over a clone of its own bus so producer and
    /// consumer talk through one instance (see docs/ARCHITECTURE.md).
    pub fn set_bus(&mut self, bus: OsiFrameBus) {
        self.bus = bus;
    }

    /// A clone of the bus (shares the same underlying slots), so a producer can
    /// feed frames the converter will drain.
    pub fn bus(&self) -> OsiFrameBus {
        self.bus.clone()
    }
}

//! OSI converter plugin — raw OSI -> Godot typed Resources.
//! (owner: `feature/converter` session)
//!
//! Layers (REQUIREMENTS.md §3, §5):
//! - [`generated`]: build-time generated typed Godot `Resource` classes mirroring
//!   the OSI proto 1:1 (raw values, "A" geometry policy). Each message gets an
//!   `OsiX` class plus a pure `convert_x(&osi3::X) -> Gd<OsiX>` free function.
//! - [`coords`]: hand-written, configurable coordinate conversion OSI
//!   (right-handed, Z-up) -> Godot (left-handed, Y-up). Engine-free, unit-tested.
//! - `node` (TODO): the `OsiConverter` Godot node, core "B": drain the newest
//!   frame from [`crate::frame_bus::OsiFrameBus`], convert, emit via signal.
//! - `spawn_helper` (TODO): optional helper "A", id-tracked Node3D spawn/update/free.
//!
//! Boundary:
//! - input:  `osi_types::osi3::GroundTruth` / `HostVehicleData` (prost types)
//! - output: typed Godot `Resource` snapshots ([`generated`] classes)

#![allow(dead_code)]

pub mod coords;
pub mod generated;
pub mod host_state;
pub mod node;
pub mod spawn_helper;
pub mod visualizer;

#[cfg(feature = "itest")]
pub mod testrig;

#[cfg(test)]
mod trace_tests;

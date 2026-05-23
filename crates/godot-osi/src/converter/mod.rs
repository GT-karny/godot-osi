//! OSI converter plugin — raw OSI -> Godot typed Resources.
//! (owner: `feature/converter` session)
//!
//! Responsibilities (REQUIREMENTS.md §5):
//! - a build-time code generator producing typed Godot `Resource` classes from
//!   the OSI proto definitions (full set, proto field names/hierarchy preserved)
//! - coordinate conversion OSI (right-handed, Z-up) -> Godot (left-handed,
//!   Y-up); see REQUIREMENTS.md §3; mapping must be configurable
//! - a Godot class `OsiConverter` (core "B"): drain the newest frame from
//!   [`crate::frame_bus::OsiFrameBus`], convert to typed Resources, emit via
//!   signal. It does NOT touch scene nodes.
//! - an optional helper/sample (option "A"): id-tracked Node3D spawn/update/free
//!
//! Boundary:
//! - input:  `osi_types::osi3::GroundTruth` / `HostVehicleData` (prost types),
//!           obtained from [`crate::frame_bus::OsiFrameBus`]
//! - output: typed Godot `Resource` snapshots (this crate's generated classes)
//!
//! Keep the pure conversion logic as free functions (e.g.
//! `convert_ground_truth(&osi3::GroundTruth) -> ...`) separate from the Godot
//! class so it is unit-testable with hand-built synthetic messages — no
//! receiver or running Godot required.

#![allow(dead_code)]

// TODO(converter session): add the build.rs code generator, the generated
// Resource classes, the coordinate-conversion module, and the `OsiConverter`
// GodotClass here.

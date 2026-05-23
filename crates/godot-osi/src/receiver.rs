//! OSI receiver plugin — gRPC client. (owner: `feature/receiver` session)
//!
//! Responsibilities (REQUIREMENTS.md §4):
//! - tonic gRPC client for `StreamGroundTruth` / `StreamHostVehicleData`
//! - a background thread running a tokio runtime; received frames are pushed
//!   newest-wins into [`crate::frame_bus::OsiFrameBus`]
//! - a Godot class `OsiReceiver` exposing: server address/port/TLS config,
//!   connect/disconnect, reconnect handling, and connection/frame signals
//!
//! Boundary:
//! - input:  none (talks to the gRPC server, or the bundled mock server)
//! - output: [`crate::frame_bus::OsiFrameBus`] (raw prost OSI frames)
//!
//! The gRPC client stubs live in `osi_types::osi::server` and the OSI messages
//! in `osi_types::osi3`. Runtime deps already wired in Cargo.toml: `tokio`,
//! `tonic`.

#![allow(dead_code)]

// TODO(receiver session): implement the `OsiReceiver` GodotClass and the
// background gRPC streaming loop here, writing into `OsiFrameBus`.

//! ASAM OSI v3.7.0 protobuf types and gRPC service stubs.
//!
//! All Rust types here are generated at build time from the `.proto` files
//! under `proto/` (see `build.rs`). This crate is transport/engine agnostic:
//! it knows nothing about Godot.

/// OSI core messages (`package osi3`): `GroundTruth`, `HostVehicleData`,
/// `MovingObject`, `BaseMoving`, etc.
pub mod osi3 {
    tonic::include_proto!("osi3");
}

/// gRPC service definitions (`package osi.server`):
/// `GroundTruthService` and `HostVehicleDataService` clients/servers.
///
/// The module nesting must mirror the proto package path (`osi.server`) so the
/// `super::` cross-package references that prost generates resolve correctly.
pub mod osi {
    pub mod server {
        tonic::include_proto!("osi.server");
    }
}

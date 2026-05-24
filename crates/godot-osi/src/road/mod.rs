//! OpenDRIVE road-network support.
//!
//! Mirrors the converter's "Godot-free core + thin Godot layer" split:
//!   - [`esmini_rm`] (separate crate) parses `.xodr` and answers geometry queries,
//!   - [`network`] exposes it as an `OsiRoadNetwork` `Resource`,
//!   - [`mesh`] turns sampled road geometry into a Godot `ArrayMesh`,
//!   - [`visualizer`] is an optional `Node3D` demo helper that renders it.
//!
//! Road vertices are converted to Godot space with the same
//! [`crate::converter::coords::AxisMapping`] used for OSI moving objects, so the
//! road and the OSI boxes share one coordinate frame.

pub mod mesh;
pub mod network;
pub mod visualizer;

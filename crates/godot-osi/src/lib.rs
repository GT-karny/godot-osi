//! Godot 4 GDExtension entry point for the OSI wrapper.
//!
//! Two plugins ship in this single extension (see REQUIREMENTS.md):
//!   1. [`receiver`]  — the OSI receiver (gRPC client, background thread),
//!   2. [`converter`] — the OSI -> Godot converter (typed Resources + coords).
//!
//! They meet only at [`frame_bus`], the raw-OSI-frame boundary, and are
//! otherwise developed independently (see docs/ARCHITECTURE.md).

use godot::prelude::*;

pub mod frame_bus;

mod converter;
mod receiver;

struct GodotOsiExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotOsiExtension {}

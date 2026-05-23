//! Godot 4 GDExtension entry point for the OSI wrapper.
//!
//! This crate hosts the two plugins described in REQUIREMENTS.md:
//!   1. the OSI receiver (gRPC client, background thread),
//!   2. the OSI -> Godot converter (typed Resources + coordinate conversion).
//!
//! For now this is a minimal entry point that registers the extension so the
//! build/load pipeline can be validated. Plugin classes are added next.

use godot::prelude::*;

struct GodotOsiExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotOsiExtension {}

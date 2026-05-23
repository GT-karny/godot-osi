//! Events sent from the background gRPC thread to the Godot main thread.
//!
//! The background tokio runtime must never touch Godot objects (engine APIs are
//! main-thread only). Instead it pushes plain values down an `std::sync::mpsc`
//! channel; `OsiReceiver::process` drains them and emits the matching signals on
//! the main thread. See `crate::receiver` and docs/ARCHITECTURE.md.

/// Which OSI stream a frame belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    GroundTruth,
    HostVehicleData,
}

/// Connection-state values mirrored to the `OsiReceiver.STATE_*` constants.
///
/// Kept as plain `i64` so the value crosses the thread boundary and reaches
/// GDScript unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Reconnecting = 3,
    Error = 4,
}

impl State {
    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

/// A message destined for the Godot main thread.
#[derive(Clone, Debug)]
pub enum Event {
    /// Connection state changed; carries the new [`State`].
    State(State),
    /// A new frame for `stream` was just stored in the bus (newest-wins).
    Frame(Stream),
    /// A non-fatal stream/connection error worth surfacing to GDScript.
    Error(String),
}

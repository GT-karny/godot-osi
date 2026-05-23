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
//!
//! Submodules (all owned by this session):
//! - [`event`]       — thread → main-thread message enum
//! - [`grpc`]        — Godot-free streaming/reconnect loops (unit-tested)
//! - [`mock_server`] — bundled mock gRPC server + `OsiMockServer` (§6)
//! - [`trace`]       — `.osi` trace record/replay (§6)

#![allow(dead_code)]

mod event;
mod grpc;
mod mock_server;
mod trace;

use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use godot::classes::{INode, Node};
use godot::prelude::*;
use tokio::sync::watch;

use crate::frame_bus::OsiFrameBus;
use event::{Event, State, Stream};
use grpc::{ConnConfig, RecordSlot};
use trace::TraceWriter;

/// Godot node that streams OSI frames from a gRPC server into an
/// [`OsiFrameBus`]. The actual networking runs on a background thread + tokio
/// runtime; this node only configures it and surfaces events as Godot signals
/// (drained on the main thread in [`OsiReceiver::process`]).
#[derive(GodotClass)]
#[class(base=Node)]
pub struct OsiReceiver {
    /// Server host (default `127.0.0.1`).
    #[var]
    address: GString,
    /// Server port (default `50051`).
    #[var]
    port: i64,
    /// Use TLS (`https`) instead of plaintext (`http`). Default `false`.
    #[var]
    use_tls: bool,
    /// Automatically reconnect when a stream drops. Default `true`.
    #[var]
    reconnect: bool,
    /// Delay between reconnect attempts, in milliseconds. Default `1000`.
    #[var]
    reconnect_delay_ms: i64,

    bus: OsiFrameBus,
    events: Option<Receiver<Event>>,
    thread: Option<JoinHandle<()>>,
    shutdown: Option<watch::Sender<bool>>,
    /// Last connection state emitted, to suppress duplicate signals.
    last_state: i64,
    rec_gt: RecordSlot,
    rec_hvd: RecordSlot,
    base: Base<Node>,
}

#[godot_api]
impl INode for OsiReceiver {
    fn init(base: Base<Node>) -> Self {
        Self {
            address: "127.0.0.1".into(),
            port: 50051,
            use_tls: false,
            reconnect: true,
            reconnect_delay_ms: 1000,
            bus: OsiFrameBus::new(),
            events: None,
            thread: None,
            shutdown: None,
            last_state: State::Disconnected.as_i64(),
            rec_gt: RecordSlot::default(),
            rec_hvd: RecordSlot::default(),
            base,
        }
    }

    fn process(&mut self, _delta: f64) {
        self.pump_events();
    }

    fn exit_tree(&mut self) {
        self.disconnect_from_server();
        self.stop_recording();
    }
}

#[godot_api]
impl OsiReceiver {
    #[constant]
    const STATE_DISCONNECTED: i64 = 0;
    #[constant]
    const STATE_CONNECTING: i64 = 1;
    #[constant]
    const STATE_CONNECTED: i64 = 2;
    #[constant]
    const STATE_RECONNECTING: i64 = 3;
    #[constant]
    const STATE_ERROR: i64 = 4;

    /// Emitted (on the main thread) when the connection state changes; carries
    /// one of the `STATE_*` constants.
    #[signal]
    fn connection_state_changed(state: i64);
    /// Emitted when a new GroundTruth frame was stored in the bus.
    #[signal]
    fn ground_truth_received();
    /// Emitted when a new HostVehicleData frame was stored in the bus.
    #[signal]
    fn host_vehicle_data_received();
    /// Emitted on a non-fatal stream/connection error.
    #[signal]
    fn stream_error(message: GString);

    /// Start streaming. Spawns the background thread; restarts if already running.
    #[func]
    fn connect_to_server(&mut self) {
        self.disconnect_from_server();

        let cfg = ConnConfig {
            address: self.address.to_string(),
            port: self.port as u16,
            use_tls: self.use_tls,
            reconnect: self.reconnect,
            reconnect_delay_ms: self.reconnect_delay_ms.max(0) as u64,
        };

        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let bus = self.bus.clone();
        let rec_gt = self.rec_gt.clone();
        let rec_hvd = self.rec_hvd.clone();

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("build receiver tokio runtime");
            rt.block_on(grpc::run(cfg, bus, event_tx, shutdown_rx, rec_gt, rec_hvd));
        });

        self.events = Some(event_rx);
        self.thread = Some(handle);
        self.shutdown = Some(shutdown_tx);
    }

    /// Stop streaming and join the background thread.
    #[func]
    fn disconnect_from_server(&mut self) {
        let was_running = self.shutdown.is_some();
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        if was_running {
            // Drain any final events (including the closing Disconnected state).
            self.pump_events();
        }
        self.events = None;
    }

    /// Whether a background streaming thread is currently running.
    #[func]
    fn is_streaming(&self) -> bool {
        self.thread.is_some()
    }

    /// Record received frames to `.osi` trace files (REQUIREMENTS.md §6). Pass
    /// an empty string for a stream to skip recording it. Recording continues
    /// until [`Self::stop_recording`]; it can be toggled while connected.
    #[func]
    fn start_recording(&mut self, ground_truth_path: GString, host_vehicle_path: GString) {
        let gt = ground_truth_path.to_string();
        if !gt.is_empty() {
            match TraceWriter::create(&gt) {
                Ok(writer) => *self.rec_gt.lock().expect("record slot poisoned") = Some(writer),
                Err(e) => godot_error!("OsiReceiver: cannot record GroundTruth to {gt}: {e}"),
            }
        }
        let hvd = host_vehicle_path.to_string();
        if !hvd.is_empty() {
            match TraceWriter::create(&hvd) {
                Ok(writer) => *self.rec_hvd.lock().expect("record slot poisoned") = Some(writer),
                Err(e) => godot_error!("OsiReceiver: cannot record HostVehicleData to {hvd}: {e}"),
            }
        }
    }

    /// Stop recording and flush both trace files.
    #[func]
    fn stop_recording(&mut self) {
        if let Some(mut writer) = self.rec_gt.lock().expect("record slot poisoned").take() {
            let _ = writer.flush();
        }
        if let Some(mut writer) = self.rec_hvd.lock().expect("record slot poisoned").take() {
            let _ = writer.flush();
        }
    }

    /// Integration hook (not exposed to GDScript): hand the converter an
    /// `Arc`-shared clone of this receiver's bus so both share one instance.
    /// See docs/ARCHITECTURE.md "統合時の配線".
    pub fn frame_bus(&self) -> OsiFrameBus {
        self.bus.clone()
    }

    /// Drain queued background events and emit the matching Godot signals.
    /// Must run on the main thread.
    fn pump_events(&mut self) {
        let mut drained = Vec::new();
        if let Some(rx) = self.events.as_ref() {
            while let Ok(ev) = rx.try_recv() {
                drained.push(ev);
            }
        }
        for ev in drained {
            match ev {
                Event::State(state) => self.set_state(state),
                Event::Frame(Stream::GroundTruth) => {
                    self.signals().ground_truth_received().emit()
                }
                Event::Frame(Stream::HostVehicleData) => {
                    self.signals().host_vehicle_data_received().emit()
                }
                Event::Error(msg) => self.signals().stream_error().emit(&GString::from(&msg)),
            }
        }
    }

    fn set_state(&mut self, state: State) {
        let value = state.as_i64();
        if value != self.last_state {
            self.last_state = value;
            self.signals().connection_state_changed().emit(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::grpc::{self, ConnConfig, RecordSlot};
    use super::mock_server::{self, MockConfig};
    use crate::frame_bus::OsiFrameBus;
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::sync::watch;

    /// Pick an OS-assigned free TCP port so concurrent test runs don't collide.
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn mock_server_feeds_frame_bus() {
        let port = free_port();

        // Start the bundled mock server (synthetic frames, fast cadence).
        let (server, server_shutdown) = mock_server::spawn(MockConfig {
            address: "127.0.0.1".to_string(),
            port,
            period: Duration::from_millis(5),
            ground_truth: Vec::new(),
            host_vehicle_data: Vec::new(),
        });

        // Run the receiver's gRPC loops against it on a dedicated runtime.
        let bus = OsiFrameBus::new();
        let (tx, _rx) = mpsc::channel();
        let (recv_shutdown_tx, recv_shutdown_rx) = watch::channel(false);
        let cfg = ConnConfig {
            address: "127.0.0.1".to_string(),
            port,
            use_tls: false,
            reconnect: true,
            reconnect_delay_ms: 50,
        };

        let bus_for_thread = bus.clone();
        let receiver = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(grpc::run(
                cfg,
                bus_for_thread,
                tx,
                recv_shutdown_rx,
                RecordSlot::default(),
                RecordSlot::default(),
            ));
        });

        // Wait (bounded) for both streams to land a frame in the bus.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if bus.ground_truth.has_frame() && bus.host_vehicle_data.has_frame() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no frames reached the bus before the deadline"
            );
            std::thread::sleep(Duration::from_millis(20));
        }

        let gt = bus.ground_truth.take().expect("ground truth frame");
        assert_eq!(gt.host_vehicle_id, Some(osi_types::osi3::Identifier { value: Some(1) }));

        // Tear down both threads.
        let _ = recv_shutdown_tx.send(true);
        let _ = receiver.join();
        let _ = server_shutdown.send(true);
        let _ = server.join();
    }
}

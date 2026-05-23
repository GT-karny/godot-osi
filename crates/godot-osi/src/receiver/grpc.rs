//! Godot-free gRPC streaming logic for the receiver.
//!
//! Two independent subscriptions (REQUIREMENTS.md §1): `StreamGroundTruth` and
//! `StreamHostVehicleData`. Each runs its own connect → subscribe → consume →
//! reconnect loop. Received frames are stored newest-wins into
//! [`OsiFrameBus`] and, while recording, appended to a `.osi` trace. A
//! lightweight [`Event`] is sent over an `mpsc` channel so the Godot main
//! thread can emit signals (see `crate::receiver`).
//!
//! Everything here is plain async/Rust with no Godot dependency, so it is unit
//! testable against the bundled mock server (`super::mock_server`).

use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::time::Duration;

use osi_types::osi::server::ground_truth_service_client::GroundTruthServiceClient;
use osi_types::osi::server::host_vehicle_data_service_client::HostVehicleDataServiceClient;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::Request;

use crate::frame_bus::OsiFrameBus;

use super::event::{Event, State, Stream};
use super::trace::TraceWriter;

/// A shared, runtime-toggleable trace recorder for one stream.
///
/// `None` = not recording. The Godot main thread swaps the writer in/out via
/// `start_recording`/`stop_recording`; the streaming loop writes to it under
/// the lock (never held across an `.await`).
pub type RecordSlot = Arc<Mutex<Option<TraceWriter>>>;

/// Connection settings, snapshotted from the `OsiReceiver` properties at
/// `connect()` time so the background thread owns an independent copy.
#[derive(Clone, Debug)]
pub struct ConnConfig {
    pub address: String,
    pub port: u16,
    pub use_tls: bool,
    pub reconnect: bool,
    pub reconnect_delay_ms: u64,
}

impl Default for ConnConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_string(),
            port: 50051,
            use_tls: false,
            reconnect: true,
            reconnect_delay_ms: 1000,
        }
    }
}

type Shutdown = tokio::sync::watch::Receiver<bool>;
type BoxError = Box<dyn Error + Send + Sync>;

/// Run both stream loops until `shutdown` flips to `true`.
pub async fn run(
    cfg: ConnConfig,
    bus: OsiFrameBus,
    tx: Sender<Event>,
    mut shutdown: Shutdown,
    rec_gt: RecordSlot,
    rec_hvd: RecordSlot,
) {
    let gt = tokio::spawn(ground_truth_loop(
        cfg.clone(),
        bus.clone(),
        tx.clone(),
        shutdown.clone(),
        rec_gt,
    ));
    let hvd = tokio::spawn(host_vehicle_loop(
        cfg,
        bus,
        tx.clone(),
        shutdown.clone(),
        rec_hvd,
    ));

    // Block until disconnect() requests shutdown, then stop both loops.
    let _ = shutdown.wait_for(|stop| *stop).await;
    gt.abort();
    hvd.abort();
    let _ = gt.await;
    let _ = hvd.await;
    let _ = tx.send(Event::State(State::Disconnected));
}

/// Build a (lazily-resolved) channel for the configured endpoint.
async fn build_channel(cfg: &ConnConfig) -> Result<Channel, BoxError> {
    let scheme = if cfg.use_tls { "https" } else { "http" };
    let uri = format!("{scheme}://{}:{}", cfg.address, cfg.port);
    let mut endpoint: Endpoint = Channel::from_shared(uri)?;
    if cfg.use_tls {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?;
    }
    Ok(endpoint.connect().await?)
}

macro_rules! stream_loop {
    (
        $loop_name:ident,
        $connect_name:ident,
        $client:ty,
        $rpc:ident,
        $slot:ident,
        $stream:expr,
        $label:literal
    ) => {
        /// Reconnecting supervisor for one stream.
        async fn $loop_name(
            cfg: ConnConfig,
            bus: OsiFrameBus,
            tx: Sender<Event>,
            mut shutdown: Shutdown,
            rec: RecordSlot,
        ) {
            loop {
                if *shutdown.borrow() {
                    return;
                }
                let _ = tx.send(Event::State(State::Connecting));
                if let Err(e) = $connect_name(&cfg, &bus, &tx, &mut shutdown, &rec).await {
                    let _ = tx.send(Event::Error(format!(concat!($label, ": {}"), e)));
                }
                if !cfg.reconnect || *shutdown.borrow() {
                    return;
                }
                let _ = tx.send(Event::State(State::Reconnecting));
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(cfg.reconnect_delay_ms)) => {}
                    _ = shutdown.changed() => {}
                }
            }
        }

        /// Connect, subscribe, and consume frames until the stream ends, an
        /// error occurs, or shutdown is requested.
        async fn $connect_name(
            cfg: &ConnConfig,
            bus: &OsiFrameBus,
            tx: &Sender<Event>,
            shutdown: &mut Shutdown,
            rec: &RecordSlot,
        ) -> Result<(), BoxError> {
            let channel = build_channel(cfg).await?;
            let mut client = <$client>::new(channel);
            let response = client.$rpc(Request::new(())).await?;
            let mut stream = response.into_inner();
            let _ = tx.send(Event::State(State::Connected));
            loop {
                tokio::select! {
                    msg = stream.message() => {
                        match msg? {
                            Some(frame) => {
                                if let Some(writer) = rec.lock().expect("record slot poisoned").as_mut() {
                                    let _ = writer.write_frame(&frame);
                                }
                                bus.$slot.store(frame);
                                let _ = tx.send(Event::Frame($stream));
                            }
                            // Server closed the stream cleanly.
                            None => return Ok(()),
                        }
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            return Ok(());
                        }
                    }
                }
            }
        }
    };
}

stream_loop!(
    ground_truth_loop,
    connect_ground_truth,
    GroundTruthServiceClient<Channel>,
    stream_ground_truth,
    ground_truth,
    Stream::GroundTruth,
    "ground_truth"
);

stream_loop!(
    host_vehicle_loop,
    connect_host_vehicle,
    HostVehicleDataServiceClient<Channel>,
    stream_host_vehicle_data,
    host_vehicle_data,
    Stream::HostVehicleData,
    "host_vehicle_data"
);

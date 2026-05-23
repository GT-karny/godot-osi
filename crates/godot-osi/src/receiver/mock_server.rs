//! Bundled mock gRPC server for testing the receiver without a real server
//! (REQUIREMENTS.md §6).
//!
//! Two modes:
//! - *synthetic*: emits procedurally-generated `GroundTruth` / `HostVehicleData`
//!   frames (a single moving object advancing along +x).
//! - *trace replay*: streams frames loaded from recorded `.osi` files
//!   (`super::trace`), looping.
//!
//! The server logic (`serve`, the service impls, `synthetic_*`) is plain Rust
//! and is exercised directly by the `cargo test` suite. `OsiMockServer` wraps it
//! as a Godot class so a scene can spin up a mock alongside an `OsiReceiver`.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use osi_types::osi::server::ground_truth_service_server::{
    GroundTruthService, GroundTruthServiceServer,
};
use osi_types::osi::server::host_vehicle_data_service_server::{
    HostVehicleDataService, HostVehicleDataServiceServer,
};
use osi_types::osi3;
use tokio::sync::watch;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

/// What the mock server should stream.
#[derive(Clone)]
pub struct MockConfig {
    pub address: String,
    pub port: u16,
    pub period: Duration,
    /// GroundTruth frames to loop over; empty => synthetic frames are generated.
    pub ground_truth: Vec<osi3::GroundTruth>,
    /// HostVehicleData frames to loop over; empty => synthetic frames are generated.
    pub host_vehicle_data: Vec<osi3::HostVehicleData>,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_string(),
            port: 50051,
            period: Duration::from_millis(50),
            ground_truth: Vec::new(),
            host_vehicle_data: Vec::new(),
        }
    }
}

// ---- synthetic data ---------------------------------------------------------

fn timestamp(seconds: i64, nanos: u32) -> osi3::Timestamp {
    osi3::Timestamp {
        seconds: Some(seconds),
        nanos: Some(nanos),
    }
}

fn version() -> osi3::InterfaceVersion {
    osi3::InterfaceVersion {
        version_major: Some(3),
        version_minor: Some(7),
        version_patch: Some(0),
    }
}

/// A short loop of GroundTruth frames: one moving object (id 1) advancing +x.
pub fn synthetic_ground_truth() -> Vec<osi3::GroundTruth> {
    (0..120)
        .map(|i| osi3::GroundTruth {
            version: Some(version()),
            timestamp: Some(timestamp(i / 20, (i % 20) as u32 * 50_000_000)),
            host_vehicle_id: Some(osi3::Identifier { value: Some(1) }),
            moving_object: vec![osi3::MovingObject {
                id: Some(osi3::Identifier { value: Some(1) }),
                base: Some(osi3::BaseMoving {
                    position: Some(osi3::Vector3d {
                        x: Some(i as f64 * 0.5),
                        y: Some(0.0),
                        z: Some(0.0),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect()
}

/// A short loop of HostVehicleData frames with advancing timestamps.
pub fn synthetic_host_vehicle_data() -> Vec<osi3::HostVehicleData> {
    (0..120)
        .map(|i| osi3::HostVehicleData {
            version: Some(version()),
            timestamp: Some(timestamp(i / 20, (i % 20) as u32 * 50_000_000)),
            host_vehicle_id: Some(osi3::Identifier { value: Some(1) }),
            ..Default::default()
        })
        .collect()
}

// ---- service implementations ------------------------------------------------

struct GtService {
    frames: Arc<Vec<osi3::GroundTruth>>,
    period: Duration,
}

#[tonic::async_trait]
impl GroundTruthService for GtService {
    type StreamGroundTruthStream = ReceiverStream<Result<osi3::GroundTruth, Status>>;

    async fn stream_ground_truth(
        &self,
        _request: Request<()>,
    ) -> Result<Response<Self::StreamGroundTruthStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let frames = self.frames.clone();
        let period = self.period;
        tokio::spawn(async move {
            if frames.is_empty() {
                return;
            }
            loop {
                for frame in frames.iter() {
                    if tx.send(Ok(frame.clone())).await.is_err() {
                        return; // client dropped the stream
                    }
                    tokio::time::sleep(period).await;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

struct HvdService {
    frames: Arc<Vec<osi3::HostVehicleData>>,
    period: Duration,
}

#[tonic::async_trait]
impl HostVehicleDataService for HvdService {
    type StreamHostVehicleDataStream = ReceiverStream<Result<osi3::HostVehicleData, Status>>;

    async fn stream_host_vehicle_data(
        &self,
        _request: Request<()>,
    ) -> Result<Response<Self::StreamHostVehicleDataStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let frames = self.frames.clone();
        let period = self.period;
        tokio::spawn(async move {
            if frames.is_empty() {
                return;
            }
            loop {
                for frame in frames.iter() {
                    if tx.send(Ok(frame.clone())).await.is_err() {
                        return;
                    }
                    tokio::time::sleep(period).await;
                }
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

/// Serve both services on `cfg.address:cfg.port` until `shutdown` flips true.
pub async fn serve(cfg: MockConfig, mut shutdown: watch::Receiver<bool>) -> Result<(), BoxError> {
    let addr: SocketAddr = format!("{}:{}", cfg.address, cfg.port).parse()?;

    let gt = if cfg.ground_truth.is_empty() {
        synthetic_ground_truth()
    } else {
        cfg.ground_truth
    };
    let hvd = if cfg.host_vehicle_data.is_empty() {
        synthetic_host_vehicle_data()
    } else {
        cfg.host_vehicle_data
    };

    let gt_service = GtService {
        frames: Arc::new(gt),
        period: cfg.period,
    };
    let hvd_service = HvdService {
        frames: Arc::new(hvd),
        period: cfg.period,
    };

    Server::builder()
        .add_service(GroundTruthServiceServer::new(gt_service))
        .add_service(HostVehicleDataServiceServer::new(hvd_service))
        .serve_with_shutdown(addr, async move {
            let _ = shutdown.wait_for(|stop| *stop).await;
        })
        .await?;
    Ok(())
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Spawn the mock server on a background thread + tokio runtime. Returns the
/// thread handle and a shutdown sender (send `true` to stop).
pub fn spawn(cfg: MockConfig) -> (JoinHandle<()>, watch::Sender<bool>) {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build mock-server tokio runtime");
        if let Err(e) = rt.block_on(serve(cfg, shutdown_rx)) {
            // Runs on a background thread, so we cannot call Godot APIs here.
            eprintln!("OsiMockServer: serve error: {e}");
        }
    });
    (handle, shutdown_tx)
}

// ---- Godot class ------------------------------------------------------------

use godot::classes::{INode, Node};
use godot::prelude::*;

/// A bundled mock gRPC server, controllable from GDScript, that feeds an
/// [`OsiReceiver`](super::OsiReceiver) without a real backend.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct OsiMockServer {
    #[var]
    address: GString,
    #[var]
    port: i64,
    /// Milliseconds between emitted frames.
    #[var]
    period_ms: i64,

    /// Optional `.osi` trace files; if set, replayed instead of synthetic data.
    ground_truth_trace: Option<String>,
    host_vehicle_trace: Option<String>,

    thread: Option<JoinHandle<()>>,
    shutdown: Option<watch::Sender<bool>>,
    base: Base<Node>,
}

#[godot_api]
impl INode for OsiMockServer {
    fn init(base: Base<Node>) -> Self {
        Self {
            address: "127.0.0.1".into(),
            port: 50051,
            period_ms: 50,
            ground_truth_trace: None,
            host_vehicle_trace: None,
            thread: None,
            shutdown: None,
            base,
        }
    }

    fn exit_tree(&mut self) {
        self.stop();
    }
}

#[godot_api]
impl OsiMockServer {
    /// Replay these `.osi` trace files (call before `start`). Pass an empty
    /// string to clear and fall back to synthetic data for that stream.
    #[func]
    fn set_traces(&mut self, ground_truth_path: GString, host_vehicle_path: GString) {
        let gt = ground_truth_path.to_string();
        let hvd = host_vehicle_path.to_string();
        self.ground_truth_trace = (!gt.is_empty()).then_some(gt);
        self.host_vehicle_trace = (!hvd.is_empty()).then_some(hvd);
    }

    /// Start serving. If already running, restarts.
    #[func]
    fn start(&mut self) {
        self.stop();

        let mut cfg = MockConfig {
            address: self.address.to_string(),
            port: self.port as u16,
            period: Duration::from_millis(self.period_ms.max(1) as u64),
            ground_truth: Vec::new(),
            host_vehicle_data: Vec::new(),
        };

        if let Some(path) = &self.ground_truth_trace {
            match super::trace::read_trace::<osi3::GroundTruth>(path) {
                Ok(frames) => cfg.ground_truth = frames,
                Err(e) => godot_error!("OsiMockServer: failed to read GroundTruth trace: {e}"),
            }
        }
        if let Some(path) = &self.host_vehicle_trace {
            match super::trace::read_trace::<osi3::HostVehicleData>(path) {
                Ok(frames) => cfg.host_vehicle_data = frames,
                Err(e) => godot_error!("OsiMockServer: failed to read HostVehicleData trace: {e}"),
            }
        }

        let (handle, shutdown) = spawn(cfg);
        self.thread = Some(handle);
        self.shutdown = Some(shutdown);
    }

    /// Stop serving and join the background thread.
    #[func]
    fn stop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(true);
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

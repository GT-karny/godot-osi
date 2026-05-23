//! Tests against real OSI traces recorded from the production gRPC server.
//!
//! `.osi` native traces are a flat sequence of length-delimited protobuf
//! messages: a 4-byte little-endian length, then that many bytes of a single
//! serialized message (`GroundTruth` for `gt.osi`, `HostVehicleData` for
//! `hvd.osi`). These tests decode *every* frame with our prost types — proving
//! the schema matches the real producer — and run the §3 coordinate conversion
//! on real `MovingObject` poses (engine-free, so plain `cargo test`).
//!
//! The traces live in `<repo>/traces/` and are not committed; if they are
//! absent the tests skip with a notice rather than fail.

use osi_types::osi3;
use prost::Message;

use crate::converter::coords::{osi_base_moving_to_transform, AxisMapping};

/// `<repo>/traces/<name>`; repo root is two levels above this crate's manifest.
fn trace_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .join("traces")
        .join(name)
}

/// Split a `.osi` blob into the raw bytes of each length-delimited frame.
fn split_frames(buf: &[u8]) -> Vec<&[u8]> {
    let mut frames = Vec::new();
    let mut off = 0;
    while off + 4 <= buf.len() {
        let len = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]) as usize;
        off += 4;
        assert!(off + len <= buf.len(), "truncated frame at offset {off}");
        frames.push(&buf[off..off + len]);
        off += len;
    }
    assert_eq!(off, buf.len(), "trailing bytes after last frame");
    frames
}

fn read_trace(name: &str) -> Option<Vec<u8>> {
    let path = trace_path(name);
    match std::fs::read(&path) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skipping: trace not found at {}", path.display());
            None
        }
    }
}

#[test]
fn decode_all_ground_truth_frames() {
    let Some(buf) = read_trace("gt.osi") else { return };
    let frames = split_frames(&buf);
    assert!(!frames.is_empty(), "no frames in gt.osi");

    let mut total_moving = 0usize;
    for (i, raw) in frames.iter().enumerate() {
        let gt = osi3::GroundTruth::decode(*raw)
            .unwrap_or_else(|e| panic!("frame {i} failed to decode: {e}"));
        total_moving += gt.moving_object.len();
        // Spot-check the version on the first frame (expect OSI 3.7.x).
        if i == 0 {
            if let Some(v) = &gt.version {
                assert_eq!(v.version_major, Some(3), "unexpected OSI major version");
            }
        }
    }
    eprintln!(
        "gt.osi: {} frames decoded, {} MovingObjects total",
        frames.len(),
        total_moving
    );
    assert!(total_moving > 0, "expected at least one MovingObject");
}

#[test]
fn decode_all_host_vehicle_data_frames() {
    let Some(buf) = read_trace("hvd.osi") else { return };
    let frames = split_frames(&buf);
    assert!(!frames.is_empty(), "no frames in hvd.osi");

    for (i, raw) in frames.iter().enumerate() {
        osi3::HostVehicleData::decode(*raw)
            .unwrap_or_else(|e| panic!("frame {i} failed to decode: {e}"));
    }
    eprintln!("hvd.osi: {} frames decoded", frames.len());
}

#[test]
fn coordinate_conversion_on_real_poses_is_finite() {
    let Some(buf) = read_trace("gt.osi") else { return };
    let frames = split_frames(&buf);
    let mapping = AxisMapping::default();

    let mut checked = 0usize;
    // First frame is enough to exercise real poses; keep the test fast.
    let gt = osi3::GroundTruth::decode(frames[0]).expect("decode frame 0");
    for mo in &gt.moving_object {
        let Some(base) = &mo.base else { continue };
        let t = osi_base_moving_to_transform(base, &mapping);
        // Every component of a real pose must convert to a finite transform.
        assert!(t.origin.x.is_finite() && t.origin.y.is_finite() && t.origin.z.is_finite());
        for col in [t.basis.col_a(), t.basis.col_b(), t.basis.col_c()] {
            assert!(col.x.is_finite() && col.y.is_finite() && col.z.is_finite());
        }
        checked += 1;
    }
    eprintln!("frame 0: converted {checked} MovingObject poses, all finite");
    assert!(checked > 0, "no poses with a base to convert");
}

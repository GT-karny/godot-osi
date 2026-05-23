//! OSI trace (`.osi`) recording and replay (REQUIREMENTS.md §6).
//!
//! Format: a flat sequence of records, each `[u32 little-endian length]` then
//! that many bytes of a protobuf-encoded OSI message. This matches the ASAM OSI
//! native single-message-per-record convention. A trace file holds frames of a
//! *single* message type (e.g. one file of `GroundTruth`, another of
//! `HostVehicleData`); the reader/writer are generic over the message type.
//!
//! Recording is driven by the receiver while connected (`start_recording`);
//! replay is served back through the bundled mock server (`mock_server.rs`).

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use prost::Message;

/// Streams length-prefixed protobuf records to a `.osi` file.
pub struct TraceWriter {
    inner: BufWriter<File>,
}

impl TraceWriter {
    /// Create (or truncate) `path` for writing.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        Ok(Self {
            inner: BufWriter::new(File::create(path)?),
        })
    }

    /// Append one message as `[u32 LE len][bytes]`.
    pub fn write_frame<M: Message>(&mut self, msg: &M) -> io::Result<()> {
        let bytes = msg.encode_to_vec();
        let len = u32::try_from(bytes.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame exceeds 4 GiB"))?;
        self.inner.write_all(&len.to_le_bytes())?;
        self.inner.write_all(&bytes)?;
        Ok(())
    }

    /// Flush buffered bytes to disk.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Read every record from a `.osi` file into a vector of decoded messages.
///
/// Returns an error on a truncated or corrupt record. Used by the mock server
/// to load a recorded trace for replay; traces are expected to fit in memory.
pub fn read_trace<M: Message + Default>(path: impl AsRef<Path>) -> io::Result<Vec<M>> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut frames = Vec::new();
    let mut len_buf = [0u8; 4];
    loop {
        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            // A clean EOF at a record boundary means we're done.
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e),
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut bytes = vec![0u8; len];
        reader.read_exact(&mut bytes)?;
        let msg = M::decode(&bytes[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        frames.push(msg);
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use osi_types::osi3;

    #[test]
    fn round_trip_preserves_frames() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("osi_trace_test_{}.osi", std::process::id()));

        let mut gt1 = osi3::GroundTruth::default();
        gt1.host_vehicle_id = Some(osi3::Identifier { value: Some(7) });
        let gt2 = osi3::GroundTruth::default();

        let mut w = TraceWriter::create(&path).unwrap();
        w.write_frame(&gt1).unwrap();
        w.write_frame(&gt2).unwrap();
        w.flush().unwrap();

        let frames: Vec<osi3::GroundTruth> = read_trace(&path).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].host_vehicle_id, gt1.host_vehicle_id);
        assert_eq!(frames[1], osi3::GroundTruth::default());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_file_yields_no_frames() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("osi_trace_empty_{}.osi", std::process::id()));
        TraceWriter::create(&path).unwrap().flush().unwrap();
        let frames: Vec<osi3::GroundTruth> = read_trace(&path).unwrap();
        assert!(frames.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}

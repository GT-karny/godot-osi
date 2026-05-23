//! Boundary between the receiver and converter plugins.
//!
//! This is the *only* shared contract the two parallel work streams must agree
//! on. The receiver (producer) writes the latest raw OSI frame; the converter
//! (consumer) reads it. "Newest-wins" semantics per REQUIREMENTS.md (§ frame
//! policy): storing a new frame drops any previous unconsumed one.
//!
//! Both sides depend only on `osi_types` (the prost-generated OSI messages),
//! so each can be developed and tested independently of the other.

use std::sync::{Arc, Mutex};

use osi_types::osi3;

/// A newest-wins slot holding at most one frame of type `T`.
pub struct LatestSlot<T> {
    inner: Mutex<Option<T>>,
}

impl<T> LatestSlot<T> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Overwrite with the newest frame, dropping any unconsumed older one.
    pub fn store(&self, value: T) {
        *self.inner.lock().expect("LatestSlot poisoned") = Some(value);
    }

    /// Take the latest frame, leaving the slot empty.
    pub fn take(&self) -> Option<T> {
        self.inner.lock().expect("LatestSlot poisoned").take()
    }

    /// Clone the latest frame without consuming it.
    pub fn peek(&self) -> Option<T>
    where
        T: Clone,
    {
        self.inner.lock().expect("LatestSlot poisoned").clone()
    }

    /// Whether a frame is currently waiting to be consumed.
    pub fn has_frame(&self) -> bool {
        self.inner.lock().expect("LatestSlot poisoned").is_some()
    }
}

impl<T> Default for LatestSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared bus connecting the receiver (producer) and converter (consumer).
///
/// Cloning shares the same underlying slots (`Arc`), so the receiver and the
/// converter node can hold clones of the *same* bus and exchange frames
/// without copying. Wiring both ends to one instance is integration work; for
/// independent development each side can create its own bus and feed/drain it
/// with synthetic data.
#[derive(Clone, Default)]
pub struct OsiFrameBus {
    pub ground_truth: Arc<LatestSlot<osi3::GroundTruth>>,
    pub host_vehicle_data: Arc<LatestSlot<osi3::HostVehicleData>>,
}

impl OsiFrameBus {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_wins() {
        let slot = LatestSlot::<u32>::new();
        assert!(!slot.has_frame());
        slot.store(1);
        slot.store(2);
        assert!(slot.has_frame());
        assert_eq!(slot.take(), Some(2));
        assert_eq!(slot.take(), None);
    }

    #[test]
    fn bus_clones_share_slots() {
        let bus = OsiFrameBus::new();
        let other = bus.clone();
        bus.ground_truth.store(osi3::GroundTruth::default());
        assert!(other.ground_truth.has_frame());
    }
}

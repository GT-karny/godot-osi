//! Optional helper "A" (REQUIREMENTS.md §5): a sample/part that turns a
//! converted GroundTruth snapshot into scene nodes.
//!
//! `OsiMovingObjectSpawner` is a `Node3D` that listens to an
//! [`OsiConverter`](super::node::OsiConverter)'s `ground_truth_converted`
//! signal, tracks each `MovingObject` by id, and:
//!   - spawns a child `Node3D` when a new id appears,
//!   - updates that child's `Transform3D` every frame (via the configurable
//!     [`AxisMapping`] coordinate conversion),
//!   - frees the child when its id disappears.
//!
//! This is intentionally separate from the core converter (which never touches
//! scene nodes); use it as-is or as a template.

use std::collections::{HashMap, HashSet};

use godot::classes::Node3D;
use godot::prelude::*;
use osi_types::osi3;

use crate::converter::coords::{self, AxisMapping};
use crate::converter::generated::OsiGroundTruth;
use crate::converter::node::OsiConverter;

#[derive(GodotClass)]
#[class(base=Node3D, init)]
pub struct OsiMovingObjectSpawner {
    /// MovingObject id -> the child node representing it.
    tracked: HashMap<i64, Gd<Node3D>>,
    /// Uniform scale forwarded to the coordinate mapping (meters by default).
    #[var]
    #[init(val = 1.0)]
    scale: real,
    base: Base<Node3D>,
}

#[godot_api]
impl OsiMovingObjectSpawner {
    /// Connect to `converter`'s `ground_truth_converted` signal so this spawner
    /// updates itself automatically. (GDScript can instead connect the signal
    /// to [`Self::on_ground_truth`] directly.)
    #[func]
    fn bind_converter(&mut self, mut converter: Gd<OsiConverter>) {
        let this = self.to_gd();
        let callable = Callable::from_object_method(&this, "on_ground_truth");
        converter.connect("ground_truth_converted", &callable);
    }

    /// Reconcile tracked child nodes against the objects in `snapshot`.
    #[func]
    pub fn on_ground_truth(&mut self, snapshot: Gd<OsiGroundTruth>) {
        let mapping = AxisMapping {
            scale: self.scale,
            ..Default::default()
        };

        // Snapshot the object list, then release the bind before mutating self.
        let objects = snapshot.bind().moving_object.clone();

        let mut present: HashSet<i64> = HashSet::with_capacity(objects.len());
        for mo_gd in objects.iter_shared() {
            let mo = mo_gd.bind();
            let id = mo
                .id
                .as_ref()
                .map(|i| i.bind().value)
                .unwrap_or(-1);
            present.insert(id);

            let transform = mo
                .base
                .as_ref()
                .map(|b| base_moving_transform(&b.bind(), &mapping))
                .unwrap_or_default();

            if let Some(node) = self.tracked.get_mut(&id) {
                node.set_transform(transform);
            } else {
                let mut node = Node3D::new_alloc();
                node.set_name(&format!("osi_mo_{id}"));
                node.set_transform(transform);
                self.base_mut().add_child(&node);
                self.tracked.insert(id, node);
            }
        }

        // Free children whose ids are no longer present.
        let stale: Vec<i64> = self
            .tracked
            .keys()
            .filter(|id| !present.contains(id))
            .copied()
            .collect();
        for id in stale {
            if let Some(mut node) = self.tracked.remove(&id) {
                node.queue_free();
            }
        }
    }

    /// Number of currently tracked objects (handy for tests/debugging).
    #[func]
    fn tracked_count(&self) -> i64 {
        self.tracked.len() as i64
    }
}

/// Build a Godot `Transform3D` from the generated `OsiBaseMoving` mirror by
/// extracting the raw position/orientation and running the shared, unit-tested
/// coordinate conversion (so the helper and the §3 tests can't diverge).
fn base_moving_transform(
    base: &super::generated::OsiBaseMoving,
    mapping: &AxisMapping,
) -> Transform3D {
    let position = base.position.as_ref().map(|p| {
        let p = p.bind();
        osi3::Vector3d {
            x: Some(p.x),
            y: Some(p.y),
            z: Some(p.z),
        }
    });
    let orientation = base.orientation.as_ref().map(|o| {
        let o = o.bind();
        osi3::Orientation3d {
            roll: Some(o.roll),
            pitch: Some(o.pitch),
            yaw: Some(o.yaw),
        }
    });
    let bm = osi3::BaseMoving {
        position,
        orientation,
        ..Default::default()
    };
    coords::osi_base_moving_to_transform(&bm, mapping)
}

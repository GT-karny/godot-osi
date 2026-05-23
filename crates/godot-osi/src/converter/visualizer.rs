//! Optional visual helper (demo): turn a converted GroundTruth snapshot into
//! *visible* boxes — sized by each object's OSI `dimension`, colored by its
//! `MovingObject` type, and posed by the shared, unit-tested coordinate
//! conversion (so it can't drift from REQUIREMENTS.md §3).
//!
//! Like [`super::spawn_helper::OsiMovingObjectSpawner`] this is intentionally a
//! sample built on top of the core converter (which never touches scene nodes).
//! `OsiMovingObjectSpawner` spawns empty `Node3D`s (a minimal template); this
//! one spawns `MeshInstance3D`s so there is something to look at.

use std::collections::{HashMap, HashSet};

use godot::classes::{BoxMesh, MeshInstance3D, Node3D, StandardMaterial3D};
use godot::prelude::*;
use osi_types::osi3;

use crate::converter::coords::{self, AxisMapping};
use crate::converter::generated::{OsiBaseMoving, OsiDimension3d, OsiGroundTruth};
use crate::converter::node::OsiConverter;

#[derive(GodotClass)]
#[class(base=Node3D, init)]
pub struct OsiMovingObjectVisualizer {
    /// MovingObject id -> the box node representing it.
    tracked: HashMap<i64, Gd<MeshInstance3D>>,
    /// Uniform scale forwarded to the coordinate mapping (meters by default).
    #[var]
    #[init(val = 1.0)]
    scale: real,
    base: Base<Node3D>,
}

#[godot_api]
impl OsiMovingObjectVisualizer {
    /// Connect to `converter`'s `ground_truth_converted` signal so the boxes
    /// update automatically every converted frame.
    #[func]
    fn bind_converter(&mut self, mut converter: Gd<OsiConverter>) {
        let this = self.to_gd();
        let callable = Callable::from_object_method(&this, "on_ground_truth");
        converter.connect("ground_truth_converted", &callable);
    }

    /// Reconcile the visible boxes against the objects in `snapshot`.
    #[func]
    pub fn on_ground_truth(&mut self, snapshot: Gd<OsiGroundTruth>) {
        let mapping = AxisMapping {
            scale: self.scale,
            ..Default::default()
        };

        let objects = snapshot.bind().moving_object.clone();
        let mut present: HashSet<i64> = HashSet::with_capacity(objects.len());

        for mo_gd in objects.iter_shared() {
            let mo = mo_gd.bind();
            let id = mo.id.as_ref().map(|i| i.bind().value).unwrap_or(-1);
            present.insert(id);

            let (transform, size) = match mo.base.as_ref() {
                Some(b) => {
                    let b = b.bind();
                    let transform = base_transform(&b, &mapping);
                    let dim = dimension_or_default(b.dimension.as_ref(), mo.type_);
                    (transform, coords::osi_dimension_to_godot(&dim, &mapping))
                }
                None => (Transform3D::IDENTITY, Vector3::ONE),
            };

            if let Some(node) = self.tracked.get_mut(&id) {
                node.set_transform(transform);
                set_box_size(node, size);
            } else {
                let mut node = make_box(size, mo.type_);
                node.set_name(&format!("osi_mo_{id}"));
                node.set_transform(transform);
                self.base_mut().add_child(&node);
                self.tracked.insert(id, node);
            }
        }

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

    /// Number of currently visible boxes.
    #[func]
    fn tracked_count(&self) -> i64 {
        self.tracked.len() as i64
    }
}

/// Build a `Transform3D` from the generated base mirror via the shared coords.
fn base_transform(base: &OsiBaseMoving, mapping: &AxisMapping) -> Transform3D {
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

/// Use the object's dimension if present and non-degenerate; otherwise a
/// type-appropriate default so objects from sources that omit `dimension`
/// (e.g. minimal mock data) are still visible.
fn dimension_or_default(dim: Option<&Gd<OsiDimension3d>>, type_: i64) -> osi3::Dimension3d {
    if let Some(d) = dim {
        let d = d.bind();
        if d.length > 0.01 || d.width > 0.01 || d.height > 0.01 {
            return osi3::Dimension3d {
                length: Some(d.length),
                width: Some(d.width),
                height: Some(d.height),
            };
        }
    }
    let (length, width, height) = match type_ {
        2 => (4.5, 2.0, 1.5), // vehicle
        3 => (0.6, 0.6, 1.8), // pedestrian
        4 => (1.0, 0.5, 0.8), // animal
        _ => (1.0, 1.0, 1.0),
    };
    osi3::Dimension3d {
        length: Some(length),
        width: Some(width),
        height: Some(height),
    }
}

/// MovingObject.Type -> a distinct albedo color.
fn color_for_type(type_: i64) -> Color {
    match type_ {
        1 => Color::from_rgba(0.85, 0.85, 0.85, 1.0), // OTHER
        2 => Color::from_rgba(0.20, 0.45, 0.90, 1.0), // VEHICLE
        3 => Color::from_rgba(0.20, 0.80, 0.35, 1.0), // PEDESTRIAN
        4 => Color::from_rgba(0.95, 0.60, 0.15, 1.0), // ANIMAL
        _ => Color::from_rgba(0.60, 0.60, 0.60, 1.0), // UNKNOWN
    }
}

fn make_box(size: Vector3, type_: i64) -> Gd<MeshInstance3D> {
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);

    let mut material = StandardMaterial3D::new_gd();
    material.set_albedo(color_for_type(type_));

    let mut node = MeshInstance3D::new_alloc();
    node.set_mesh(&mesh);
    node.set_material_override(&material);
    node
}

fn set_box_size(node: &mut Gd<MeshInstance3D>, size: Vector3) {
    if let Some(mesh) = node.get_mesh() {
        if let Ok(mut box_mesh) = mesh.try_cast::<BoxMesh>() {
            box_mesh.set_size(size);
        }
    }
}

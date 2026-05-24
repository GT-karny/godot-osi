//! `OsiRoadNetworkVisualizer`: optional `Node3D` demo helper that renders an
//! [`OsiRoadNetwork`] — a road-surface `MeshInstance3D` plus small markers at
//! each road sign. Roads are static, so this builds once on demand (no per-frame
//! polling, unlike the OSI converter pipeline).

use godot::classes::{BoxMesh, Material, MeshInstance3D, Node3D, StandardMaterial3D};
use godot::prelude::*;

use super::mesh::{build_road_marks, build_road_mesh};
use super::network::{to_godot, OsiRoadNetwork};
use crate::converter::coords::AxisMapping;

#[derive(GodotClass)]
#[class(base=Node3D, init)]
pub struct OsiRoadNetworkVisualizer {
    /// Uniform scale forwarded to the coordinate mapping (meters by default).
    #[var]
    #[init(val = 1.0)]
    scale: real,
    /// Longitudinal sampling stride (m) for the road surface mesh.
    #[var]
    #[init(val = 1.0)]
    sample_step: f64,
    /// Whether to drop a marker box at each road sign.
    #[var]
    #[init(val = true)]
    show_signs: bool,
    /// Whether to draw the OpenDRIVE road marks (lane lines) on the surface.
    #[var]
    #[init(val = true)]
    show_road_marks: bool,

    surface: Option<Gd<MeshInstance3D>>,
    road_marks: Option<Gd<MeshInstance3D>>,
    signs_root: Option<Gd<Node3D>>,
    base: Base<Node3D>,
}

#[godot_api]
impl OsiRoadNetworkVisualizer {
    /// (Re)build the visible road from `network`. Frees any previous render.
    #[func]
    fn build_from(&mut self, network: Gd<OsiRoadNetwork>) {
        if let Some(mut n) = self.surface.take() {
            n.queue_free();
        }
        if let Some(mut n) = self.road_marks.take() {
            n.queue_free();
        }
        if let Some(mut n) = self.signs_root.take() {
            n.queue_free();
        }

        let mapping = AxisMapping {
            scale: self.scale,
            ..Default::default()
        };
        let step = self.sample_step;
        let show_signs = self.show_signs;
        let show_road_marks = self.show_road_marks;

        // Build geometry while the network is borrowed; collect plain results.
        let (mesh, marks, sign_pts) = {
            let net = network.bind();
            let Some(map) = net.map() else {
                godot_warn!("[OsiRoadNetworkVisualizer] network not loaded");
                return;
            };
            let mesh = build_road_mesh(map, &mapping, step);
            let marks = if show_road_marks {
                build_road_marks(map, &mapping)
            } else {
                None
            };
            let sign_pts: Vec<Vector3> = if show_signs {
                map.signs()
                    .iter()
                    .map(|s| to_godot([s.x, s.y, s.z], &mapping))
                    .collect()
            } else {
                Vec::new()
            };
            (mesh, marks, sign_pts)
        };

        match mesh {
            Some(mesh) => {
                let mut mi = MeshInstance3D::new_alloc();
                mi.set_name("RoadSurface");
                mi.set_mesh(&mesh);
                self.base_mut().add_child(&mi);
                self.surface = Some(mi);
            }
            None => godot_warn!("[OsiRoadNetworkVisualizer] no road surface generated"),
        }

        if let Some(marks) = marks {
            let mut mi = MeshInstance3D::new_alloc();
            mi.set_name("RoadMarks");
            mi.set_mesh(&marks);
            self.base_mut().add_child(&mi);
            self.road_marks = Some(mi);
        }

        if !sign_pts.is_empty() {
            let marker = sign_marker_mesh();
            let mut root = Node3D::new_alloc();
            root.set_name("Signs");
            for (i, p) in sign_pts.iter().enumerate() {
                let mut mi = MeshInstance3D::new_alloc();
                mi.set_name(&format!("sign_{i}"));
                mi.set_mesh(&marker);
                mi.set_position(*p);
                root.add_child(&mi);
            }
            self.base_mut().add_child(&root);
            self.signs_root = Some(root);
        }
    }

    /// Number of road-surface meshes currently shown (0 or 1). Test/debug hook.
    #[func]
    fn has_surface(&self) -> bool {
        self.surface.is_some()
    }
}

/// A small yellow box mesh shared by all sign markers.
fn sign_marker_mesh() -> Gd<BoxMesh> {
    let mut m = BoxMesh::new_gd();
    m.set_size(Vector3::new(0.3, 2.0, 0.3));
    let mut mat = StandardMaterial3D::new_gd();
    mat.set_albedo(Color::from_rgba(0.95, 0.85, 0.15, 1.0));
    let mat: Gd<Material> = mat.upcast();
    m.set_material(&mat);
    m
}

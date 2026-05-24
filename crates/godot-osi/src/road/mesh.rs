//! Build a Godot `ArrayMesh` for the road surface from sampled OpenDRIVE geometry.
//!
//! [`esmini_rm::OdrMap::road_surface_quads`] returns world-space quads (esmini
//! already baked in elevation/superelevation/spiral geometry); here we just map
//! them to Godot space and triangulate via `SurfaceTool`. Stage 1 uses a single
//! flat asphalt material with culling disabled (so triangle winding never hides
//! a lane); per-lane-type coloring/road-marks are a later stage.

use esmini_rm::{road_mark_color, OdrMap};
use godot::classes::base_material_3d::{CullMode, Flags, ShadingMode};
use godot::classes::mesh::PrimitiveType;
use godot::classes::{ArrayMesh, Material, StandardMaterial3D, SurfaceTool};
use godot::prelude::*;

use super::network::to_godot;
use crate::converter::coords::AxisMapping;

/// Triangulate the road surface into an `ArrayMesh`, or `None` if the network
/// has no drivable surface to draw. `step` is the longitudinal sampling stride
/// in meters.
pub fn build_road_mesh(map: &OdrMap, mapping: &AxisMapping, step: f64) -> Option<Gd<ArrayMesh>> {
    let quads = map.road_surface_quads(step);
    if quads.is_empty() {
        return None;
    }

    let mut st = SurfaceTool::new_gd();
    st.begin(PrimitiveType::TRIANGLES);

    for q in &quads {
        let c = [
            to_godot(q.corners[0], mapping),
            to_godot(q.corners[1], mapping),
            to_godot(q.corners[2], mapping),
            to_godot(q.corners[3], mapping),
        ];
        // Two triangles forming the quad ring 0-1-2-3.
        for &i in &[0usize, 1, 2, 0, 2, 3] {
            st.add_vertex(c[i]);
        }
    }

    st.generate_normals();

    let mut mat = StandardMaterial3D::new_gd();
    mat.set_albedo(Color::from_rgba(0.25, 0.25, 0.27, 1.0));
    mat.set_cull_mode(CullMode::DISABLED);
    let mat: Gd<Material> = mat.upcast();
    st.set_material(&mat);

    st.commit()
}

/// Build the real OpenDRIVE `<roadMark>` geometry (solid/broken painted strips,
/// per-mark color) as a vertex-colored, unshaded `ArrayMesh`. esmini's
/// RoadManager precomputes the painted segments; [`OdrMap::road_marks`] returns
/// them as a triangle soup which we color and triangulate here.
pub fn build_road_marks(map: &OdrMap, mapping: &AxisMapping) -> Option<Gd<ArrayMesh>> {
    // ~2.5 cm above the asphalt (OpenDRIVE up) to avoid z-fighting.
    let marks = map.road_marks(0.025);
    if marks.verts.is_empty() {
        return None;
    }

    let mut st = SurfaceTool::new_gd();
    st.begin(PrimitiveType::TRIANGLES);
    for (i, v) in marks.verts.iter().enumerate() {
        st.set_color(road_mark_color_to_godot(marks.colors[i]));
        st.add_vertex(to_godot(*v, mapping));
    }

    let mut mat = StandardMaterial3D::new_gd();
    mat.set_shading_mode(ShadingMode::UNSHADED);
    mat.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
    mat.set_cull_mode(CullMode::DISABLED);
    let mat: Gd<Material> = mat.upcast();
    st.set_material(&mat);

    st.commit()
}

/// Map an OpenDRIVE `RoadMarkColor` to a Godot color (white for the common
/// white/standard/undefined cases).
fn road_mark_color_to_godot(c: i32) -> Color {
    match c {
        road_mark_color::YELLOW => Color::from_rgba(0.95, 0.80, 0.15, 1.0),
        road_mark_color::BLUE => Color::from_rgba(0.20, 0.40, 0.90, 1.0),
        road_mark_color::GREEN => Color::from_rgba(0.20, 0.75, 0.30, 1.0),
        road_mark_color::ORANGE => Color::from_rgba(0.95, 0.55, 0.10, 1.0),
        road_mark_color::RED => Color::from_rgba(0.90, 0.20, 0.20, 1.0),
        road_mark_color::VIOLET => Color::from_rgba(0.60, 0.30, 0.80, 1.0),
        road_mark_color::BLACK => Color::from_rgba(0.05, 0.05, 0.05, 1.0),
        _ => Color::from_rgba(0.95, 0.95, 0.95, 1.0), // white / standard / undefined
    }
}

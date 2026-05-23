//! Coordinate conversion: OSI (right-handed, Z-up, world frame) -> Godot
//! (left-handed, Y-up). REQUIREMENTS.md §3.
//!
//! These are hand-written, engine-free free functions: they take `osi3` prost
//! types and return Godot builtin math types (`Vector3`/`Basis`/`Transform3D`),
//! which are pure-Rust in gdext and therefore unit-testable under plain
//! `cargo test` without a running Godot. The generated typed `Resource` mirror
//! keeps OSI values *raw*; this module is where the §3 transform lives, and the
//! mapping is configurable via [`AxisMapping`].
//!
//! Default mapping (REQUIREMENTS.md §3):
//! - position: `Godot(x, y, z) = OSI(x, z, -y)` (a rotation of -90° about X),
//! - dimension: `length -> x`, `height -> y`, `width -> z`,
//! - orientation: similarity transform `B = P · R · Pᵀ` of the OSI euler
//!   rotation `R` by the position-mapping basis `P`.

use godot::builtin::{real, Basis, Transform3D, Vector3};
use osi_types::osi3;

/// Configurable axis mapping from the OSI world frame to the Godot frame.
///
/// `basis` is the linear map `P` such that `godot_vec = P * osi_vec`; `scale`
/// is applied to positions/dimensions (lengths) but never to rotations. The
/// [`Default`] yields the REQUIREMENTS.md §3 convention; swap in another `P`
/// for a project that prefers different axes.
#[derive(Clone, Copy, Debug)]
pub struct AxisMapping {
    pub basis: Basis,
    pub scale: real,
}

impl Default for AxisMapping {
    fn default() -> Self {
        // Columns of P = images of the OSI basis vectors in Godot coordinates:
        //   P*e_x = (1, 0, 0), P*e_y = (0, 0, -1), P*e_z = (0, 1, 0)
        // so that P*(x, y, z) = (x, z, -y).
        Self {
            basis: Basis::from_cols(
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, -1.0),
                Vector3::new(0.0, 1.0, 0.0),
            ),
            scale: 1.0,
        }
    }
}

fn opt(v: Option<f64>) -> real {
    v.unwrap_or(0.0) as real
}

/// OSI position/velocity/acceleration vector -> Godot, with axis swap + scale.
pub fn osi_position_to_godot(v: &osi3::Vector3d, m: &AxisMapping) -> Vector3 {
    let osi = Vector3::new(opt(v.x), opt(v.y), opt(v.z));
    (m.basis * osi) * m.scale
}

/// OSI box dimension (length/width/height along OSI x/y/z) -> Godot extents.
///
/// Extents transform by the absolute value of the mapping, so a pure axis swap
/// just permutes them: with the default mapping `(length, width, height)`
/// becomes `(length, height, width)`.
pub fn osi_dimension_to_godot(d: &osi3::Dimension3d, m: &AxisMapping) -> Vector3 {
    let osi = Vector3::new(opt(d.length), opt(d.width), opt(d.height));
    (m.basis * osi).abs() * m.scale
}

/// OSI euler orientation -> Godot rotation basis.
///
/// The OSI rotation is built in its own (Z-up) frame as
/// `R = Rz(yaw) · Ry(pitch) · Rx(roll)` (Tait-Bryan z-y'-x''), then expressed
/// in the Godot frame by the similarity transform `B = P · R · Pᵀ`.
pub fn osi_orientation_to_basis(o: &osi3::Orientation3d, m: &AxisMapping) -> Basis {
    let yaw = opt(o.yaw);
    let pitch = opt(o.pitch);
    let roll = opt(o.roll);
    let rz = Basis::from_axis_angle(Vector3::new(0.0, 0.0, 1.0), yaw);
    let ry = Basis::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), pitch);
    let rx = Basis::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), roll);
    let r = rz * ry * rx;
    m.basis * r * m.basis.transposed()
}

/// OSI `BaseMoving` (position + orientation) -> a Godot `Transform3D`.
pub fn osi_base_moving_to_transform(b: &osi3::BaseMoving, m: &AxisMapping) -> Transform3D {
    let origin = b
        .position
        .as_ref()
        .map(|p| osi_position_to_godot(p, m))
        .unwrap_or(Vector3::ZERO);
    let basis = b
        .orientation
        .as_ref()
        .map(|o| osi_orientation_to_basis(o, m))
        .unwrap_or(Basis::IDENTITY);
    Transform3D::new(basis, origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    const EPS: real = 1e-5;

    fn vec(x: f64, y: f64, z: f64) -> osi3::Vector3d {
        osi3::Vector3d {
            x: Some(x),
            y: Some(y),
            z: Some(z),
        }
    }

    fn close(a: Vector3, b: Vector3) -> bool {
        (a - b).length() < EPS
    }

    #[test]
    fn position_axis_swap() {
        let m = AxisMapping::default();
        // Godot(x, y, z) = OSI(x, z, -y).
        let g = osi_position_to_godot(&vec(10.0, 5.0, 0.0), &m);
        assert!(close(g, Vector3::new(10.0, 0.0, -5.0)), "got {g:?}");

        let g2 = osi_position_to_godot(&vec(1.0, 2.0, 3.0), &m);
        assert!(close(g2, Vector3::new(1.0, 3.0, -2.0)), "got {g2:?}");
    }

    #[test]
    fn position_scale() {
        let m = AxisMapping {
            scale: 2.0,
            ..Default::default()
        };
        let g = osi_position_to_godot(&vec(1.0, 2.0, 3.0), &m);
        assert!(close(g, Vector3::new(2.0, 6.0, -4.0)), "got {g:?}");
    }

    #[test]
    fn dimension_permutes_extents() {
        let m = AxisMapping::default();
        let d = osi3::Dimension3d {
            length: Some(4.5),
            width: Some(1.8),
            height: Some(1.5),
        };
        // length->x, height->y, width->z.
        let g = osi_dimension_to_godot(&d, &m);
        assert!(close(g, Vector3::new(4.5, 1.5, 1.8)), "got {g:?}");
    }

    #[test]
    fn yaw_maps_to_godot_up_axis() {
        let m = AxisMapping::default();
        // OSI yaw of +90° about Z rotates the OSI forward x-axis to +y.
        // In Godot frame, the mapped forward axis (1,0,0) must rotate to the
        // mapped +y, i.e. (0,0,-1). B * (1,0,0) = first column of B.
        let o = osi3::Orientation3d {
            roll: Some(0.0),
            pitch: Some(0.0),
            yaw: Some(FRAC_PI_2),
        };
        let b = osi_orientation_to_basis(&o, &m);
        let forward = b * Vector3::new(1.0, 0.0, 0.0);
        assert!(close(forward, Vector3::new(0.0, 0.0, -1.0)), "got {forward:?}");
    }

    #[test]
    fn identity_orientation_is_identity_basis() {
        let m = AxisMapping::default();
        let o = osi3::Orientation3d {
            roll: Some(0.0),
            pitch: Some(0.0),
            yaw: Some(0.0),
        };
        let b = osi_orientation_to_basis(&o, &m);
        let i = Basis::IDENTITY;
        assert!(close(b.col_a(), i.col_a()));
        assert!(close(b.col_b(), i.col_b()));
        assert!(close(b.col_c(), i.col_c()));
    }

    #[test]
    fn base_moving_transform_combines_position_and_orientation() {
        let m = AxisMapping::default();
        let bm = osi3::BaseMoving {
            position: Some(vec(10.0, 5.0, 1.0)),
            orientation: Some(osi3::Orientation3d {
                roll: Some(0.0),
                pitch: Some(0.0),
                yaw: Some(0.0),
            }),
            ..Default::default()
        };
        let t = osi_base_moving_to_transform(&bm, &m);
        assert!(close(t.origin, Vector3::new(10.0, 1.0, -5.0)), "got {:?}", t.origin);
    }
}

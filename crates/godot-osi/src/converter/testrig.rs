//! Headless-test rig for the generated converter (compiled only with the
//! `itest` feature). The structural `convert_*` output can't be checked under
//! plain `cargo test` because building `Gd<Resource>` needs a running Godot, so
//! this exposes a hand-built sample frame to a headless Godot project that then
//! asserts on the converted snapshot. See `itest/`.

use godot::prelude::*;
use osi_types::osi3;

use crate::converter::generated::{convert_ground_truth, OsiGroundTruth};

#[derive(GodotClass)]
#[class(base=RefCounted, init)]
pub struct OsiTestRig {
    base: Base<RefCounted>,
}

#[godot_api]
impl OsiTestRig {
    /// Build a deterministic `osi3::GroundTruth` (host id 7, one MovingObject
    /// id 42 at raw OSI position (10, 5, 1), dimension (4.5, 1.8, 1.5)) and
    /// return the converted typed snapshot. GDScript asserts the *raw* values
    /// are preserved 1:1 (the "A" geometry policy — no coordinate transform in
    /// the Resource mirror).
    #[func]
    fn make_sample_ground_truth(&self) -> Gd<OsiGroundTruth> {
        let gt = osi3::GroundTruth {
            host_vehicle_id: Some(osi3::Identifier { value: Some(7) }),
            moving_object: vec![osi3::MovingObject {
                id: Some(osi3::Identifier { value: Some(42) }),
                base: Some(osi3::BaseMoving {
                    position: Some(osi3::Vector3d {
                        x: Some(10.0),
                        y: Some(5.0),
                        z: Some(1.0),
                    }),
                    dimension: Some(osi3::Dimension3d {
                        length: Some(4.5),
                        width: Some(1.8),
                        height: Some(1.5),
                    }),
                    orientation: Some(osi3::Orientation3d {
                        roll: Some(0.0),
                        pitch: Some(0.0),
                        yaw: Some(0.0),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        convert_ground_truth(&gt)
    }
}

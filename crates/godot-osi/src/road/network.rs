//! `OsiRoadNetwork`: a `Resource` wrapping a loaded OpenDRIVE map.
//!
//! Holds the engine-agnostic [`esmini_rm::OdrMap`] and exposes load + a few
//! logical queries to Godot. The heavy geometry sampling lives in
//! [`super::mesh`]; this type just owns the map and converts query results into
//! Godot coordinates via [`crate::converter::coords`].

use esmini_rm::{OdrMap, RM_ID_UNDEFINED};
use godot::prelude::*;

use crate::converter::coords::{self, AxisMapping};

#[derive(GodotClass)]
#[class(base=Resource, init)]
pub struct OsiRoadNetwork {
    /// The loaded road network, or `None` before a successful `load`.
    map: Option<OdrMap>,
    base: Base<Resource>,
}

#[godot_api]
impl OsiRoadNetwork {
    /// Load an OpenDRIVE file. Accepts a `res://`/`user://` path (resolved to an
    /// OS path) or a plain filesystem path. Returns `true` on success.
    ///
    /// Only one network can be loaded process-wide (the underlying library is a
    /// global singleton), so any previously loaded map is released first.
    #[func]
    fn load(&mut self, path: GString) -> bool {
        // Release any existing map before loading another (singleton).
        self.map = None;

        let os_path = godot::classes::ProjectSettings::singleton()
            .globalize_path(&path)
            .to_string();

        match OdrMap::load(&os_path) {
            Ok(map) => {
                godot_print!(
                    "[OsiRoadNetwork] loaded {os_path} ({} roads)",
                    map.road_count()
                );
                self.map = Some(map);
                true
            }
            Err(e) => {
                godot_error!("[OsiRoadNetwork] failed to load {os_path}: {e}");
                false
            }
        }
    }

    /// Whether a road network is currently loaded.
    #[func]
    fn is_loaded(&self) -> bool {
        self.map.is_some()
    }

    /// Number of roads in the loaded network (0 if none loaded).
    #[func]
    fn road_count(&self) -> i64 {
        self.map.as_ref().map(|m| m.road_count() as i64).unwrap_or(0)
    }

    /// World position (Godot space) at road (`road_id`, `s`, `t`).
    #[func]
    fn world_position(&self, road_id: i64, s: f64, t: f64) -> Vector3 {
        let Some(map) = self.map.as_ref() else {
            return Vector3::ZERO;
        };
        match map.world_position(road_id as u32, s, t) {
            Some(p) => to_godot(p, &AxisMapping::default()),
            None => Vector3::ZERO,
        }
    }

    /// Road ID at `index` (0-based), or -1 if none/out of range.
    #[func]
    fn road_id_at(&self, index: i64) -> i64 {
        match self.map.as_ref() {
            Some(m) => {
                let id = m.road_id_at(index as i32);
                if id == RM_ID_UNDEFINED {
                    -1
                } else {
                    id as i64
                }
            }
            None => -1,
        }
    }

    /// Length (m) of the road with `road_id` (0.0 if unknown).
    #[func]
    fn road_length(&self, road_id: i64) -> f64 {
        self.map.as_ref().map(|m| m.road_length(road_id as u32)).unwrap_or(0.0)
    }

    /// Drivable lane IDs on `road_id` at distance `s`.
    #[func]
    fn drivable_lanes(&self, road_id: i64, s: f64) -> PackedInt32Array {
        let mut out = PackedInt32Array::new();
        if let Some(m) = self.map.as_ref() {
            for id in m.drivable_lanes(road_id as u32, s) {
                out.push(id);
            }
        }
        out
    }

    /// World position (Godot space) of the center of `lane_id` on `road_id` at `s`.
    #[func]
    fn lane_point(&self, road_id: i64, lane_id: i64, s: f64) -> Vector3 {
        let Some(map) = self.map.as_ref() else {
            return Vector3::ZERO;
        };
        match map.lane_center(road_id as u32, lane_id as i32, s) {
            Some(p) => to_godot(p, &AxisMapping::default()),
            None => Vector3::ZERO,
        }
    }

    /// Number of road signs across all roads.
    #[func]
    fn sign_count(&self) -> i64 {
        self.map.as_ref().map(|m| m.signs().len() as i64).unwrap_or(0)
    }

    /// Sign positions in Godot space (handy for GDScript-side placement).
    #[func]
    fn sign_positions(&self) -> PackedVector3Array {
        let mapping = AxisMapping::default();
        let mut out = PackedVector3Array::new();
        if let Some(map) = self.map.as_ref() {
            for sign in map.signs() {
                out.push(to_godot([sign.x, sign.y, sign.z], &mapping));
            }
        }
        out
    }

    // === Extended RoadManager queries (Dictionary/Array surface) =============
    //
    // These mirror the `esmini_rm::OdrMap` methods, returning Godot
    // `Dictionary`/`Array` so GDScript can read the full OpenDRIVE structure.
    // Positions carry both raw OpenDRIVE fields and a `pos` Vector3 in Godot
    // space. Scalar getters return `NAN` when the value is unavailable.

    /// OSI-point selector constants for [`Self::lane_osi_points`].
    #[constant]
    const OSI_LANE: i32 = esmini_rm::osi_point_kind::LANE;
    #[constant]
    const OSI_REF_LINE: i32 = esmini_rm::osi_point_kind::REF_LINE;
    #[constant]
    const OSI_BOUNDARY: i32 = esmini_rm::osi_point_kind::BOUNDARY;
    /// Road-link `link_type` constants for [`Self::road_link`].
    #[constant]
    const LINK_PREDECESSOR: i32 = esmini_rm::link::PREDECESSOR;
    #[constant]
    const LINK_SUCCESSOR: i32 = esmini_rm::link::SUCCESSOR;

    /// Reference-line geometry primitives of `road_id` (one Dictionary each).
    #[func]
    fn geometries(&self, road_id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for g in m.geometries(road_id as u32) {
                out.push(&vdict! {
                    "type" => g.geom_type as i64,
                    "s" => g.s, "x" => g.x, "y" => g.y, "hdg" => g.hdg, "length" => g.length,
                    "curv_start" => g.curv_start, "curv_end" => g.curv_end,
                    "a" => g.a, "b" => g.b, "c" => g.c, "d" => g.d,
                    "a2" => g.a2, "b2" => g.b2, "c2" => g.c2, "d2" => g.d2,
                });
            }
        }
        out
    }

    /// Precomputed OSI sample points of a lane / reference line / boundary.
    /// `kind` is one of `OSI_LANE` / `OSI_REF_LINE` / `OSI_BOUNDARY`.
    #[func]
    fn lane_osi_points(&self, road_id: i64, section_idx: i64, lane_id: i64, kind: i64) -> Array<VarDictionary> {
        let mapping = AxisMapping::default();
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for p in m.lane_osi_points(road_id as u32, section_idx as u32, lane_id as i32, kind as i32) {
                out.push(&vdict! {
                    "pos" => to_godot([p.x, p.y, p.z], &mapping),
                    "s" => p.s, "x" => p.x, "y" => p.y, "z" => p.z,
                    "h" => p.h, "p" => p.p, "r" => p.r, "nx" => p.nx, "ny" => p.ny,
                    "endpoint" => p.endpoint,
                });
            }
        }
        out
    }

    /// Lane sections of `road_id` (one Dictionary each).
    #[func]
    fn lane_sections(&self, road_id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for (i, s) in m.lane_sections(road_id as u32).into_iter().enumerate() {
                out.push(&vdict! {
                    "index" => i as i64,
                    "s" => s.s, "length" => s.length, "n_lanes" => s.n_lanes as i64,
                });
            }
        }
        out
    }

    /// Lanes of section `section_idx` on `road_id` (one Dictionary each).
    #[func]
    fn lanes(&self, road_id: i64, section_idx: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for l in m.lanes(road_id as u32, section_idx as u32) {
                out.push(&vdict! {
                    "lane_id" => l.lane_id as i64,
                    "lane_type" => l.lane_type as i64,
                    "global_id" => l.global_id as i64,
                    "is_road_edge" => l.is_road_edge,
                    "predecessor" => l.pred_lane_id.map(|v| v as i64).unwrap_or(i64::MIN),
                    "successor" => l.succ_lane_id.map(|v| v as i64).unwrap_or(i64::MIN),
                });
            }
        }
        out
    }

    /// Lateral center offset (m) of `lane_id` at road `s` (`NAN` on error).
    #[func]
    fn lane_center_offset(&self, road_id: i64, lane_id: i64, s: f64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.lane_center_offset(road_id as u32, lane_id as i32, s))
            .unwrap_or(f64::NAN)
    }

    /// Friction of `lane_id`'s material at road `s` (`NAN` if undefined).
    #[func]
    fn lane_friction(&self, road_id: i64, lane_id: i64, s: f64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.lane_friction(road_id as u32, lane_id as i32, s))
            .unwrap_or(f64::NAN)
    }

    /// `<roadMark>` style records on lane `lane_id` of section `section_idx`.
    #[func]
    fn road_marks(&self, road_id: i64, section_idx: i64, lane_id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for r in m.road_mark_meta(road_id as u32, section_idx as u32, lane_id as i32) {
                out.push(&vdict! {
                    "type" => r.mark_type as i64,
                    "weight" => r.weight as i64,
                    "color" => r.color as i64,
                    "material" => r.material as i64,
                    "lane_change" => r.lane_change as i64,
                    "width" => r.width, "height" => r.height,
                    "s_offset" => r.s_offset, "fade" => r.fade,
                });
            }
        }
        out
    }

    /// `<object>` records on `road_id` (barriers, poles, trees, ...).
    #[func]
    fn road_objects(&self, road_id: i64) -> Array<VarDictionary> {
        let mapping = AxisMapping::default();
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for (i, o) in m.road_objects(road_id as u32).into_iter().enumerate() {
                out.push(&vdict! {
                    "index" => i as i64,
                    "id" => o.id as i64,
                    "global_id" => o.global_id as i64,
                    "type" => o.obj_type as i64,
                    "type_name" => o.type_name.as_str(),
                    "name" => o.name.as_str(),
                    "orientation" => o.orientation as i64,
                    "pos" => to_godot([o.x, o.y, o.z], &mapping),
                    "s" => o.s, "t" => o.t,
                    "z_offset" => o.z_offset, "h_offset" => o.h_offset,
                    "pitch" => o.pitch, "roll" => o.roll, "heading" => o.heading,
                    "length" => o.length, "width" => o.width, "height" => o.height,
                    "parking_access" => o.parking_access.map(|v| v as i64).unwrap_or(-1),
                    "n_outlines" => o.n_outlines as i64,
                    "n_repeats" => o.n_repeats as i64,
                });
            }
        }
        out
    }

    /// Metadata of outline `outline_idx` of object `obj_idx` (empty if none).
    #[func]
    fn object_outline_info(&self, road_id: i64, obj_idx: i64, outline_idx: i64) -> VarDictionary {
        let Some(o) = self
            .map
            .as_ref()
            .and_then(|m| m.object_outline_info(road_id as u32, obj_idx as u32, outline_idx as u32))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "id" => o.id as i64,
            "fill_type" => o.fill_type as i64,
            "contour_type" => o.contour_type as i64,
            "closed" => o.closed,
            "roof" => o.roof,
            "n_corners" => o.n_corners as i64,
        }
    }

    /// World-space outline corners of object `obj_idx`, each with its outline index.
    #[func]
    fn object_outline_corners(&self, road_id: i64, obj_idx: i64) -> Array<VarDictionary> {
        let mapping = AxisMapping::default();
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for (p, idx) in m.object_outline_corners(road_id as u32, obj_idx as u32) {
                out.push(&vdict! {
                    "pos" => to_godot(p, &mapping),
                    "outline_index" => idx as i64,
                });
            }
        }
        out
    }

    /// `<tunnel>` records on `road_id`.
    #[func]
    fn tunnels(&self, road_id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for t in m.tunnels(road_id as u32) {
                out.push(&vdict! {
                    "id" => t.id as i64,
                    "type" => t.tunnel_type as i64,
                    "name" => t.name.as_str(),
                    "s" => t.s, "length" => t.length, "width" => t.width,
                    "lighting" => t.lighting, "daylight" => t.daylight,
                });
            }
        }
        out
    }

    /// Predecessor (`LINK_PREDECESSOR`) or successor (`LINK_SUCCESSOR`) link of
    /// `road_id` (empty Dictionary if there is none).
    #[func]
    fn road_link(&self, road_id: i64, link_type: i64) -> VarDictionary {
        let Some(l) = self
            .map
            .as_ref()
            .and_then(|m| m.road_link(road_id as u32, link_type as i32))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "element_type" => l.element_type as i64,
            "element_id" => l.element_id as i64,
            "contact_point" => l.contact_point as i64,
        }
    }

    /// All junctions in the network.
    #[func]
    fn junctions(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for j in m.junctions() {
                out.push(&vdict! {
                    "id" => j.id as i64,
                    "global_id" => j.global_id as i64,
                    "type" => j.junction_type as i64,
                    "name" => j.name.as_str(),
                    "n_connections" => j.n_connections as i64,
                    "n_controllers" => j.n_controllers as i64,
                });
            }
        }
        out
    }

    /// Connections of junction `junction_id`.
    #[func]
    fn junction_connections(&self, junction_id: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for c in m.junction_connections(junction_id as u32) {
                out.push(&vdict! {
                    "incoming_road_id" => c.incoming_road_id as i64,
                    "connecting_road_id" => c.connecting_road_id as i64,
                    "contact_point" => c.contact_point as i64,
                    "n_lane_links" => c.n_lane_links as i64,
                });
            }
        }
        out
    }

    /// Incoming->connecting lane id pairs of connection `conn_idx` in `junction_id`.
    #[func]
    fn junction_lane_links(&self, junction_id: i64, conn_idx: i64) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for (from, to) in m.junction_lane_links(junction_id as u32, conn_idx as u32) {
                out.push(&vdict! { "from" => from as i64, "to" => to as i64 });
            }
        }
        out
    }

    /// All network controllers.
    #[func]
    fn controllers(&self) -> Array<VarDictionary> {
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for c in m.controllers() {
                out.push(&vdict! {
                    "id" => c.id as i64,
                    "sequence" => c.sequence as i64,
                    "name" => c.name.as_str(),
                    "n_controls" => c.n_controls as i64,
                });
            }
        }
        out
    }

    /// Full `<signal>` detail across all roads (richer than [`Self::sign_positions`]).
    #[func]
    fn signals(&self) -> Array<VarDictionary> {
        let mapping = AxisMapping::default();
        let mut out = Array::new();
        if let Some(m) = self.map.as_ref() {
            for s in m.signals() {
                out.push(&vdict! {
                    "road_id" => s.road_id as i64,
                    "id" => s.id as i64,
                    "global_id" => s.global_id as i64,
                    "osi_type" => s.osi_type as i64,
                    "orientation" => s.orientation as i64,
                    "dynamic" => s.dynamic,
                    "pos" => to_godot([s.x, s.y, s.z], &mapping),
                    "s" => s.s, "t" => s.t,
                    "z_offset" => s.z_offset, "h_offset" => s.h_offset,
                    "pitch" => s.pitch, "roll" => s.roll, "heading" => s.heading,
                    "height" => s.height, "width" => s.width, "depth" => s.depth, "length" => s.length,
                    "value" => s.value,
                    "name" => s.name.as_str(),
                    "type" => s.sign_type.as_str(),
                    "subtype" => s.subtype.as_str(),
                    "country" => s.country.as_str(),
                    "value_str" => s.value_str.as_str(),
                    "unit" => s.unit.as_str(),
                    "text" => s.text.as_str(),
                });
            }
        }
        out
    }

    /// Elevation profile entries of `road_id` (cubic `a..d` from `s`).
    #[func]
    fn elevations(&self, road_id: i64) -> Array<VarDictionary> {
        self.elevation_dicts(self.map.as_ref().map(|m| m.elevations(road_id as u32)))
    }

    /// Super-elevation (cross-slope) profile entries of `road_id`.
    #[func]
    fn super_elevations(&self, road_id: i64) -> Array<VarDictionary> {
        self.elevation_dicts(self.map.as_ref().map(|m| m.super_elevations(road_id as u32)))
    }

    /// Lateral lane offset of the reference line at road `s` (`NAN` on error).
    #[func]
    fn lane_offset(&self, road_id: i64, s: f64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.lane_offset(road_id as u32, s))
            .unwrap_or(f64::NAN)
    }

    /// Traffic rule of `road_id` (0 RHT, 1 LHT; -1 on error).
    #[func]
    fn road_rule(&self, road_id: i64) -> i64 {
        self.map
            .as_ref()
            .and_then(|m| m.road_rule(road_id as u32))
            .map(|v| v as i64)
            .unwrap_or(-1)
    }

    /// OpenDRIVE road type at road `s` (-1 on error).
    #[func]
    fn road_type(&self, road_id: i64, s: f64) -> i64 {
        self.map
            .as_ref()
            .and_then(|m| m.road_type(road_id as u32, s))
            .map(|v| v as i64)
            .unwrap_or(-1)
    }

    /// Speed (m/s) from the road-type element at `s` (`NAN` on error).
    #[func]
    fn road_speed(&self, road_id: i64, s: f64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.road_speed(road_id as u32, s))
            .unwrap_or(f64::NAN)
    }

    /// Width (m) of `road_id` at `s` on `side` (-1 right, 1 left, 0 both; `NAN` on error).
    #[func]
    fn road_width(&self, road_id: i64, s: f64, side: i64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.road_width(road_id as u32, s, side as i32))
            .unwrap_or(f64::NAN)
    }

    /// Network metadata: version, speed unit, friction (empty if unloaded).
    #[func]
    fn network_info(&self) -> VarDictionary {
        let Some(n) = self.map.as_ref().and_then(|m| m.network_info()) else {
            return VarDictionary::new();
        };
        vdict! {
            "version_major" => n.version_major as i64,
            "version_minor" => n.version_minor as i64,
            "speed_unit" => n.speed_unit as i64,
            "friction" => n.friction,
        }
    }

    /// The network geo offset (OSI 3.7.0; empty if unloaded).
    #[func]
    fn geo_offset(&self) -> VarDictionary {
        let Some(g) = self.map.as_ref().and_then(|m| m.geo_offset()) else {
            return VarDictionary::new();
        };
        vdict! { "x" => g.x, "y" => g.y, "z" => g.z, "hdg" => g.hdg }
    }

    /// Shortest-path distance (m) between two road positions (`NAN` if no path).
    #[func]
    fn shortest_path_distance(&self, road_a: i64, s_a: f64, road_b: i64, s_b: f64) -> f64 {
        self.map
            .as_ref()
            .and_then(|m| m.shortest_path_distance(road_a as u32, s_a, road_b as u32, s_b))
            .unwrap_or(f64::NAN)
    }

    /// Engine-internal access to the loaded map (used by the visualizer/mesh).
    pub fn map(&self) -> Option<&OdrMap> {
        self.map.as_ref()
    }

    /// Shared conversion of an elevation list into Godot Dictionaries.
    fn elevation_dicts(&self, list: Option<Vec<esmini_rm::Elevation>>) -> Array<VarDictionary> {
        let mut out = Array::new();
        for e in list.into_iter().flatten() {
            out.push(&vdict! {
                "s" => e.s, "length" => e.length,
                "a" => e.a, "b" => e.b, "c" => e.c, "d" => e.d,
            });
        }
        out
    }
}

/// Convert an esmini world point (OpenDRIVE/OSI frame) into Godot space.
pub(super) fn to_godot(p: [f64; 3], mapping: &AxisMapping) -> Vector3 {
    coords::osi_position_to_godot(
        &osi_types::osi3::Vector3d {
            x: Some(p[0]),
            y: Some(p[1]),
            z: Some(p[2]),
        },
        mapping,
    )
}

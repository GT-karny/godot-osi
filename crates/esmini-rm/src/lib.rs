//! Safe, engine-agnostic Rust wrapper over esmini's RoadManager C API.
//!
//! Loads an OpenDRIVE (`.xodr`) file and exposes road/lane/sign queries plus a
//! road-surface sampler for mesh generation. No Godot dependency — `godot-osi`
//! wraps this in `Resource`/`Node3D` classes.
//!
//! **Global singleton**: esminiRMLib keeps a single global road network, so only
//! one [`OdrMap`] may exist at a time. [`OdrMap::load`] returns an error if one
//! is already loaded; dropping it unloads the network.

mod ffi;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub use ffi::{IdT, RM_ID_UNDEFINED};

/// Lane half-widths below this (m) are treated as "lane absent here".
const WIDTH_EPS: f64 = 1e-3;
/// `type_mask` value meaning "any lane type" in the RM API.
const LANE_TYPE_ANY: i32 = -1;

/// Enforces the single-loaded-network invariant of the underlying C library.
static LOADED: AtomicBool = AtomicBool::new(false);

/// Copy a library-owned C string into an owned `String` (null → empty).
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string that stays alive for
/// the duration of the call (true while the map is loaded).
unsafe fn cstr_to_string(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

/// A road sign, with world position and metadata, copied out of the library.
#[derive(Clone, Debug, Default)]
pub struct RoadSign {
    pub id: i32,
    pub road_id: IdT,
    /// Sign name — esmini conventionally uses this as a 3D model filename.
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub z_offset: f64,
    pub heading: f64,
    pub s: f64,
    pub t: f64,
    /// 1 = faces traffic in road direction, -1 = opposite.
    pub orientation: i32,
    pub length: f64,
    pub height: f64,
    pub width: f64,
}

/// Road-mark geometry as a flat triangle soup with a per-vertex color.
///
/// `verts.len()` is a multiple of 3 (triangles); `colors[i]` is the OpenDRIVE
/// `RoadMarkColor` for vertex `i` (see [`road_mark_color_name`]).
#[derive(Clone, Debug, Default)]
pub struct RoadMarks {
    pub verts: Vec<[f64; 3]>,
    pub colors: Vec<i32>,
}

/// OpenDRIVE `RoadMarkColor` enum values (RoadManager.hpp), for interpreting
/// [`RoadMarks::colors`].
pub mod road_mark_color {
    pub const UNDEFINED: i32 = 0;
    pub const BLACK: i32 = 1;
    pub const BLUE: i32 = 2;
    pub const GREEN: i32 = 3;
    pub const ORANGE: i32 = 4;
    pub const RED: i32 = 5;
    pub const STANDARD: i32 = 6; // white
    pub const VIOLET: i32 = 7;
    pub const WHITE: i32 = 8;
    pub const YELLOW: i32 = 9;
}

/// OpenDRIVE `RoadMarkType` enum values (RoadManager.hpp), for [`RoadMark::mark_type`].
pub mod road_mark_type {
    pub const NONE: i32 = 1;
    pub const SOLID: i32 = 2;
    pub const BROKEN: i32 = 3;
    pub const SOLID_SOLID: i32 = 4;
    pub const SOLID_BROKEN: i32 = 5;
    pub const BROKEN_SOLID: i32 = 6;
    pub const BROKEN_BROKEN: i32 = 7;
    pub const BOTTS_DOTS: i32 = 8;
    pub const GRASS: i32 = 9;
    pub const CURB: i32 = 10;
}

/// `RoadMarkWeight` (0 = standard, 1 = bold) and `RoadMarkLaneChange`
/// (0 = increase, 1 = decrease, 2 = both, 3 = none) enum values.
pub mod road_mark {
    pub const WEIGHT_STANDARD: i32 = 0;
    pub const WEIGHT_BOLD: i32 = 1;
    pub const LANE_CHANGE_INCREASE: i32 = 0;
    pub const LANE_CHANGE_DECREASE: i32 = 1;
    pub const LANE_CHANGE_BOTH: i32 = 2;
    pub const LANE_CHANGE_NONE: i32 = 3;
}

/// One quad of road surface: a ring of four world-space corners
/// (`s0` inner, `s0` outer, `s1` outer, `s1` inner).
#[derive(Clone, Copy, Debug)]
pub struct RoadQuad {
    /// OpenDRIVE lane type (see RoadManager::Lane::LaneType).
    pub lane_type: i32,
    pub corners: [[f64; 3]; 4],
}

/// OpenDRIVE reference-line `<geometry>` primitive type
/// (`roadmanager::Geometry::GeometryType`), for [`Geometry::geom_type`].
pub mod geometry_type {
    pub const UNKNOWN: i32 = 0;
    pub const LINE: i32 = 1;
    pub const ARC: i32 = 2;
    pub const SPIRAL: i32 = 3;
    pub const POLY3: i32 = 4;
    pub const PARAM_POLY3: i32 = 5;
}

/// OpenDRIVE road-object type (`RMObject::ObjectType`), for [`RoadObject::obj_type`].
pub mod object_type {
    pub const BARRIER: i32 = 0;
    pub const BIKE: i32 = 1;
    pub const BUILDING: i32 = 2;
    pub const BUS: i32 = 3;
    pub const CAR: i32 = 4;
    pub const CROSSWALK: i32 = 5;
    pub const GANTRY: i32 = 6;
    pub const MOTORBIKE: i32 = 7;
    pub const NONE: i32 = 8;
    pub const OBSTACLE: i32 = 9;
    pub const PARKINGSPACE: i32 = 10;
    pub const PATCH: i32 = 11;
    pub const PEDESTRIAN: i32 = 12;
    pub const POLE: i32 = 13;
    pub const RAILING: i32 = 14;
    pub const ROADMARK: i32 = 15;
    pub const SOUNDBARRIER: i32 = 16;
    pub const STREETLAMP: i32 = 17;
    pub const TRAFFICISLAND: i32 = 18;
    pub const TRAILER: i32 = 19;
    pub const TRAIN: i32 = 20;
    pub const TRAM: i32 = 21;
    pub const TREE: i32 = 22;
    pub const VAN: i32 = 23;
    pub const VEGETATION: i32 = 24;
    pub const WIND: i32 = 25;
    pub const BRIDGE: i32 = 26;
}

/// Road-link element type (`RoadLink::ElementType`) for [`RoadLink::element_type`],
/// and contact-point type (`ContactPointType`) for link/connection contact points.
pub mod link {
    pub const ELEMENT_UNKNOWN: i32 = 0;
    pub const ELEMENT_ROAD: i32 = 1;
    pub const ELEMENT_JUNCTION: i32 = 2;
    pub const CONTACT_UNDEFINED: i32 = 0;
    pub const CONTACT_START: i32 = 1;
    pub const CONTACT_END: i32 = 2;
    pub const CONTACT_JUNCTION: i32 = 3;
    /// `link_type` argument values for predecessor/successor queries.
    pub const PREDECESSOR: i32 = -1;
    pub const SUCCESSOR: i32 = 1;
}

/// Junction type (`Junction::JunctionType`) for [`Junction::junction_type`].
pub mod junction_type {
    pub const DEFAULT: i32 = 0;
    pub const DIRECT: i32 = 1;
    pub const VIRTUAL: i32 = 2;
}

/// Traffic rule from [`OdrMap::road_rule`] (`roadmanager::Road::RoadRule`).
pub mod road_rule {
    pub const RIGHT_HAND_TRAFFIC: i32 = 0;
    pub const LEFT_HAND_TRAFFIC: i32 = 1;
}

/// OpenDRIVE road type (`roadmanager::Road::RoadType`) from [`OdrMap::road_type`].
pub mod road_type {
    pub const UNKNOWN: i32 = 0;
    pub const RURAL: i32 = 1;
    pub const MOTORWAY: i32 = 2;
    pub const TOWN: i32 = 3;
    pub const LOWSPEED: i32 = 4;
    pub const PEDESTRIAN: i32 = 5;
    pub const BICYCLE: i32 = 6;
    pub const TOWNARTERIAL: i32 = 7;
    pub const TOWNCOLLECTOR: i32 = 8;
    pub const TOWNEXPRESSWAY: i32 = 9;
    pub const TOWNLOCAL: i32 = 10;
    pub const TOWNPLAYSTREET: i32 = 11;
    pub const TOWNPRIVATE: i32 = 12;
}

/// Speed unit (`roadmanager::SpeedUnit`) in [`NetworkInfo::speed_unit`].
pub mod speed_unit {
    pub const UNDEFINED: i32 = 0;
    pub const KMH: i32 = 1;
    pub const MS: i32 = 2;
    pub const MPH: i32 = 3;
}

/// Which precomputed OSI point set to read in [`OdrMap::lane_osi_points`].
pub mod osi_point_kind {
    /// The lane's own OSI points (its outer edge).
    pub const LANE: i32 = 0;
    /// The lane section's reference-line OSI points (lane id ignored).
    pub const REF_LINE: i32 = 1;
    /// The lane's OSI lane-boundary points.
    pub const BOUNDARY: i32 = 2;
}

/// One reference-line geometry primitive. `curv_*` apply to arc/spiral; `a..d`
/// are poly3 coefficients (or paramPoly3 U), `a2..d2` are paramPoly3 V.
#[derive(Clone, Copy, Debug, Default)]
pub struct Geometry {
    pub road_id: IdT,
    /// See [`geometry_type`].
    pub geom_type: i32,
    pub s: f64,
    pub x: f64,
    pub y: f64,
    pub hdg: f64,
    pub length: f64,
    pub curv_start: f64,
    pub curv_end: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub a2: f64,
    pub b2: f64,
    pub c2: f64,
    pub d2: f64,
}

/// One precomputed OSI sample point, with orientation and an `endpoint` flag
/// marking the end of a contiguous run (e.g. a dash of a broken road mark).
#[derive(Clone, Copy, Debug, Default)]
pub struct OsiPoint {
    pub s: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub h: f64,
    pub p: f64,
    pub r: f64,
    pub nx: f64,
    pub ny: f64,
    pub endpoint: bool,
}

/// Style metadata of one `<roadMark>` on a lane. See [`road_mark_type`],
/// [`road_mark_color`], [`road_mark`] for the enum constants.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoadMark {
    pub road_id: IdT,
    pub section_idx: u32,
    pub lane_id: i32,
    pub mark_type: i32,
    pub weight: i32,
    pub color: i32,
    pub material: i32,
    pub lane_change: i32,
    pub width: f64,
    pub height: f64,
    pub s_offset: f64,
    pub fade: f64,
}

/// Full `<signal>` detail. `osi_type` is the raw `Signal::OSIType`; `orientation`
/// is 0 positive / 1 negative / 2 none. World pose in `x`/`y`/`z`/`heading`.
#[derive(Clone, Debug, Default)]
pub struct Signal {
    pub road_id: IdT,
    pub id: i32,
    pub global_id: IdT,
    pub osi_type: i32,
    pub orientation: i32,
    pub dynamic: bool,
    pub s: f64,
    pub t: f64,
    pub z_offset: f64,
    pub h_offset: f64,
    pub pitch: f64,
    pub roll: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub heading: f64,
    pub height: f64,
    pub width: f64,
    pub depth: f64,
    pub length: f64,
    pub value: f64,
    pub name: String,
    pub sign_type: String,
    pub subtype: String,
    pub country: String,
    pub value_str: String,
    pub unit: String,
    pub text: String,
}

/// One elevation / super-elevation profile entry: a cubic `a..d` valid from `s`
/// over `length` meters (evaluated in local `ds` from `s`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Elevation {
    pub road_id: IdT,
    pub s: f64,
    pub length: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

/// Network metadata. `speed_unit` is a [`speed_unit`] value.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetworkInfo {
    pub version_major: i32,
    pub version_minor: i32,
    pub speed_unit: i32,
    pub friction: f64,
}

/// The network geo offset (OSI 3.7.0).
#[derive(Clone, Copy, Debug, Default)]
pub struct GeoOffset {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub hdg: f64,
}

/// A road predecessor/successor link. `element_type` is a [`link`] element
/// constant; `contact_point` a [`link`] contact constant.
#[derive(Clone, Copy, Debug, Default)]
pub struct RoadLink {
    pub element_type: i32,
    pub element_id: IdT,
    pub contact_point: i32,
}

/// A junction. `junction_type` is a [`junction_type`] constant.
#[derive(Clone, Debug, Default)]
pub struct Junction {
    pub id: IdT,
    pub global_id: IdT,
    pub junction_type: i32,
    pub name: String,
    pub n_connections: i32,
    pub n_controllers: i32,
}

/// One connection within a junction.
#[derive(Clone, Copy, Debug, Default)]
pub struct JunctionConnection {
    pub incoming_road_id: IdT,
    pub connecting_road_id: IdT,
    pub contact_point: i32,
    pub n_lane_links: i32,
}

/// A network controller (`<controller>`).
#[derive(Clone, Debug, Default)]
pub struct Controller {
    pub id: IdT,
    pub sequence: i32,
    pub name: String,
    pub n_controls: i32,
}

/// One `<object>` on a road (barrier, pole, tree, building, parking space, ...).
/// `obj_type` is an [`object_type`] value; `orientation` is 0 positive / 1
/// negative / 2 none; `parking_access` is `None` unless this is a parking space.
#[derive(Clone, Debug, Default)]
pub struct RoadObject {
    pub road_id: IdT,
    pub id: IdT,
    pub global_id: IdT,
    pub obj_type: i32,
    pub type_name: String,
    pub name: String,
    pub orientation: i32,
    pub s: f64,
    pub t: f64,
    pub z_offset: f64,
    pub h_offset: f64,
    pub pitch: f64,
    pub roll: f64,
    /// World position and heading.
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub heading: f64,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub parking_access: Option<i32>,
    pub n_outlines: i32,
    pub n_repeats: i32,
}

/// Metadata of one `<outline>` of a road object. `fill_type` is
/// `Outline::FillType`, `contour_type` is `Outline::ContourType`.
#[derive(Clone, Copy, Debug, Default)]
pub struct OutlineInfo {
    pub id: IdT,
    pub fill_type: i32,
    pub contour_type: i32,
    pub closed: bool,
    pub roof: bool,
    pub n_corners: i32,
}

/// One `<tunnel>` on a road. `tunnel_type` is 0 standard / 1 underpass.
#[derive(Clone, Debug, Default)]
pub struct Tunnel {
    pub road_id: IdT,
    pub id: IdT,
    pub tunnel_type: i32,
    pub name: String,
    pub s: f64,
    pub length: f64,
    pub width: f64,
    pub lighting: f64,
    pub daylight: f64,
}

/// One `<laneSection>` of a road: its start `s`, `length` and lane count.
#[derive(Clone, Copy, Debug, Default)]
pub struct LaneSection {
    pub road_id: IdT,
    pub s: f64,
    pub length: f64,
    pub n_lanes: i32,
}

/// One lane within a section. `lane_type` is the `roadmanager::Lane::LaneType`
/// bitmask; `pred_lane_id`/`succ_lane_id` are `None` when no link exists.
#[derive(Clone, Copy, Debug, Default)]
pub struct Lane {
    pub road_id: IdT,
    pub section_idx: u32,
    pub lane_id: i32,
    pub lane_type: i32,
    pub global_id: IdT,
    pub is_road_edge: bool,
    pub pred_lane_id: Option<i32>,
    pub succ_lane_id: Option<i32>,
}

/// A loaded OpenDRIVE road network. Drop unloads it.
pub struct OdrMap {
    /// Reusable RM position handle (>= 0), used for all coordinate lookups.
    pos: i32,
    // Raw pointer marker: the underlying C state is global and not thread-safe,
    // so make the handle neither Send nor Sync.
    _not_sync: PhantomData<*const ()>,
}

impl OdrMap {
    /// Load an OpenDRIVE file. Fails if another [`OdrMap`] is already loaded or
    /// the file cannot be parsed.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        if LOADED.swap(true, Ordering::SeqCst) {
            return Err("an OdrMap is already loaded (esminiRMLib is a global singleton)".into());
        }
        // From here on, any early return must release the LOADED flag.
        let path = path.as_ref();
        let c_path = match CString::new(path.to_string_lossy().as_bytes()) {
            Ok(p) => p,
            Err(e) => {
                LOADED.store(false, Ordering::SeqCst);
                return Err(format!("invalid path: {e}"));
            }
        };

        // Disable esmini's logfile (keep the plugin from littering the cwd).
        let empty = CString::new("").unwrap();
        unsafe { ffi::RM_SetLogFilePath(empty.as_ptr()) };

        let rc = unsafe { ffi::RM_Init(c_path.as_ptr()) };
        if rc != 0 {
            LOADED.store(false, Ordering::SeqCst);
            return Err(format!("RM_Init failed (code {rc}) for {}", path.display()));
        }

        let pos = unsafe { ffi::RM_CreatePosition() };
        if pos < 0 {
            unsafe { ffi::RM_Close() };
            LOADED.store(false, Ordering::SeqCst);
            return Err("RM_CreatePosition failed".into());
        }

        Ok(Self {
            pos,
            _not_sync: PhantomData,
        })
    }

    /// Number of roads in the network.
    pub fn road_count(&self) -> i32 {
        unsafe { ffi::RM_GetNumberOfRoads() }.max(0)
    }

    /// Road ID at `index`, or [`RM_ID_UNDEFINED`] on error.
    pub fn road_id_at(&self, index: i32) -> IdT {
        if index < 0 {
            return RM_ID_UNDEFINED;
        }
        unsafe { ffi::RM_GetIdOfRoadFromIndex(index as u32) }
    }

    /// Length (m) of the road with `road_id`.
    pub fn road_length(&self, road_id: IdT) -> f64 {
        unsafe { ffi::RM_GetRoadLength(road_id) }
    }

    /// World position for road (s, t) coordinates, or `None` on error.
    pub fn world_position(&self, road_id: IdT, s: f64, t: f64) -> Option<[f64; 3]> {
        if unsafe { ffi::RM_SetRoadPosition(self.pos, road_id, s, t, false) } < 0 {
            return None;
        }
        self.position_world()
    }

    /// Drivable lane IDs on `road_id` at distance `s`.
    pub fn drivable_lanes(&self, road_id: IdT, s: f64) -> Vec<i32> {
        let n = unsafe { ffi::RM_GetRoadNumberOfDrivableLanes(road_id, s) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut lane_id: i32 = 0;
            if unsafe { ffi::RM_GetDrivableLaneIdByIndex(road_id, i, s, &mut lane_id) } == 0 {
                out.push(lane_id);
            }
        }
        out
    }

    /// World position of the center of `lane_id` on `road_id` at distance `s`.
    pub fn lane_center(&self, road_id: IdT, lane_id: i32, s: f64) -> Option<[f64; 3]> {
        self.lane_edge(road_id, lane_id, 0.0, s)
    }

    /// All road signs across all roads, with world coordinates and metadata.
    pub fn signs(&self) -> Vec<RoadSign> {
        let mut out = Vec::new();
        for ri in 0..self.road_count() {
            let road_id = self.road_id_at(ri);
            if road_id == RM_ID_UNDEFINED {
                continue;
            }
            let n = unsafe { ffi::RM_GetNumberOfRoadSigns(road_id) };
            for index in 0..n.max(0) {
                let mut raw = ffi::RM_RoadSign::default();
                if unsafe { ffi::RM_GetRoadSign(road_id, index as u32, &mut raw) } != 0 {
                    continue;
                }
                let name = unsafe { cstr_to_string(raw.name) };
                out.push(RoadSign {
                    id: raw.id,
                    road_id: raw.road_id,
                    name,
                    x: raw.x,
                    y: raw.y,
                    z: raw.z,
                    z_offset: raw.z_offset,
                    heading: raw.h,
                    s: raw.s,
                    t: raw.t,
                    orientation: raw.orientation,
                    length: raw.length,
                    height: raw.height,
                    width: raw.width,
                });
            }
        }
        out
    }

    /// Sample the road surface as a list of world-space quads, stepping `step`
    /// meters along each road. esmini computes elevation/superelevation/spiral
    /// geometry, so the returned corners are ready to triangulate.
    pub fn road_surface_quads(&self, step: f64) -> Vec<RoadQuad> {
        let step = step.max(0.1);
        let mut quads = Vec::new();

        for ri in 0..self.road_count() {
            let road_id = self.road_id_at(ri);
            if road_id == RM_ID_UNDEFINED {
                continue;
            }
            let len = self.road_length(road_id);
            if len <= 0.0 {
                continue;
            }

            // s stations: 0, step, 2*step, ..., len.
            let mut stations: Vec<f64> = Vec::new();
            let mut s = 0.0;
            while s < len {
                stations.push(s);
                s += step;
            }
            stations.push(len);

            for pair in stations.windows(2) {
                let (s0, s1) = (pair[0], pair[1]);
                let n = unsafe { ffi::RM_GetRoadNumberOfLanes(road_id, s0, LANE_TYPE_ANY) };
                for i in 0..n.max(0) {
                    let mut lane_id: i32 = 0;
                    if unsafe {
                        ffi::RM_GetLaneIdByIndex(road_id, i, s0, LANE_TYPE_ANY, &mut lane_id)
                    } < 0
                    {
                        continue;
                    }
                    if lane_id == 0 {
                        continue; // reference/center lane has no surface
                    }
                    let w0 = self.lane_width(road_id, lane_id, s0);
                    let w1 = self.lane_width(road_id, lane_id, s1);
                    if w0 <= WIDTH_EPS || w1 <= WIDTH_EPS {
                        continue; // lane absent at one of the stations
                    }
                    let lane_type = unsafe { ffi::RM_GetLaneTypeByRoadId(road_id, lane_id, s0) };

                    let (Some(a), Some(b), Some(c), Some(d)) = (
                        self.lane_edge(road_id, lane_id, -w0 / 2.0, s0),
                        self.lane_edge(road_id, lane_id, w0 / 2.0, s0),
                        self.lane_edge(road_id, lane_id, w1 / 2.0, s1),
                        self.lane_edge(road_id, lane_id, -w1 / 2.0, s1),
                    ) else {
                        continue;
                    };
                    quads.push(RoadQuad {
                        lane_type,
                        corners: [a, b, c, d],
                    });
                }
            }
        }
        quads
    }

    /// OpenDRIVE `<roadMark>` geometry (solid/broken painted strips with color)
    /// for the whole network, as a triangle soup. `z_offset` lifts the marks
    /// above the road surface (OpenDRIVE up/z meters). Reads RoadManager's
    /// precomputed per-mark OSI points via our C++ shim.
    pub fn road_marks(&self, z_offset: f64) -> RoadMarks {
        let verts_count = unsafe { ffi::GTRM_BuildRoadMarks(z_offset) }.max(0) as usize;
        if verts_count == 0 {
            unsafe { ffi::GTRM_ClearRoadMarks() };
            return RoadMarks::default();
        }
        let mut xyz = vec![0.0_f64; verts_count * 3];
        let mut colors = vec![0_i32; verts_count];
        unsafe { ffi::GTRM_CopyRoadMarks(xyz.as_mut_ptr(), colors.as_mut_ptr()) };
        unsafe { ffi::GTRM_ClearRoadMarks() };

        let verts = xyz.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
        RoadMarks { verts, colors }
    }

    /// Reference-line geometry primitives of `road_id`, in s-order.
    pub fn geometries(&self, road_id: IdT) -> Vec<Geometry> {
        let n = unsafe { ffi::GTRM_GetNumberOfGeometries(road_id) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut g = ffi::GTRM_Geometry::default();
            if unsafe { ffi::GTRM_GetGeometry(road_id, i as u32, &mut g) } != 0 {
                continue;
            }
            out.push(Geometry {
                road_id: g.road_id,
                geom_type: g.geom_type,
                s: g.s,
                x: g.x,
                y: g.y,
                hdg: g.hdg,
                length: g.length,
                curv_start: g.curv_start,
                curv_end: g.curv_end,
                a: g.a,
                b: g.b,
                c: g.c,
                d: g.d,
                a2: g.a2,
                b2: g.b2,
                c2: g.c2,
                d2: g.d2,
            });
        }
        out
    }

    /// Precomputed OSI sample points for one lane (or the lane-section
    /// reference line). `kind` selects the set; see [`osi_point_kind`].
    pub fn lane_osi_points(
        &self,
        road_id: IdT,
        section_idx: u32,
        lane_id: i32,
        kind: i32,
    ) -> Vec<OsiPoint> {
        let n =
            unsafe { ffi::GTRM_BuildLaneOsiPoints(road_id, section_idx, lane_id, kind) }.max(0) as usize;
        if n == 0 {
            unsafe { ffi::GTRM_ClearOsiPoints() };
            return Vec::new();
        }
        let mut buf = vec![ffi::GTRM_OsiPoint::default(); n];
        unsafe { ffi::GTRM_CopyOsiPoints(buf.as_mut_ptr()) };
        unsafe { ffi::GTRM_ClearOsiPoints() };
        buf.into_iter()
            .map(|p| OsiPoint {
                s: p.s,
                x: p.x,
                y: p.y,
                z: p.z,
                h: p.h,
                p: p.p,
                r: p.r,
                nx: p.nx,
                ny: p.ny,
                endpoint: p.endpoint != 0,
            })
            .collect()
    }

    /// Lane sections of `road_id`, in s-order.
    pub fn lane_sections(&self, road_id: IdT) -> Vec<LaneSection> {
        let n = unsafe { ffi::GTRM_GetNumberOfLaneSections(road_id) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut s = ffi::GTRM_LaneSection::default();
            if unsafe { ffi::GTRM_GetLaneSection(road_id, i as u32, &mut s) } == 0 {
                out.push(LaneSection {
                    road_id: s.road_id,
                    s: s.s,
                    length: s.length,
                    n_lanes: s.n_lanes,
                });
            }
        }
        out
    }

    /// All lanes of section `section_idx` on `road_id` (includes the center lane).
    pub fn lanes(&self, road_id: IdT, section_idx: u32) -> Vec<Lane> {
        let n = unsafe { ffi::GTRM_GetNumberOfLanesInSection(road_id, section_idx) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut l = ffi::GTRM_Lane::default();
            if unsafe { ffi::GTRM_GetLane(road_id, section_idx, i as u32, &mut l) } != 0 {
                continue;
            }
            out.push(Lane {
                road_id: l.road_id,
                section_idx: l.section_idx,
                lane_id: l.lane_id,
                lane_type: l.lane_type,
                global_id: l.global_id,
                is_road_edge: l.is_road_edge != 0,
                pred_lane_id: (l.has_pred != 0).then_some(l.pred_lane_id),
                succ_lane_id: (l.has_succ != 0).then_some(l.succ_lane_id),
            });
        }
        out
    }

    /// `<roadMark>` style records on lane `lane_id` of section `section_idx`.
    pub fn road_mark_meta(&self, road_id: IdT, section_idx: u32, lane_id: i32) -> Vec<RoadMark> {
        let n = unsafe { ffi::GTRM_GetNumberOfRoadMarks(road_id, section_idx, lane_id) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut m = ffi::GTRM_RoadMark::default();
            if unsafe { ffi::GTRM_GetRoadMark(road_id, section_idx, lane_id, i as u32, &mut m) } != 0
            {
                continue;
            }
            out.push(RoadMark {
                road_id: m.road_id,
                section_idx: m.section_idx,
                lane_id: m.lane_id,
                mark_type: m.mark_type,
                weight: m.weight,
                color: m.color,
                material: m.material,
                lane_change: m.lane_change,
                width: m.width,
                height: m.height,
                s_offset: m.s_offset,
                fade: m.fade,
            });
        }
        out
    }

    /// Lateral center offset (m) of `lane_id` from the road reference line at
    /// road distance `s`, or `None` on error.
    pub fn lane_center_offset(&self, road_id: IdT, lane_id: i32, s: f64) -> Option<f64> {
        let mut o = 0.0;
        (unsafe { ffi::GTRM_GetLaneCenterOffset(road_id, lane_id, s, &mut o) } == 0).then_some(o)
    }

    /// Friction of `lane_id`'s material at road distance `s`, or `None` if
    /// no material is defined.
    pub fn lane_friction(&self, road_id: IdT, lane_id: i32, s: f64) -> Option<f64> {
        let mut f = 0.0;
        (unsafe { ffi::GTRM_GetLaneFriction(road_id, lane_id, s, &mut f) } == 0).then_some(f)
    }

    /// All `<object>` records on `road_id`.
    pub fn road_objects(&self, road_id: IdT) -> Vec<RoadObject> {
        let n = unsafe { ffi::GTRM_GetNumberOfObjects(road_id) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut o = ffi::GTRM_RoadObject::default();
            if unsafe { ffi::GTRM_GetRoadObject(road_id, i as u32, &mut o) } != 0 {
                continue;
            }
            out.push(RoadObject {
                road_id: o.road_id,
                id: o.id,
                global_id: o.global_id,
                obj_type: o.obj_type,
                type_name: unsafe { cstr_to_string(o.type_str) },
                name: unsafe { cstr_to_string(o.name) },
                orientation: o.orientation,
                s: o.s,
                t: o.t,
                z_offset: o.z_offset,
                h_offset: o.h_offset,
                pitch: o.pitch,
                roll: o.roll,
                x: o.x,
                y: o.y,
                z: o.z,
                heading: o.heading,
                length: o.length,
                width: o.width,
                height: o.height,
                parking_access: (o.parking_access >= 0).then_some(o.parking_access),
                n_outlines: o.n_outlines,
                n_repeats: o.n_repeats,
            });
        }
        out
    }

    /// Metadata of outline `outline_idx` of object `obj_idx` on `road_id`.
    pub fn object_outline_info(
        &self,
        road_id: IdT,
        obj_idx: u32,
        outline_idx: u32,
    ) -> Option<OutlineInfo> {
        let mut o = ffi::GTRM_OutlineInfo::default();
        if unsafe { ffi::GTRM_GetObjectOutlineInfo(road_id, obj_idx, outline_idx, &mut o) } != 0 {
            return None;
        }
        Some(OutlineInfo {
            id: o.id,
            fill_type: o.fill_type,
            contour_type: o.contour_type,
            closed: o.closed != 0,
            roof: o.roof != 0,
            n_corners: o.n_corners,
        })
    }

    /// World-space outline corners of object `obj_idx` on `road_id`, paired with
    /// the index of the outline each corner belongs to.
    pub fn object_outline_corners(&self, road_id: IdT, obj_idx: u32) -> Vec<([f64; 3], i32)> {
        let n = unsafe { ffi::GTRM_BuildObjectOutline(road_id, obj_idx) }.max(0) as usize;
        if n == 0 {
            unsafe { ffi::GTRM_ClearObjectOutline() };
            return Vec::new();
        }
        let mut xyz = vec![0.0_f64; n * 3];
        let mut idx = vec![0_i32; n];
        unsafe { ffi::GTRM_CopyObjectOutline(xyz.as_mut_ptr(), idx.as_mut_ptr()) };
        unsafe { ffi::GTRM_ClearObjectOutline() };
        xyz.chunks_exact(3)
            .zip(idx)
            .map(|(c, i)| ([c[0], c[1], c[2]], i))
            .collect()
    }

    /// All `<tunnel>` records on `road_id`.
    pub fn tunnels(&self, road_id: IdT) -> Vec<Tunnel> {
        let n = unsafe { ffi::GTRM_GetNumberOfTunnels(road_id) };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut t = ffi::GTRM_Tunnel::default();
            if unsafe { ffi::GTRM_GetTunnel(road_id, i as u32, &mut t) } != 0 {
                continue;
            }
            out.push(Tunnel {
                road_id: t.road_id,
                id: t.id,
                tunnel_type: t.tunnel_type,
                name: unsafe { cstr_to_string(t.name) },
                s: t.s,
                length: t.length,
                width: t.width,
                lighting: t.lighting,
                daylight: t.daylight,
            });
        }
        out
    }

    /// Predecessor/successor link of `road_id`, or `None` if there is none.
    /// `link_type` is [`link::PREDECESSOR`] or [`link::SUCCESSOR`].
    pub fn road_link(&self, road_id: IdT, link_type: i32) -> Option<RoadLink> {
        let mut l = ffi::GTRM_RoadLink::default();
        if unsafe { ffi::GTRM_GetRoadLink(road_id, link_type, &mut l) } != 0 {
            return None;
        }
        Some(RoadLink {
            element_type: l.element_type,
            element_id: l.element_id,
            contact_point: l.contact_point,
        })
    }

    /// All junctions in the network.
    pub fn junctions(&self) -> Vec<Junction> {
        let n = unsafe { ffi::GTRM_GetNumberOfJunctions() };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut j = ffi::GTRM_Junction::default();
            if unsafe { ffi::GTRM_GetJunctionByIndex(i as u32, &mut j) } != 0 {
                continue;
            }
            out.push(Junction {
                id: j.id,
                global_id: j.global_id,
                junction_type: j.junction_type,
                name: unsafe { cstr_to_string(j.name) },
                n_connections: j.n_connections,
                n_controllers: j.n_controllers,
            });
        }
        out
    }

    /// Connections of junction `junction_id`.
    pub fn junction_connections(&self, junction_id: IdT) -> Vec<JunctionConnection> {
        let mut out = Vec::new();
        // Connection count is read from the junction record; iterate until error.
        let mut i = 0u32;
        loop {
            let mut c = ffi::GTRM_JunctionConnection::default();
            if unsafe { ffi::GTRM_GetJunctionConnection(junction_id, i, &mut c) } != 0 {
                break;
            }
            out.push(JunctionConnection {
                incoming_road_id: c.incoming_road_id,
                connecting_road_id: c.connecting_road_id,
                contact_point: c.contact_point,
                n_lane_links: c.n_lane_links,
            });
            i += 1;
        }
        out
    }

    /// Incoming->connecting lane id pairs of connection `conn_idx` in `junction_id`.
    pub fn junction_lane_links(&self, junction_id: IdT, conn_idx: u32) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        let mut i = 0u32;
        loop {
            let mut l = ffi::GTRM_LaneLink::default();
            if unsafe { ffi::GTRM_GetJunctionLaneLink(junction_id, conn_idx, i, &mut l) } != 0 {
                break;
            }
            out.push((l.from, l.to));
            i += 1;
        }
        out
    }

    /// All network controllers.
    pub fn controllers(&self) -> Vec<Controller> {
        let n = unsafe { ffi::GTRM_GetNumberOfControllers() };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut c = ffi::GTRM_Controller::default();
            if unsafe { ffi::GTRM_GetController(i as u32, &mut c) } != 0 {
                continue;
            }
            out.push(Controller {
                id: c.id,
                sequence: c.sequence,
                name: unsafe { cstr_to_string(c.name) },
                n_controls: c.n_controls,
            });
        }
        out
    }

    /// Full `<signal>` detail across all roads (richer than [`OdrMap::signs`]).
    pub fn signals(&self) -> Vec<Signal> {
        let mut out = Vec::new();
        for ri in 0..self.road_count() {
            let road_id = self.road_id_at(ri);
            if road_id == RM_ID_UNDEFINED {
                continue;
            }
            let n = unsafe { ffi::GTRM_GetNumberOfSignals(road_id) };
            for i in 0..n.max(0) {
                let mut s = ffi::GTRM_Signal::default();
                if unsafe { ffi::GTRM_GetSignal(road_id, i as u32, &mut s) } != 0 {
                    continue;
                }
                out.push(Signal {
                    road_id: s.road_id,
                    id: s.id,
                    global_id: s.global_id,
                    osi_type: s.osi_type,
                    orientation: s.orientation,
                    dynamic: s.dynamic != 0,
                    s: s.s,
                    t: s.t,
                    z_offset: s.z_offset,
                    h_offset: s.h_offset,
                    pitch: s.pitch,
                    roll: s.roll,
                    x: s.x,
                    y: s.y,
                    z: s.z,
                    heading: s.heading,
                    height: s.height,
                    width: s.width,
                    depth: s.depth,
                    length: s.length,
                    value: s.value,
                    name: unsafe { cstr_to_string(s.name) },
                    sign_type: unsafe { cstr_to_string(s.sign_type) },
                    subtype: unsafe { cstr_to_string(s.subtype) },
                    country: unsafe { cstr_to_string(s.country) },
                    value_str: unsafe { cstr_to_string(s.value_str) },
                    unit: unsafe { cstr_to_string(s.unit) },
                    text: unsafe { cstr_to_string(s.text) },
                });
            }
        }
        out
    }

    /// Elevation profile entries of `road_id`, in s-order.
    pub fn elevations(&self, road_id: IdT) -> Vec<Elevation> {
        self.elevation_list(road_id, false)
    }

    /// Super-elevation (cross-slope) profile entries of `road_id`, in s-order.
    pub fn super_elevations(&self, road_id: IdT) -> Vec<Elevation> {
        self.elevation_list(road_id, true)
    }

    fn elevation_list(&self, road_id: IdT, super_elev: bool) -> Vec<Elevation> {
        let n = unsafe {
            if super_elev {
                ffi::GTRM_GetNumberOfSuperElevations(road_id)
            } else {
                ffi::GTRM_GetNumberOfElevations(road_id)
            }
        };
        let mut out = Vec::new();
        for i in 0..n.max(0) {
            let mut e = ffi::GTRM_Elevation::default();
            let rc = unsafe {
                if super_elev {
                    ffi::GTRM_GetSuperElevation(road_id, i as u32, &mut e)
                } else {
                    ffi::GTRM_GetElevation(road_id, i as u32, &mut e)
                }
            };
            if rc == 0 {
                out.push(Elevation {
                    road_id: e.road_id,
                    s: e.s,
                    length: e.length,
                    a: e.a,
                    b: e.b,
                    c: e.c,
                    d: e.d,
                });
            }
        }
        out
    }

    /// Lateral lane offset of the reference line at road `s`, or `None` on error.
    pub fn lane_offset(&self, road_id: IdT, s: f64) -> Option<f64> {
        let mut o = 0.0;
        (unsafe { ffi::GTRM_GetLaneOffset(road_id, s, &mut o) } == 0).then_some(o)
    }

    /// Traffic rule of `road_id` (see [`road_rule`]), or `None` on error.
    pub fn road_rule(&self, road_id: IdT) -> Option<i32> {
        let r = unsafe { ffi::GTRM_GetRoadRule(road_id) };
        (r >= 0).then_some(r)
    }

    /// OpenDRIVE road type at road `s` (see [`road_type`]), or `None` on error.
    pub fn road_type(&self, road_id: IdT, s: f64) -> Option<i32> {
        let t = unsafe { ffi::GTRM_GetRoadType(road_id, s) };
        (t >= 0).then_some(t)
    }

    /// Speed (m/s) from the road-type element active at `s`, or `None` on error.
    pub fn road_speed(&self, road_id: IdT, s: f64) -> Option<f64> {
        let mut v = 0.0;
        (unsafe { ffi::GTRM_GetRoadSpeed(road_id, s, &mut v) } == 0).then_some(v)
    }

    /// Width (m) of `road_id` at `s` on `side` (-1 right, 1 left, 0 both),
    /// over any lane type, or `None` on error.
    pub fn road_width(&self, road_id: IdT, s: f64, side: i32) -> Option<f64> {
        let mut w = 0.0;
        (unsafe { ffi::GTRM_GetRoadWidth(road_id, s, side, &mut w) } == 0).then_some(w)
    }

    /// Network metadata (version, speed unit, friction), or `None` if unloaded.
    pub fn network_info(&self) -> Option<NetworkInfo> {
        let mut n = ffi::GTRM_NetworkInfo::default();
        if unsafe { ffi::GTRM_GetNetworkInfo(&mut n) } != 0 {
            return None;
        }
        Some(NetworkInfo {
            version_major: n.version_major,
            version_minor: n.version_minor,
            speed_unit: n.speed_unit,
            friction: n.friction,
        })
    }

    /// The network geo offset (OSI 3.7.0), or `None` if unloaded.
    pub fn geo_offset(&self) -> Option<GeoOffset> {
        let mut g = ffi::GTRM_GeoOffset::default();
        if unsafe { ffi::GTRM_GetGeoOffset(&mut g) } != 0 {
            return None;
        }
        Some(GeoOffset {
            x: g.x,
            y: g.y,
            z: g.z,
            hdg: g.hdg,
        })
    }

    /// Shortest-path distance (m) between two road positions, searching both
    /// directions, or `None` if no path exists. A negative value means the path
    /// runs opposite the heading at the start position.
    ///
    /// Note: OpenSCENARIO routes/trajectories/shapes are populated by the
    /// scenario engine, not by loading an `.xodr`, so only this road-network
    /// routing is exposed.
    pub fn shortest_path_distance(
        &self,
        road_a: IdT,
        s_a: f64,
        road_b: IdT,
        s_b: f64,
    ) -> Option<f64> {
        let mut d = 0.0;
        (unsafe { ffi::GTRM_ShortestPathDistance(road_a, s_a, road_b, s_b, &mut d) } == 0)
            .then_some(d)
    }

    /// Width (m) of `lane_id` on `road_id` at `s`; 0.0 if absent/error.
    fn lane_width(&self, road_id: IdT, lane_id: i32, s: f64) -> f64 {
        let mut w = 0.0;
        if unsafe { ffi::RM_GetLaneWidthByRoadId(road_id, lane_id, s, &mut w) } != 0 {
            return 0.0;
        }
        w
    }

    /// World point at lane (`lane_id`, lateral `offset` from lane center, `s`).
    fn lane_edge(&self, road_id: IdT, lane_id: i32, offset: f64, s: f64) -> Option<[f64; 3]> {
        if unsafe { ffi::RM_SetLanePosition(self.pos, road_id, lane_id, offset, s, false) } < 0 {
            return None;
        }
        self.position_world()
    }

    /// Read the current position object's world coordinates.
    fn position_world(&self) -> Option<[f64; 3]> {
        let mut data = ffi::RM_PositionData::default();
        if unsafe { ffi::RM_GetPositionData(self.pos, &mut data) } < 0 {
            return None;
        }
        Some([data.x, data.y, data.z])
    }
}

impl Drop for OdrMap {
    fn drop(&mut self) {
        unsafe {
            ffi::RM_DeletePosition(-1); // free all position objects
            ffi::RM_Close();
        }
        LOADED.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn xodr(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("external")
            .join("esmini")
            .join("resources")
            .join("xodr")
            .join(name)
    }

    // Single test on purpose: the C library is a global singleton, so we must
    // exercise maps sequentially (cargo runs separate #[test]s in parallel).
    #[test]
    fn loads_queries_and_reloads() {
        // Regression against PoC values.
        let map = OdrMap::load(xodr("multi_intersections.xodr")).expect("load multi");
        assert_eq!(map.road_count(), 63, "road count");
        assert_eq!(map.signs().len(), 127, "sign count");
        let quads = map.road_surface_quads(2.0);
        assert!(!quads.is_empty(), "expected road surface quads");

        // Phase 5: this network has junctions, each with at least one connection.
        let junctions = map.junctions();
        assert!(!junctions.is_empty(), "expected junctions");
        let j0 = &junctions[0];
        let conns = map.junction_connections(j0.id);
        assert_eq!(
            conns.len() as i32,
            j0.n_connections,
            "connection count matches record"
        );
        assert!(!conns.is_empty(), "expected junction connections");
        drop(map);

        // A second map loads after the first is dropped (RM_Close ran).
        let map2 = OdrMap::load(xodr("straight_500m_signs.xodr")).expect("load straight");
        assert_eq!(map2.road_count(), 1);
        assert_eq!(map2.signs().len(), 17);
        // Reference-line midpoint should be ~250 m along a 500 m straight road.
        let id = map2.road_id_at(0);
        assert!((map2.road_length(id) - 500.0).abs() < 1.0);
        assert!(map2.world_position(id, 250.0, 0.0).is_some());

        // Phase 1: a straight road is a single LINE geometry spanning ~500 m.
        let geoms = map2.geometries(id);
        assert!(!geoms.is_empty(), "expected reference-line geometry");
        assert_eq!(geoms[0].geom_type, geometry_type::LINE);
        let total: f64 = geoms.iter().map(|g| g.length).sum();
        assert!((total - 500.0).abs() < 1.0, "geometry length sum {total}");

        // Phase 1: the lane-section reference line has precomputed OSI points.
        let ref_pts = map2.lane_osi_points(id, 0, 0, osi_point_kind::REF_LINE);
        assert!(!ref_pts.is_empty(), "expected reference-line OSI points");

        // Phase 2: at least one lane section, with lanes including the center.
        let sections = map2.lane_sections(id);
        assert!(!sections.is_empty(), "expected lane sections");
        let lanes = map2.lanes(id, 0);
        assert!(!lanes.is_empty(), "expected lanes in section 0");
        assert!(lanes.iter().any(|l| l.lane_id == 0), "expected center lane");
        // The center lane sits on the reference line (offset ~0).
        let off = map2.lane_center_offset(id, 0, 250.0).expect("center offset");
        assert!(off.abs() < 1e-6, "center lane offset {off}");

        // Phase 3: the network defines at least one <roadMark> somewhere.
        let any_mark = lanes
            .iter()
            .any(|l| !map2.road_mark_meta(id, 0, l.lane_id).is_empty());
        assert!(any_mark, "expected at least one road mark");

        // Phase 4: straight_500m_signs.xodr has 13 <object> poles on road 0.
        let objects = map2.road_objects(id);
        assert_eq!(objects.len(), 13, "object count");
        assert!(
            objects.iter().all(|o| !o.type_name.is_empty()),
            "objects should have a type string"
        );

        // Phase 6: detailed signals match the count from the simpler signs() API.
        let signals = map2.signals();
        assert_eq!(signals.len(), 17, "detailed signal count");
        // Detail beyond signs(): at least some signals carry a type string.
        assert!(
            signals.iter().any(|s| !s.sign_type.is_empty()),
            "expected at least one typed signal"
        );

        // Phase 7: network metadata is queryable and the straight road is flat.
        let info = map2.network_info().expect("network info");
        assert!(info.version_major >= 1, "OpenDRIVE major version");
        let rule = map2.road_rule(id).expect("road rule");
        assert!(rule == road_rule::RIGHT_HAND_TRAFFIC || rule == road_rule::LEFT_HAND_TRAFFIC);
        // A flat 500 m road: full width at mid-span is positive.
        let w = map2.road_width(id, 250.0, 0).expect("road width");
        assert!(w > 0.0, "road width {w}");

        // Phase 8: shortest path along the same 500 m road spans ~500 m.
        let d = map2
            .shortest_path_distance(id, 0.0, id, 500.0)
            .expect("shortest path");
        assert!((d.abs() - 500.0).abs() < 5.0, "path distance {d}");
    }
}

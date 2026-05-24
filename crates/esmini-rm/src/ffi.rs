#![allow(dead_code)] // full struct/fn surface declared; not all used yet
//! Hand-written FFI declarations for esmini's RoadManager C API.
//!
//! Canon: `external/esmini/EnvironmentSimulator/Libraries/esminiRMLib/esminiRMLib.hpp`.
//! Only the subset needed for loading + road/lane/sign queries is declared; add
//! more as needed. Layouts mirror the `#[repr(C)]` structs in the header exactly.

use std::os::raw::{c_char, c_int, c_uint};

/// `typedef uint32_t id_t;` — `0xffffffff` means "undefined".
pub type IdT = u32;
pub const RM_ID_UNDEFINED: IdT = 0xffff_ffff;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RM_PositionXYZ {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RM_PositionData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub h: f64,
    pub p: f64,
    pub r: f64,
    pub h_relative: f64,
    pub road_id: IdT,
    pub junction_id: IdT,
    pub lane_id: c_int,
    pub lane_offset: f64,
    pub s: f64,
}

/// `RM_RoadSign` — note the trailing `name` is a borrowed C string owned by the
/// library; copy it out promptly (valid while the map stays loaded).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RM_RoadSign {
    pub id: c_int,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub z_offset: f64,
    pub h: f64,
    pub road_id: IdT,
    pub s: f64,
    pub t: f64,
    pub name: *const c_char,
    pub orientation: c_int,
    pub length: f64,
    pub height: f64,
    pub width: f64,
}

impl Default for RM_RoadSign {
    fn default() -> Self {
        // Safe: all-zero is a valid layout; `name` becomes a null pointer which
        // the wrapper treats as an empty string.
        unsafe { std::mem::zeroed() }
    }
}

// --- GTRM shim structs (cpp/gt_*.cpp), mirroring the C++ PODs exactly --------

/// `GTRM_Geometry` — one reference-line `<geometry>` record. See
/// `cpp/gt_geometry.cpp`. `type` is `roadmanager::Geometry::GeometryType`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_Geometry {
    pub road_id: IdT,
    pub geom_type: c_int,
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

/// `GTRM_OsiPoint` — one precomputed OSI sample point (mirrors `PointStruct`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_OsiPoint {
    pub s: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub h: f64,
    pub p: f64,
    pub r: f64,
    pub nx: f64,
    pub ny: f64,
    pub endpoint: c_int,
}

/// `GTRM_RoadMark` — `<roadMark>` style metadata. See `cpp/gt_roadmark.cpp`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_RoadMark {
    pub road_id: IdT,
    pub section_idx: c_uint,
    pub lane_id: c_int,
    pub mark_type: c_int,
    pub weight: c_int,
    pub color: c_int,
    pub material: c_int,
    pub lane_change: c_int,
    pub width: f64,
    pub height: f64,
    pub s_offset: f64,
    pub fade: f64,
}

/// `GTRM_RoadObject` — one `<object>`. See `cpp/gt_object.cpp`. `name`/`type_str`
/// are library-owned (copy out promptly).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GTRM_RoadObject {
    pub road_id: IdT,
    pub id: IdT,
    pub global_id: IdT,
    pub obj_type: c_int,
    pub orientation: c_int,
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
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub parking_access: c_int,
    pub n_outlines: c_int,
    pub n_repeats: c_int,
    pub name: *const c_char,
    pub type_str: *const c_char,
}

impl Default for GTRM_RoadObject {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `GTRM_OutlineInfo` — one `<outline>` of an object. See `cpp/gt_object.cpp`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_OutlineInfo {
    pub id: IdT,
    pub fill_type: c_int,
    pub contour_type: c_int,
    pub closed: c_int,
    pub roof: c_int,
    pub n_corners: c_int,
}

/// `GTRM_Tunnel` — one `<tunnel>`. `name` is library-owned (copy out promptly).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GTRM_Tunnel {
    pub road_id: IdT,
    pub id: IdT,
    pub tunnel_type: c_int,
    pub s: f64,
    pub length: f64,
    pub width: f64,
    pub lighting: f64,
    pub daylight: f64,
    pub name: *const c_char,
}

impl Default for GTRM_Tunnel {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `GTRM_Elevation` — one elevation / super-elevation cubic. See `cpp/gt_misc.cpp`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_Elevation {
    pub road_id: IdT,
    pub s: f64,
    pub length: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
}

/// `GTRM_GeoOffset` — the network geo offset (OSI 3.7.0).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_GeoOffset {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub hdg: f64,
}

/// `GTRM_NetworkInfo` — network metadata. `speed_unit`: 0 undefined,1 km/h,2 m/s,3 mph.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_NetworkInfo {
    pub version_major: c_int,
    pub version_minor: c_int,
    pub speed_unit: c_int,
    pub friction: f64,
}

/// `GTRM_Signal` — full `<signal>` detail. See `cpp/gt_signal.cpp`. All string
/// fields are library-owned (copy out promptly).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GTRM_Signal {
    pub road_id: IdT,
    pub id: c_int,
    pub global_id: IdT,
    pub osi_type: c_int,
    pub orientation: c_int,
    pub dynamic: c_int,
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
    pub name: *const c_char,
    pub sign_type: *const c_char,
    pub subtype: *const c_char,
    pub country: *const c_char,
    pub value_str: *const c_char,
    pub unit: *const c_char,
    pub text: *const c_char,
}

impl Default for GTRM_Signal {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `GTRM_RoadLink` — a road predecessor/successor link. See `cpp/gt_topology.cpp`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_RoadLink {
    pub element_type: c_int,
    pub element_id: IdT,
    pub contact_point: c_int,
}

/// `GTRM_Junction` — a junction. `name` is library-owned (copy out promptly).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GTRM_Junction {
    pub id: IdT,
    pub global_id: IdT,
    pub junction_type: c_int,
    pub n_connections: c_int,
    pub n_controllers: c_int,
    pub name: *const c_char,
}

impl Default for GTRM_Junction {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `GTRM_JunctionConnection` — one connection within a junction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_JunctionConnection {
    pub incoming_road_id: IdT,
    pub connecting_road_id: IdT,
    pub contact_point: c_int,
    pub n_lane_links: c_int,
}

/// `GTRM_LaneLink` — one incoming->connecting lane mapping.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_LaneLink {
    pub from: c_int,
    pub to: c_int,
}

/// `GTRM_Controller` — a network controller. `name` is library-owned.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GTRM_Controller {
    pub id: IdT,
    pub sequence: c_int,
    pub n_controls: c_int,
    pub name: *const c_char,
}

impl Default for GTRM_Controller {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

/// `GTRM_LaneSection` — one `<laneSection>`. See `cpp/gt_lane.cpp`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_LaneSection {
    pub road_id: IdT,
    pub s: f64,
    pub length: f64,
    pub n_lanes: c_int,
}

/// `GTRM_Lane` — one lane within a section. `lane_type` is the
/// `roadmanager::Lane::LaneType` bitmask.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GTRM_Lane {
    pub road_id: IdT,
    pub section_idx: c_uint,
    pub lane_id: c_int,
    pub lane_type: c_int,
    pub global_id: IdT,
    pub is_road_edge: c_int,
    pub has_pred: c_int,
    pub pred_lane_id: c_int,
    pub has_succ: c_int,
    pub succ_lane_id: c_int,
}

extern "C" {
    /// Set the logfile path; `""` disables the logfile. Call before `RM_Init`.
    pub fn RM_SetLogFilePath(log_file_path: *const c_char);
    /// Load an OpenDRIVE file. Returns 0 on success, -1 on failure.
    pub fn RM_Init(odr_filename: *const c_char) -> c_int;
    /// Unload the current road network. Returns 0 on success.
    pub fn RM_Close() -> c_int;

    /// Number of roads in the loaded network, or -1 on error.
    pub fn RM_GetNumberOfRoads() -> c_int;
    /// Road ID at the given index, or `RM_ID_UNDEFINED` on error.
    pub fn RM_GetIdOfRoadFromIndex(index: c_uint) -> IdT;
    /// Length (m) of the road with the given ID, or 0.0 if not found.
    pub fn RM_GetRoadLength(id: IdT) -> f64;

    /// Number of lanes of `type_mask` (use -1 for any) on `road_id` at `s`.
    pub fn RM_GetRoadNumberOfLanes(road_id: IdT, s: f64, type_mask: c_int) -> c_int;
    /// Lane ID by index/type at `s`, written to `lane_id`. Returns 0 on success.
    pub fn RM_GetLaneIdByIndex(
        road_id: IdT,
        lane_index: c_int,
        s: f64,
        type_mask: c_int,
        lane_id: *mut c_int,
    ) -> c_int;
    /// Width (m) of `lane_id` on `road_id` at `s`, written to `width`. 0 on success.
    pub fn RM_GetLaneWidthByRoadId(road_id: IdT, lane_id: c_int, s: f64, width: *mut f64) -> c_int;
    /// Lane type (see RoadManager::Lane::LaneType) of `lane_id` on `road_id` at `s`.
    pub fn RM_GetLaneTypeByRoadId(road_id: IdT, lane_id: c_int, s: f64) -> c_int;
    /// Number of drivable lanes on `road_id` at `s`, or -1 on error.
    pub fn RM_GetRoadNumberOfDrivableLanes(road_id: IdT, s: f64) -> c_int;
    /// Drivable lane ID by index at `s`, written to `lane_id`. 0 on success.
    pub fn RM_GetDrivableLaneIdByIndex(
        road_id: IdT,
        lane_index: c_int,
        s: f64,
        lane_id: *mut c_int,
    ) -> c_int;

    /// Allocate a position object. Returns a handle >= 0, or -1 on error.
    pub fn RM_CreatePosition() -> c_int;
    /// Free a position object (-1 frees all). Returns 0 on success.
    pub fn RM_DeletePosition(handle: c_int) -> c_int;
    /// Set a position from road (s, t) coordinates; world coords are computed.
    pub fn RM_SetRoadPosition(handle: c_int, road_id: IdT, s: f64, t: f64, align: bool) -> c_int;
    /// Set a position from lane coordinates. `lane_offset` is the lateral
    /// distance from the lane center (so +/- half-width gives the lane borders);
    /// world coords (incl. elevation/superelevation) are computed.
    pub fn RM_SetLanePosition(
        handle: c_int,
        road_id: IdT,
        lane_id: c_int,
        lane_offset: f64,
        s: f64,
        align: bool,
    ) -> c_int;
    /// Read back the computed fields (world x/y/z/h, lane id, ...). >=0 on success.
    pub fn RM_GetPositionData(handle: c_int, data: *mut RM_PositionData) -> c_int;

    // --- Our own shims (cpp/gt_*.cpp), not part of esminiRMLib ---
    /// Build road-mark triangles for the loaded network; returns vertex count.
    pub fn GTRM_BuildRoadMarks(z_offset: f64) -> c_int;
    /// Copy built geometry: `out_xyz` = 3*verts f64, `out_color` = verts i32.
    pub fn GTRM_CopyRoadMarks(out_xyz: *mut f64, out_color: *mut c_int);
    /// Release the accumulated road-mark buffers.
    pub fn GTRM_ClearRoadMarks();
    /// Number of `<roadMark>` entries on lane `lane_id` of (road, section); -1 on error.
    pub fn GTRM_GetNumberOfRoadMarks(road_id: IdT, section_idx: c_uint, lane_id: c_int) -> c_int;
    /// Fill `out` with road-mark `mark_idx` of lane `lane_id`. 0 / -1.
    pub fn GTRM_GetRoadMark(
        road_id: IdT,
        section_idx: c_uint,
        lane_id: c_int,
        mark_idx: c_uint,
        out: *mut GTRM_RoadMark,
    ) -> c_int;

    // gt_geometry.cpp — reference-line geometry + OSI sample points.
    /// Number of `<geometry>` records on `road_id`'s reference line; -1 on error.
    pub fn GTRM_GetNumberOfGeometries(road_id: IdT) -> c_int;
    /// Fill `out` with geometry `idx` of `road_id`. 0 on success, -1 on error.
    pub fn GTRM_GetGeometry(road_id: IdT, idx: c_uint, out: *mut GTRM_Geometry) -> c_int;
    /// Build OSI points of a lane/ref-line/boundary into a buffer; returns count.
    /// `kind`: 0 = lane, 1 = lane-section reference line (lane_id ignored), 2 = boundary.
    pub fn GTRM_BuildLaneOsiPoints(
        road_id: IdT,
        section_idx: c_uint,
        lane_id: c_int,
        kind: c_int,
    ) -> c_int;
    /// Copy the built OSI points: `out` needs room for the Build count.
    pub fn GTRM_CopyOsiPoints(out: *mut GTRM_OsiPoint);
    /// Release the accumulated OSI points.
    pub fn GTRM_ClearOsiPoints();

    // gt_lane.cpp — lane-section / lane structure.
    /// Number of lane sections on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfLaneSections(road_id: IdT) -> c_int;
    /// Fill `out` with lane section `section_idx`. 0 / -1.
    pub fn GTRM_GetLaneSection(road_id: IdT, section_idx: c_uint, out: *mut GTRM_LaneSection)
        -> c_int;
    /// Number of lanes in section `section_idx`; -1 on error.
    pub fn GTRM_GetNumberOfLanesInSection(road_id: IdT, section_idx: c_uint) -> c_int;
    /// Fill `out` with lane `lane_idx` (vector index) of the section. 0 / -1.
    pub fn GTRM_GetLane(
        road_id: IdT,
        section_idx: c_uint,
        lane_idx: c_uint,
        out: *mut GTRM_Lane,
    ) -> c_int;
    /// Lateral center offset (m) of `lane_id` at road `s`, into `out`. 0 / -1.
    pub fn GTRM_GetLaneCenterOffset(road_id: IdT, lane_id: c_int, s: f64, out: *mut f64) -> c_int;
    /// Friction of `lane_id` material at road `s`, into `out`. 0 / -1.
    pub fn GTRM_GetLaneFriction(road_id: IdT, lane_id: c_int, s: f64, out: *mut f64) -> c_int;

    // gt_object.cpp — road objects, outlines, tunnels.
    /// Number of `<object>` records on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfObjects(road_id: IdT) -> c_int;
    /// Fill `out` with object `obj_idx`. 0 / -1.
    pub fn GTRM_GetRoadObject(road_id: IdT, obj_idx: c_uint, out: *mut GTRM_RoadObject) -> c_int;
    /// Fill `out` with outline `outline_idx` metadata of object `obj_idx`. 0 / -1.
    pub fn GTRM_GetObjectOutlineInfo(
        road_id: IdT,
        obj_idx: c_uint,
        outline_idx: c_uint,
        out: *mut GTRM_OutlineInfo,
    ) -> c_int;
    /// Build the world-space outline corners of object `obj_idx`; returns corner count.
    pub fn GTRM_BuildObjectOutline(road_id: IdT, obj_idx: c_uint) -> c_int;
    /// Copy built corners: `out_xyz` = 3*corners f64, `out_outline_idx` = corners i32.
    pub fn GTRM_CopyObjectOutline(out_xyz: *mut f64, out_outline_idx: *mut c_int);
    /// Release the accumulated outline corners.
    pub fn GTRM_ClearObjectOutline();
    /// Number of `<tunnel>` records on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfTunnels(road_id: IdT) -> c_int;
    /// Fill `out` with tunnel `idx` of `road_id`. 0 / -1.
    pub fn GTRM_GetTunnel(road_id: IdT, idx: c_uint, out: *mut GTRM_Tunnel) -> c_int;

    // gt_topology.cpp — road links, junctions, connections, controllers.
    /// Fill `out` with the predecessor (-1) or successor (1) link of `road_id`.
    /// 0 if a link exists, -1 otherwise.
    pub fn GTRM_GetRoadLink(road_id: IdT, link_type: c_int, out: *mut GTRM_RoadLink) -> c_int;
    /// Number of junctions in the network; -1 on error.
    pub fn GTRM_GetNumberOfJunctions() -> c_int;
    /// Fill `out` with junction at vector `index`. 0 / -1.
    pub fn GTRM_GetJunctionByIndex(index: c_uint, out: *mut GTRM_Junction) -> c_int;
    /// Fill `out` with connection `conn_idx` of junction `junction_id`. 0 / -1.
    pub fn GTRM_GetJunctionConnection(
        junction_id: IdT,
        conn_idx: c_uint,
        out: *mut GTRM_JunctionConnection,
    ) -> c_int;
    /// Fill `out` with lane link `link_idx` of connection `conn_idx`. 0 / -1.
    pub fn GTRM_GetJunctionLaneLink(
        junction_id: IdT,
        conn_idx: c_uint,
        link_idx: c_uint,
        out: *mut GTRM_LaneLink,
    ) -> c_int;
    /// Number of network controllers; -1 on error.
    pub fn GTRM_GetNumberOfControllers() -> c_int;
    /// Fill `out` with controller at vector `index`. 0 / -1.
    pub fn GTRM_GetController(index: c_uint, out: *mut GTRM_Controller) -> c_int;

    // gt_signal.cpp — full <signal> detail.
    /// Number of `<signal>` records on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfSignals(road_id: IdT) -> c_int;
    /// Fill `out` with signal `idx` of `road_id`. 0 / -1.
    pub fn GTRM_GetSignal(road_id: IdT, idx: c_uint, out: *mut GTRM_Signal) -> c_int;

    // gt_misc.cpp — profiles & network metadata.
    /// Number of elevation entries on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfElevations(road_id: IdT) -> c_int;
    /// Fill `out` with elevation entry `idx`. 0 / -1.
    pub fn GTRM_GetElevation(road_id: IdT, idx: c_uint, out: *mut GTRM_Elevation) -> c_int;
    /// Number of super-elevation entries on `road_id`; -1 on error.
    pub fn GTRM_GetNumberOfSuperElevations(road_id: IdT) -> c_int;
    /// Fill `out` with super-elevation entry `idx`. 0 / -1.
    pub fn GTRM_GetSuperElevation(road_id: IdT, idx: c_uint, out: *mut GTRM_Elevation) -> c_int;
    /// Lane offset at road `s`, into `out`. 0 / -1.
    pub fn GTRM_GetLaneOffset(road_id: IdT, s: f64, out: *mut f64) -> c_int;
    /// Road rule: 0 RHT, 1 LHT; -1 on error.
    pub fn GTRM_GetRoadRule(road_id: IdT) -> c_int;
    /// OpenDRIVE road type at road `s`; -1 on error.
    pub fn GTRM_GetRoadType(road_id: IdT, s: f64) -> c_int;
    /// Speed (m/s) from the road type element at `s`, into `out`. 0 / -1.
    pub fn GTRM_GetRoadSpeed(road_id: IdT, s: f64, out: *mut f64) -> c_int;
    /// Width (m) at `s` on `side` (-1 right,1 left,0 both), into `out`. 0 / -1.
    pub fn GTRM_GetRoadWidth(road_id: IdT, s: f64, side: c_int, out: *mut f64) -> c_int;
    /// Fill `out` with network metadata. 0 / -1.
    pub fn GTRM_GetNetworkInfo(out: *mut GTRM_NetworkInfo) -> c_int;
    /// Fill `out` with the network geo offset. 0 / -1.
    pub fn GTRM_GetGeoOffset(out: *mut GTRM_GeoOffset) -> c_int;

    // gt_route.cpp — network routing (RoadPath).
    /// Shortest-path distance (m) between (road_a, s_a) and (road_b, s_b),
    /// into `out_dist` (negative = opposite start heading). 0 / -1 (no path).
    pub fn GTRM_ShortestPathDistance(
        road_a: IdT,
        s_a: f64,
        road_b: IdT,
        s_b: f64,
        out_dist: *mut f64,
    ) -> c_int;

    /// Number of road signs along `road_id`, or -1 on error.
    pub fn RM_GetNumberOfRoadSigns(road_id: IdT) -> c_int;
    /// Fill `road_sign` for the sign at `index` (not ID) on `road_id`. 0 on success.
    pub fn RM_GetRoadSign(road_id: IdT, index: c_uint, road_sign: *mut RM_RoadSign) -> c_int;
}

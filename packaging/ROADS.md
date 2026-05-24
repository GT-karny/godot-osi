# godot-osi — OpenDRIVE Road API

The addon can also load an **OpenDRIVE** (`.xodr`) road network and answer
geometry / lane / object / signal / topology queries from GDScript. This is
powered by a bundled, statically-linked build of [esmini](https://github.com/esmini/esmini)'s
RoadManager — no external DLL, server, or scenario engine is required, just the
`.xodr` file.

Two classes are involved:

- **`OsiRoadNetwork`** *(extends Resource)* — loads a map and answers queries.
- **`OsiRoadNetworkVisualizer`** *(extends Node3D)* — an optional demo helper
  that renders the loaded network (road surface + lane markings + sign markers).

For the OSI streaming/converter pipeline (a separate feature) see
[README.md](README.md) and [API.md](API.md). This document is self-contained for
the road feature.

---

## Quickstart

### Load and visualize

```gdscript
extends Node3D

func _ready() -> void:
    var net := OsiRoadNetwork.new()
    if not net.load("res://maps/my_road.xodr"):
        push_error("failed to load map")
        return

    var viz := OsiRoadNetworkVisualizer.new()
    add_child(viz)
    viz.build_from(net)        # builds road surface + lane marks + sign markers
```

> The **examples** download bundles sample maps under
> `res://addons/godot_osi/examples/roads/` (e.g. `straight_500m_signs.xodr`,
> `e6mini.xodr`, `multi_intersections.xodr`) you can load directly.

### Load and query

```gdscript
var net := OsiRoadNetwork.new()
net.load("res://maps/my_road.xodr")

print("roads: ", net.road_count())
var rid := net.road_id_at(0)              # first road's id
print("length: ", net.road_length(rid))

# Drive a point along a lane:
for s in range(0, int(net.road_length(rid)), 5):
    var p: Vector3 = net.lane_point(rid, -1, float(s))   # lane -1 center, Godot space
    # ... place something at p ...

# Inspect structure:
for sec in net.lane_sections(rid):
    for lane in net.lanes(rid, sec["index"]):
        print("lane %d type=%d" % [lane["lane_id"], lane["lane_type"]])

for sig in net.signals():
    print("%s @ %s" % [sig["type"], sig["pos"]])
```

> **Singleton:** the underlying library keeps **one** road network loaded
> process-wide. Calling `load()` again releases the previous map first; only use
> one `OsiRoadNetwork` instance at a time.

## Coordinate systems

Methods that return a world position do so as a Godot `Vector3` already converted
into **Godot space** (left-handed, Y-up) using the default axis mapping
(`Godot(x, y, z) = OSI(x, z, -y)`) — these are the `Vector3` return values and
the `"pos"` key inside returned dictionaries.

Where a dictionary also carries raw `"x"`/`"y"`/`"z"` (and `s`/`t`) fields, those
are the **untransformed OpenDRIVE/OSI** values (right-handed, Z-up, meters), so
you can do your own math. Headings/`s`/`t`/widths/offsets are always raw OpenDRIVE.

Scalar getters return `NAN` when the value is unavailable; id getters return `-1`;
dictionary getters return an **empty** dictionary on failure (check with
`.is_empty()`).

---

## `OsiRoadNetwork` — methods

### Loading & roads

| Method | Returns | Description |
|---|---|---|
| `load(path: String)` | bool | Load a `.xodr` (accepts `res://`/`user://` or an OS path). Releases any previous map. `true` on success. |
| `is_loaded()` | bool | Whether a network is currently loaded. |
| `road_count()` | int | Number of roads in the network. |
| `road_id_at(index: int)` | int | Road id at `index` (0-based), or `-1`. |
| `road_length(road_id: int)` | float | Road length in meters (`0.0` if unknown). |
| `world_position(road_id: int, s: float, t: float)` | Vector3 | Godot-space point at road `(s, t)`; `Vector3.ZERO` on error. |

### Lanes

| Method | Returns | Description |
|---|---|---|
| `drivable_lanes(road_id: int, s: float)` | PackedInt32Array | Drivable lane ids at distance `s`. |
| `lane_point(road_id: int, lane_id: int, s: float)` | Vector3 | Godot-space center of `lane_id` at `s`; `Vector3.ZERO` on error. |
| `lane_sections(road_id: int)` | Array[Dictionary] | Lane sections of the road. |
| `lanes(road_id: int, section_idx: int)` | Array[Dictionary] | Lanes in a section (includes the center lane 0). |
| `lane_center_offset(road_id: int, lane_id: int, s: float)` | float | Lateral offset (m) of the lane center from the reference line; `NAN` on error. |
| `lane_friction(road_id: int, lane_id: int, s: float)` | float | Lane material friction at `s`; `NAN` if undefined. |

### Geometry & OSI sample points

| Method | Returns | Description |
|---|---|---|
| `geometries(road_id: int)` | Array[Dictionary] | Reference-line geometry primitives (line/arc/spiral/poly3/paramPoly3), in s-order. |
| `lane_osi_points(road_id: int, section_idx: int, lane_id: int, kind: int)` | Array[Dictionary] | Precomputed OSI sample points. `kind` ∈ `OSI_LANE` / `OSI_REF_LINE` / `OSI_BOUNDARY` (see constants). For `OSI_REF_LINE`, `lane_id` is ignored. |

### Road marks (lane lines)

| Method | Returns | Description |
|---|---|---|
| `road_marks(road_id: int, section_idx: int, lane_id: int)` | Array[Dictionary] | `<roadMark>` style records on a lane (solid/broken/color/width/…). The visualizer turns these into painted geometry. |

### Road objects, outlines & tunnels

| Method | Returns | Description |
|---|---|---|
| `road_objects(road_id: int)` | Array[Dictionary] | `<object>` records (barriers, poles, trees, buildings, parking spaces, …). |
| `object_outline_info(road_id: int, obj_idx: int, outline_idx: int)` | Dictionary | Metadata of one object outline (fill/contour type, closed/roof, corner count). Empty if none. |
| `object_outline_corners(road_id: int, obj_idx: int)` | Array[Dictionary] | World-space outline corners of an object, each tagged with its outline index. |
| `tunnels(road_id: int)` | Array[Dictionary] | `<tunnel>` records on the road. |

### Topology — links, junctions, controllers

| Method | Returns | Description |
|---|---|---|
| `road_link(road_id: int, link_type: int)` | Dictionary | Predecessor (`LINK_PREDECESSOR`) or successor (`LINK_SUCCESSOR`) link. Empty if none. |
| `junctions()` | Array[Dictionary] | All junctions in the network. |
| `junction_connections(junction_id: int)` | Array[Dictionary] | Connections of a junction. |
| `junction_lane_links(junction_id: int, conn_idx: int)` | Array[Dictionary] | Incoming→connecting lane id pairs of a connection. |
| `controllers()` | Array[Dictionary] | Network `<controller>` records. |

### Signals

| Method | Returns | Description |
|---|---|---|
| `sign_count()` | int | Number of road signs across all roads. |
| `sign_positions()` | PackedVector3Array | Godot-space positions of all signs (quick placement). |
| `signals()` | Array[Dictionary] | Full `<signal>` detail (osi type, type/subtype/country/value/unit/text, pose) plus semantic classification (category, label, icon, …). |
| `classify_signal(type, subtype)` | Dictionary | Classify a signal by `type`/`subtype` (country `"OpenDRIVE"`) without a loaded map. |

### Profiles & network metadata

| Method | Returns | Description |
|---|---|---|
| `elevations(road_id: int)` | Array[Dictionary] | Elevation profile entries (cubic `a..d` from `s`). |
| `super_elevations(road_id: int)` | Array[Dictionary] | Super-elevation (cross-slope) profile entries. |
| `lane_offset(road_id: int, s: float)` | float | Lateral lane offset of the reference line at `s`; `NAN` on error. |
| `road_rule(road_id: int)` | int | Traffic rule (`0` RHT, `1` LHT); `-1` on error. |
| `road_type(road_id: int, s: float)` | int | OpenDRIVE road type at `s` (see constants); `-1` on error. |
| `road_speed(road_id: int, s: float)` | float | Speed (m/s) from the active road-type element; `NAN` on error. |
| `road_width(road_id: int, s: float, side: int)` | float | Width (m) at `s` on `side` (`-1` right, `1` left, `0` both); `NAN` on error. |
| `network_info()` | Dictionary | Version, speed unit, friction. Empty if unloaded. |
| `geo_offset()` | Dictionary | Network geo offset (OSI 3.7.0). Empty if unloaded. |

### Routing

| Method | Returns | Description |
|---|---|---|
| `shortest_path_distance(road_a: int, s_a: float, road_b: int, s_b: float)` | float | Shortest-path distance (m) between two road positions (negative = opposite the start heading); `NAN` if no path. |

### Constants

| Constant | Value | Use |
|---|---|---|
| `OSI_LANE` | 0 | `lane_osi_points` — the lane's own OSI points (outer edge). |
| `OSI_REF_LINE` | 1 | `lane_osi_points` — the lane-section reference line. |
| `OSI_BOUNDARY` | 2 | `lane_osi_points` — the lane's OSI boundary. |
| `LINK_PREDECESSOR` | -1 | `road_link` — predecessor link. |
| `LINK_SUCCESSOR` | 1 | `road_link` — successor link. |

---

## Returned dictionary shapes

`pos` is a Godot-space `Vector3`; `x`/`y`/`z` are raw OpenDRIVE/OSI; angles,
`s`/`t`, widths and offsets are raw OpenDRIVE (meters / radians). Enum-coded
integer fields are explained under **Enumerations** below.

**`geometries()`** — one reference-line primitive:
| Key | Type | Notes |
|---|---|---|
| `type` | int | Geometry type (see *Geometry type*). |
| `s` | float | Start station along the road. |
| `x`, `y`, `hdg` | float | Start pose of the segment. |
| `length` | float | Segment length. |
| `curv_start`, `curv_end` | float | Curvature (arc: both equal; spiral: linear from start→end; else 0). |
| `a`, `b`, `c`, `d` | float | poly3 coefficients, or paramPoly3 **U** coefficients. |
| `a2`, `b2`, `c2`, `d2` | float | paramPoly3 **V** coefficients (else 0). |

**`lane_osi_points()`** — one sampled point:
| Key | Type | Notes |
|---|---|---|
| `pos` | Vector3 | Godot-space position. |
| `s` | float | Station. |
| `x`, `y`, `z` | float | Raw OSI position. |
| `h`, `p`, `r` | float | Heading / pitch / roll. |
| `nx`, `ny` | float | Surface-normal XY components. |
| `endpoint` | bool | End of a contiguous run (e.g. a dash of a broken mark). |

**`lane_sections()`**:
| Key | Type | Notes |
|---|---|---|
| `index` | int | Section index (pass to `lanes` / `lane_osi_points`). |
| `s` | float | Section start station. |
| `length` | float | Section length. |
| `n_lanes` | int | Lane count (incl. center). |

**`lanes()`**:
| Key | Type | Notes |
|---|---|---|
| `lane_id` | int | OpenDRIVE lane id (0 = center, >0 left, <0 right). |
| `lane_type` | int | Lane type bitmask (see *Lane type*). |
| `global_id` | int | OSI global lane id. |
| `is_road_edge` | bool | Whether this lane is the paved-road edge. |
| `predecessor` | int | Linked lane id in the previous section, or `-9223372036854775808` (`int min`) if none. |
| `successor` | int | Linked lane id in the next section, or `int min` if none. |

**`road_marks()`**:
| Key | Type | Notes |
|---|---|---|
| `type` | int | Mark type (see *Road-mark type*). |
| `weight` | int | 0 standard, 1 bold. |
| `color` | int | Mark color (see *Road-mark color*). |
| `material` | int | 0 standard. |
| `lane_change` | int | 0 increase, 1 decrease, 2 both, 3 none. |
| `width`, `height` | float | Mark dimensions. |
| `s_offset` | float | Start offset within the lane section. |
| `fade` | float | Fade factor. |

**`road_objects()`**:
| Key | Type | Notes |
|---|---|---|
| `index` | int | Object index (pass to outline calls). |
| `id`, `global_id` | int | OpenDRIVE id / OSI global id. |
| `type` | int | Object type (see *Object type*). |
| `type_name` | String | Object type string. |
| `name` | String | Object name. |
| `orientation` | int | 0 positive, 1 negative, 2 none. |
| `pos` | Vector3 | Godot-space position. |
| `s`, `t` | float | Road-relative position. |
| `z_offset`, `h_offset`, `pitch`, `roll`, `heading` | float | Pose. |
| `length`, `width`, `height` | float | Bounding box. |
| `parking_access` | int | Parking access (see *Parking access*) if a parking space, else `-1`. |
| `n_outlines`, `n_repeats` | int | Counts of outlines / repeat records. |

**`object_outline_info()`**:
| Key | Type | Notes |
|---|---|---|
| `id` | int | Outline id. |
| `fill_type` | int | See *Outline fill type*. |
| `contour_type` | int | 0 polygon, 1 quad-strip. |
| `closed`, `roof` | bool | Closed contour / has a roof. |
| `n_corners` | int | Corner count. |

**`object_outline_corners()`**: `{ "pos": Vector3, "outline_index": int }`.

**`tunnels()`**:
| Key | Type | Notes |
|---|---|---|
| `id` | int | Tunnel id. |
| `type` | int | 0 standard, 1 underpass. |
| `name` | String | Tunnel name. |
| `s`, `length`, `width` | float | Placement / extent. |
| `lighting`, `daylight` | float | OpenDRIVE lighting / daylight factors. |

**`road_link()`**:
| Key | Type | Notes |
|---|---|---|
| `element_type` | int | 0 unknown, 1 road, 2 junction. |
| `element_id` | int | Linked road or junction id. |
| `contact_point` | int | 0 undefined, 1 start, 2 end, 3 junction. |

**`junctions()`**:
| Key | Type | Notes |
|---|---|---|
| `id`, `global_id` | int | Junction id / OSI global id. |
| `type` | int | 0 default, 1 direct, 2 virtual. |
| `name` | String | Junction name. |
| `n_connections`, `n_controllers` | int | Counts. |

**`junction_connections()`**: `{ "incoming_road_id": int, "connecting_road_id": int, "contact_point": int, "n_lane_links": int }`.

**`junction_lane_links()`**: `{ "from": int, "to": int }`.

**`controllers()`**: `{ "id": int, "sequence": int, "name": String, "n_controls": int }`.

**`signals()`**:
| Key | Type | Notes |
|---|---|---|
| `road_id`, `id`, `global_id` | int | Owning road / OpenDRIVE id / OSI global id. |
| `osi_type` | int | Raw OSI signal type code. |
| `orientation` | int | 0 positive, 1 negative, 2 none. |
| `dynamic` | bool | Dynamic signal (e.g. traffic light). |
| `pos` | Vector3 | Godot-space position. |
| `s`, `t` | float | Road-relative position. |
| `z_offset`, `h_offset`, `pitch`, `roll`, `heading` | float | Pose. |
| `height`, `width`, `depth`, `length` | float | Bounding box. |
| `value` | float | Numeric value (e.g. speed limit), if any. |
| `name`, `type`, `subtype`, `country`, `value_str`, `unit`, `text` | String | OpenDRIVE signal strings. |
| `matched` | bool | `true` if the signal catalogue classified this `type`/`subtype`. |
| `category` | String | `traffic_light`, `road_marking`, or `tram_signal`. |
| `subcategory` | String | e.g. `vehicle`, `pedestrian`, `bicycle`, `vehicle_arrow`, `tram`. |
| `color` | String | Lit lamp colour (`red`/`yellow`/`green`), `multi` for a full head, else `none`. |
| `arrow` | String | Arrow direction (`left`/`right`/`straight`/…) or `none`. |
| `osi_type_name` | String | OSI main-sign type enum name (catalogue, else resolved from `osi_type`). |
| `nr_lamps` | int | Lamp count of the signal head (0 for markings). |
| `icon` | String | Icon key under `addons/godot_osi/icons/signals/<icon>.png`, or empty. |
| `label_en`, `label_ja` | String | Human-readable label (English / Japanese). |

The classification (`category`…`label_ja`) is derived from the bundled OpenDRIVE
signal catalogue (`country = "OpenDRIVE"`); it does not depend on esmini locating
its runtime traffic-sign files. Use [`classify_signal()`](#classify_signal) to
look up the same fields without a loaded map.

**`classify_signal(type: String, subtype: String)`**: classify an OpenDRIVE
signal by `type`/`subtype` alone (country `"OpenDRIVE"`), returning the
`matched`, `category`, `subcategory`, `color`, `arrow`, `osi_type_name`,
`nr_lamps`, `icon`, `label_en`, `label_ja` fields above. Handy when choosing
which signal model/icon to instance in a scene.

**`elevations()` / `super_elevations()`**: `{ "s": float, "length": float, "a": float, "b": float, "c": float, "d": float }` — evaluate the cubic in local `ds = roadS - s`.

**`network_info()`**: `{ "version_major": int, "version_minor": int, "speed_unit": int, "friction": float }`.

**`geo_offset()`**: `{ "x": float, "y": float, "z": float, "hdg": float }`.

---

## Enumerations

**Geometry type** (`geometries().type`): 0 unknown, 1 line, 2 arc, 3 spiral,
4 poly3, 5 paramPoly3.

**Lane type** (`lanes().lane_type`) — a **bitmask**; test with `&`. Common values:
`1` none/reference-line, `2` driving, `4` stop, `8` shoulder, `16` biking,
`32` sidewalk, `64` border, `128` restricted, `256` parking, `512` bidirectional,
`1024` median, `32768` tram, `65536` rail, `131072` entry, `262144` exit,
`524288` off-ramp, `1048576` on-ramp, `2097152` curb, `4194304` connecting-ramp.

**Road-mark type** (`road_marks().type`): 1 none, 2 solid, 3 broken,
4 solid-solid, 5 solid-broken, 6 broken-solid, 7 broken-broken, 8 botts-dots,
9 grass, 10 curb.

**Road-mark color** (`road_marks().color`): 0 undefined, 1 black, 2 blue,
3 green, 4 orange, 5 red, 6 standard (white), 7 violet, 8 white, 9 yellow.

**Object type** (`road_objects().type`): 0 barrier, 1 bike, 2 building, 3 bus,
4 car, 5 crosswalk, 6 gantry, 7 motorbike, 8 none, 9 obstacle, 10 parking-space,
11 patch, 12 pedestrian, 13 pole, 14 railing, 15 road-mark, 16 sound-barrier,
17 street-lamp, 18 traffic-island, 19 trailer, 20 train, 21 tram, 22 tree,
23 van, 24 vegetation, 25 wind, 26 bridge.

**Outline fill type** (`object_outline_info().fill_type`): 0 grass, 1 concrete,
2 cobble, 3 asphalt, 4 pavement, 5 gravel, 6 soil, 7 undefined.

**Parking access** (`road_objects().parking_access`): 0 all, 1 bus, 2 car,
3 electric, 4 handicapped, 5 residents, 6 truck, 7 women.

**Road type** (`road_type()`): 0 unknown, 1 rural, 2 motorway, 3 town,
4 low-speed, 5 pedestrian, 6 bicycle, 7 town-arterial, 8 town-collector,
9 town-expressway, 10 town-local, 11 town-play-street, 12 town-private.

**Road rule** (`road_rule()`): 0 right-hand traffic, 1 left-hand traffic.

**Speed unit** (`network_info().speed_unit`): 0 undefined, 1 km/h, 2 m/s, 3 mph.

---

## `OsiRoadNetworkVisualizer` — properties & methods

A `Node3D` demo helper that renders a loaded network as child nodes: a
`RoadSurface` `MeshInstance3D`, a `RoadMarks` `MeshInstance3D`, and a `Signs`
node holding a small marker box at each sign. Roads are static, so it builds once
on demand (no per-frame polling).

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `scale` | float | `1.0` | Uniform scale forwarded to the coordinate mapping (meters by default). |
| `sample_step` | float | `1.0` | Longitudinal sampling stride (m) for the road-surface mesh. |
| `show_signs` | bool | `true` | Drop a marker box at each road sign. |
| `show_road_marks` | bool | `true` | Draw the OpenDRIVE lane markings. |

### Methods
| Method | Returns | Description |
|---|---|---|
| `build_from(network: OsiRoadNetwork)` | void | (Re)build the visible road from `network`. Frees any previous render. |
| `has_surface()` | bool | Whether a road-surface mesh is currently shown. |

---

## Scope & limitations

- **One network at a time** (global singleton, see Quickstart).
- The surface mesh is a flat-shaded asphalt triangulation sampled along each lane;
  use the query API (OSI points, lanes, road marks) to build your own geometry if
  you need more control or per-lane-type materials.
- **OpenSCENARIO** constructs (routes, trajectories, trajectory shapes) are
  produced by a scenario engine, not by loading an `.xodr`, so they are not part
  of this API. Network-level routing is available via `shortest_path_distance()`.
- Supported OpenDRIVE features follow esmini's RoadManager (OpenDRIVE ~1.5+):
  roads, lane sections/lanes, road marks, objects, junctions, signals, elevation/
  super-elevation, geometry primitives. Unsupported elements in newer files are
  ignored on load.

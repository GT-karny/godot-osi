# godot-osi — API Reference

The node classes exposed to GDScript by the addon. For the typed snapshot
`Resource` classes carried by the converter's signals (`OsiGroundTruth`,
`OsiMovingObject`, …) see [SCHEMA.md](SCHEMA.md); for wiring and coordinate-system
notes see [README.md](README.md).

The OpenDRIVE road classes (`OsiRoadNetwork`, `OsiRoadNetworkVisualizer`) are a
separate feature documented in full — methods, returned dictionary shapes, and
enum tables — in [ROADS.md](ROADS.md).

Defaults and types below are exactly what the binary registers. `int` is Godot's
64-bit integer; `float` properties noted as such are 32-bit (`real`).

---

## OsiReceiver  *(extends Node)*

Streams OSI frames from a gRPC server on a background thread and stores the
newest frame on a shared bus. Add it to the tree, configure, then
`connect_to_server()`.

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `address` | String | `"127.0.0.1"` | Server host. |
| `port` | int | `50051` | Server port. |
| `use_tls` | bool | `false` | Use TLS (`https`) instead of plaintext (`http`). |
| `reconnect` | bool | `true` | Automatically reconnect when the stream drops. |
| `reconnect_delay_ms` | int | `1000` | Delay between reconnect attempts (ms). |

### Methods
| Method | Returns | Description |
|---|---|---|
| `connect_to_server()` | void | Start streaming. Spawns the background thread; restarts if already running. |
| `disconnect_from_server()` | void | Stop streaming and join the background thread. |
| `is_streaming()` | bool | Whether a streaming thread is currently running. |
| `start_recording(ground_truth_path: String, host_vehicle_path: String)` | void | Record incoming frames to `.osi` trace files. Empty string skips that stream. Can be toggled while connected. |
| `stop_recording()` | void | Stop recording and flush both trace files. |

### Signals
| Signal | Description |
|---|---|
| `connection_state_changed(state: int)` | Connection state changed; `state` is one of the `STATE_*` constants. Emitted on the main thread. |
| `ground_truth_received()` | A new GroundTruth frame was stored on the bus. |
| `host_vehicle_data_received()` | A new HostVehicleData frame was stored on the bus. |
| `stream_error(message: String)` | A non-fatal stream/connection error occurred. |

### Constants
`STATE_DISCONNECTED = 0`, `STATE_CONNECTING = 1`, `STATE_CONNECTED = 2`,
`STATE_RECONNECTING = 3`, `STATE_ERROR = 4`.

---

## OsiConverter  *(extends Node)*

Each frame, drains the newest raw OSI frame from the shared bus, converts it to a
typed Godot `Resource` snapshot, and emits it. Does not spawn or mutate scene
nodes — that is the helpers' job.

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `auto_poll` | bool | `true` | When `true`, drains the bus every `_process`. Set `false` to drive it manually via `poll()`. |

### Methods
| Method | Returns | Description |
|---|---|---|
| `connect_source(receiver: OsiReceiver)` | void | Share the receiver's frame bus so this converter consumes what the receiver produces. Safe to call before/after the receiver connects, and across reconnects. |
| `poll()` | void | Drain whatever is on the bus now and emit conversions. Safe to call manually regardless of `auto_poll`. |

### Signals
| Signal | Description |
|---|---|
| `ground_truth_converted(snapshot: OsiGroundTruth)` | One converted GroundTruth frame. See [SCHEMA.md](SCHEMA.md#osigroundtruth). |
| `host_vehicle_data_converted(snapshot: OsiHostVehicleData)` | One converted HostVehicleData frame. |

---

## OsiMockServer  *(extends Node)*

A bundled mock gRPC server for developing the Godot side without a real backend.
Serves synthetic frames (two vehicles + one pedestrian on a circular path) or
replays `.osi` traces. Point an `OsiReceiver` at it.

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `address` | String | `"127.0.0.1"` | Bind host. |
| `port` | int | `50051` | Bind port. |
| `period_ms` | int | `50` | Milliseconds between emitted frames. |

### Methods
| Method | Returns | Description |
|---|---|---|
| `set_traces(ground_truth_path: String, host_vehicle_path: String)` | void | Replay these `.osi` files (call before `start()`). Empty string falls back to synthetic data for that stream. |
| `start()` | void | Start serving. Restarts if already running. |
| `stop()` | void | Stop serving and join the background thread. |

---

## OsiMovingObjectSpawner  *(extends Node3D)*

Sample helper that keeps one **empty** child `Node3D` per `MovingObject` in each
converted GroundTruth, transformed into Godot space (default `AxisMapping`).
Attach your own meshes as children of the spawned nodes. Applies the OSI→Godot
coordinate transform for you (see [README.md](README.md#coordinate-systems--important)).

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `scale` | float | `1.0` | Uniform scale forwarded to the coordinate mapping (meters by default). |

### Methods
| Method | Returns | Description |
|---|---|---|
| `bind_converter(converter: OsiConverter)` | void | Connect to the converter's `ground_truth_converted` so this spawner updates automatically. |
| `on_ground_truth(snapshot: OsiGroundTruth)` | void | Reconcile tracked child nodes against the objects in `snapshot`. (Auto-called once bound.) |
| `tracked_count()` | int | Number of currently tracked objects. |

---

## OsiMovingObjectVisualizer  *(extends Node3D)*

Demo helper that spawns a visible `MeshInstance3D` box per `MovingObject`, sized
by OSI dimension and colored by type: **vehicle = blue, pedestrian = green,
animal = orange** (others gray). Applies the OSI→Godot transform like the spawner.

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `scale` | float | `1.0` | Uniform scale forwarded to the coordinate mapping (meters by default). |

### Methods
| Method | Returns | Description |
|---|---|---|
| `bind_converter(converter: OsiConverter)` | void | Connect to the converter's `ground_truth_converted` so the boxes update every frame. |
| `on_ground_truth(snapshot: OsiGroundTruth)` | void | Reconcile the visible boxes against the objects in `snapshot`. (Auto-called once bound.) |
| `tracked_count()` | int | Number of currently visible boxes. |

---

## OsiRoadNetwork  *(extends Resource)*

Loads an OpenDRIVE (`.xodr`) map and answers geometry / lane / object / signal /
topology / routing queries. Unrelated to the OSI streaming pipeline above. Full
reference — every method, the returned `Dictionary` shapes, and the enum tables —
is in **[ROADS.md](ROADS.md)**.

### Methods (summary)
| Method | Returns | Description |
|---|---|---|
| `load(path: String)` | bool | Load a `.xodr` (releases any previous map; singleton). |
| `is_loaded()` | bool | Whether a network is loaded. |
| `road_count()` / `road_id_at(index)` / `road_length(road_id)` | int / int / float | Road enumeration and length. |
| `world_position(road_id, s, t)` / `lane_point(road_id, lane_id, s)` | Vector3 | Godot-space points along the road / lane. |
| `drivable_lanes(road_id, s)` | PackedInt32Array | Drivable lane ids at `s`. |
| `geometries` / `lane_osi_points` / `lane_sections` / `lanes` | Array[Dictionary] | Reference-line geometry, OSI points, lane structure. |
| `road_marks` / `road_objects` / `tunnels` / `signals` | Array[Dictionary] | Lane markings, objects, tunnels, full signal detail. |
| `junctions` / `junction_connections` / `junction_lane_links` / `controllers` | Array[Dictionary] | Network topology. |
| `road_link` / `network_info` / `geo_offset` | Dictionary | Road links and network metadata. |
| `elevations` / `super_elevations` | Array[Dictionary] | Elevation / cross-slope profiles. |
| `lane_center_offset` / `lane_friction` / `lane_offset` / `road_speed` / `road_width` | float | Scalar queries (`NAN` on error). |
| `road_rule` / `road_type` | int | Traffic rule / road type (`-1` on error). |
| `shortest_path_distance(road_a, s_a, road_b, s_b)` | float | Network routing distance (`NAN` if no path). |

### Constants
`OSI_LANE = 0`, `OSI_REF_LINE = 1`, `OSI_BOUNDARY = 2` (for `lane_osi_points`);
`LINK_PREDECESSOR = -1`, `LINK_SUCCESSOR = 1` (for `road_link`).

---

## OsiRoadNetworkVisualizer  *(extends Node3D)*

Optional demo helper that renders an `OsiRoadNetwork` as child nodes (road
surface + lane markings + sign markers). Builds once on demand. See
[ROADS.md](ROADS.md#osiroadnetworkvisualizer--properties--methods).

### Properties
| Property | Type | Default | Description |
|---|---|---|---|
| `scale` | float | `1.0` | Uniform scale forwarded to the coordinate mapping. |
| `sample_step` | float | `1.0` | Longitudinal sampling stride (m) for the surface mesh. |
| `show_signs` | bool | `true` | Drop a marker box at each road sign. |
| `show_road_marks` | bool | `true` | Draw the OpenDRIVE lane markings. |

### Methods
| Method | Returns | Description |
|---|---|---|
| `build_from(network: OsiRoadNetwork)` | void | (Re)build the visible road. Frees any previous render. |
| `has_surface()` | bool | Whether a road-surface mesh is currently shown. |

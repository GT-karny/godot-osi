# godot-osi — Godot 4 ASAM OSI receiver/converter

A native GDExtension that receives [ASAM OSI](https://www.asam.net/standards/detail/osi/)
(Open Simulation Interface, v3.7.0) over gRPC and exposes it to Godot as typed
`Resource` objects and ready-to-use scene nodes.

- **Receive**: `GroundTruthService.StreamGroundTruth` and
  `HostVehicleDataService.StreamHostVehicleData` (server-streaming gRPC).
- **Convert**: every OSI message becomes a typed Godot `Resource` (`OsiGroundTruth`,
  `OsiMovingObject`, …). See [SCHEMA.md](SCHEMA.md) for the full list.
- **Target**: Godot 4.6+, Windows / Linux / macOS x86_64 (macOS also arm64).

This package is **runtime-only**: drop it in and use it from GDScript. No Rust
toolchain or build step is required.

## Install

1. Unzip so that the addon lands at `res://addons/godot_osi/` in your project:
   ```
   <your project>/addons/godot_osi/
   ├─ godot_osi.gdextension
   ├─ bin/{windows,linux,macos}/…
   ├─ README.md  API.md  ROADS.md  SCHEMA.md
   └─ third_party/osi3/…  (proto sources + licenses)
   ```
2. Restart the Godot editor. On load you should see in the log:
   ```
   Initialize godot-rust (API v4.6.stable.official, …)
   ```
   The classes `OsiReceiver`, `OsiConverter`, `OsiMockServer`,
   `OsiMovingObjectSpawner`, `OsiMovingObjectVisualizer`, `OsiHostVehicleState`
   — plus the OpenDRIVE road classes `OsiRoadNetwork` and
   `OsiRoadNetworkVisualizer` — are now available in GDScript and the
   "Create Node" dialog.

There is no `plugin.cfg` / EditorPlugin — nothing to enable under
*Project Settings → Plugins*. Loading the GDExtension is all that is needed.

## Quickstart

Wire a receiver to a converter (they share one internal frame bus), then turn
converted frames into scene nodes. Attach this to a `Node3D` and run:

```gdscript
extends Node3D

func _ready() -> void:
    # 1. Receiver: streams raw OSI frames from a gRPC server into its frame bus.
    var receiver := OsiReceiver.new()
    receiver.address = "127.0.0.1"
    receiver.port = 50051
    add_child(receiver)

    # 2. Converter: shares the receiver's bus, emits typed Godot snapshots.
    var converter := OsiConverter.new()
    add_child(converter)
    converter.connect_source(receiver)        # <-- the integration wiring

    # 3. (optional) Spawner: a tracked child Node3D per MovingObject.
    var spawner := OsiMovingObjectSpawner.new()
    add_child(spawner)
    spawner.bind_converter(converter)

    # Or consume the typed snapshot yourself:
    converter.ground_truth_converted.connect(func(snap):
        print("moving objects: ", snap.moving_object.size()))

    receiver.connect_to_server()
```

`connect_source()` may be called before or after `connect_to_server()`, and the
shared bus survives reconnects. The bus is **newest-wins**: a slow consumer only
ever sees the latest frame and silently drops intermediate ones.

### No server? Use the bundled mock

`OsiMockServer` streams synthetic frames (two vehicles + one pedestrian moving in
a circle) — or replays recorded `.osi` traces — so you can build the Godot side
without a real simulator:

```gdscript
var mock := OsiMockServer.new()
mock.address = "127.0.0.1"
mock.port = 50051
mock.period_ms = 33
add_child(mock)
mock.start()        # now point an OsiReceiver at 127.0.0.1:50051
```

## Coordinate systems — important

The typed `Resource` snapshots mirror the OSI values **1:1, with no coordinate
transform** (raw mirror). OSI is a **right-handed, Z-up, metric** world frame;
Godot is **left-handed, Y-up**. So if you read positions/orientations straight
off the snapshots, you must convert them yourself.

The bundled helper nodes already do this conversion for you. Their default
mapping is:

- Position: `Godot(x, y, z) = OSI(x, z, -y)`
- Dimension: OSI `length → x`, `height → y`, `width → z`
- Orientation: the matching similarity transform (scale never affects rotation)

Use **`OsiMovingObjectSpawner`** (empty tracked `Node3D` per object — attach your
own meshes as children) or **`OsiMovingObjectVisualizer`** (colored boxes sized by
OSI dimension: vehicle = blue, pedestrian = green, animal = orange) and bind it to
the converter with `bind_converter(converter)`.

## Dashboard / HMI (ego vehicle state)

For a meter cluster or ADAS display, `OsiHostVehicleState` gives you the common
ego quantities (speed, gear, rpm, pedals, steering, lights) in one call instead
of walking the nested `OsiHostVehicleData` snapshot:

```gdscript
var ego := OsiHostVehicleState.new()
add_child(ego)
ego.bind_converter(converter)        # same converter as above

func _process(_dt):
    if ego.is_ready():
        var s := ego.ego_state()
        $SpeedLabel.text = "%d km/h" % int(s["speed_kph"])
        $GearLabel.text = "DNR"[clampi(s["gear"] + 1, 0, 2)]  # toy example
```

The bundled `OsiMockServer` streams a synthetic cruising ego (~30–65 km/h in a
forward gear), so the cluster moves without a real simulator. See
[API.md](API.md#osihostvehiclestate--extends-node) for every field.

## OpenDRIVE roads

Independently of the OSI stream, the addon can load an **OpenDRIVE** (`.xodr`)
road network and query or render it — no server or scenario engine required:

```gdscript
var net := OsiRoadNetwork.new()
net.load("res://maps/my_road.xodr")

var viz := OsiRoadNetworkVisualizer.new()
add_child(viz)
viz.build_from(net)              # road surface + lane markings + sign markers
```

`OsiRoadNetwork` exposes the full road structure to GDScript — reference-line
geometry, lane sections/lanes, road marks, objects, junctions, signals,
elevation profiles, and shortest-path routing. See **[ROADS.md](ROADS.md)** for
the complete method list, returned dictionary shapes, and enum tables.

## Recording & replay

`OsiReceiver.start_recording(gt_path, hvd_path)` writes incoming frames to `.osi`
trace files (length-delimited protobuf, the OSI convention); pass an empty string
to skip a stream. `OsiMockServer.set_traces(gt_path, hvd_path)` replays such files
instead of synthesizing data.

## Reference

- [API.md](API.md) — the node classes (`OsiReceiver`, `OsiConverter`,
  `OsiMockServer`, `OsiMovingObjectSpawner`, `OsiMovingObjectVisualizer`,
  `OsiHostVehicleState`): properties, methods, signals, constants.
- [ROADS.md](ROADS.md) — the OpenDRIVE road API (`OsiRoadNetwork`,
  `OsiRoadNetworkVisualizer`): methods, returned dictionary shapes, enum tables.
- [SCHEMA.md](SCHEMA.md) — every generated `Osi*` `Resource` class and its fields.
- `third_party/osi3/*.proto` — the OSI message definitions (field units, meaning,
  and validity rules live here).

## License

This addon's own code is `MIT OR Apache-2.0` (see `LICENSE-MIT` / `LICENSE-APACHE`).
The binaries also incorporate MPL-2.0 components (the ASAM OSI generated types,
the godot-rust bindings, and esmini's RoadManager used for OpenDRIVE) plus MIT
components (pugixml, fmt); their source/notices are in `THIRD_PARTY_NOTICES.md`
and `third_party/osi3/`. Your own game/application code is unaffected.

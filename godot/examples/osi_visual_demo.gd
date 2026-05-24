## Visual OSI demo: receive -> convert -> draw colored boxes.
##
## Pipeline: OsiReceiver --shared OsiFrameBus--> OsiConverter
##           --ground_truth_converted--> OsiMovingObjectVisualizer
##
## Each MovingObject is drawn as a box sized by its OSI `dimension`, colored by
## its type (vehicle=blue, pedestrian=green, animal=orange, other/unknown=gray),
## and oriented by its yaw. Coordinate conversion (OSI right-handed Z-up ->
## Godot left-handed Y-up) is done in the native converter.
##
## The camera chases the host vehicle (the MovingObject whose id equals the
## GroundTruth host_vehicle_id) from behind. Set follow_host = false (or pass
## `-- --no-follow`) for a fixed overhead view.
##
## Run this scene (examples/osi_visual_demo.tscn). With `use_mock = true` it
## starts the bundled mock server, so no external server is needed. Set
## `use_mock = false` (and host/port) to visualize a real gRPC OSI source.
extends Node3D

@export var use_mock: bool = true
@export var host: String = "127.0.0.1"
@export var port: int = 50051

## OpenDRIVE map to render under the OSI objects. Empty disables the road.
## Loaded via the native OsiRoadNetwork (esmini RoadManager). Note: the mock OSI
## data is not tied to this map, so boxes won't literally follow the lanes — the
## road is drawn in the same coordinate frame to show the integration.
@export var road_file: String = "res://examples/roads/e6mini.xodr"

## Camera chases the host vehicle from behind when true.
@export var follow_host: bool = true
## Distance the camera trails behind the host (meters).
@export var chase_distance: float = 10.0
## Camera height above the host (meters).
@export var chase_height: float = 4.0
## Follow responsiveness (higher = snappier).
@export var follow_speed: float = 4.0

## When a road is loaded, how many cars to drive along its drivable lanes.
@export var car_count: int = 3
## Road-driving speed (m/s) for those cars.
@export var drive_speed: float = 12.0

var receiver: OsiReceiver
var converter: OsiConverter
var viz: OsiMovingObjectVisualizer
var mock                 # OsiMockServer when use_mock
var road_net: OsiRoadNetwork
var road_viz: OsiRoadNetworkVisualizer
var cam: Camera3D
var _hud: Label
var _host_id: int = -1

# Road-driving mode: cars driven along the loaded road's lanes (used instead of
# the OSI mock pipeline when --road is given, since mock OSI data is not tied to
# the map). Each entry: {node: MeshInstance3D, lane_id: int, s: float}.
var _road_cars: Array = []
var _drive_road_id: int = -1
var _drive_len: float = 0.0
var _road_mode: bool = false

func _ready() -> void:
	_apply_cmdline_overrides()
	_setup_environment()
	_setup_hud()
	_setup_road()

	# With a road loaded, drive cars along its lanes. Otherwise run the OSI
	# mock/receiver/converter pipeline and visualize the moving objects.
	if _road_mode:
		_setup_road_cars()
		print("[visual] road-drive mode: %d cars on road %d (len %.1f m)" % [
			_road_cars.size(), _drive_road_id, _drive_len])
		return

	if use_mock:
		mock = OsiMockServer.new()
		mock.address = host
		mock.port = port
		mock.period_ms = 33
		add_child(mock)
		mock.start()

	receiver = OsiReceiver.new()
	receiver.address = host
	receiver.port = port
	add_child(receiver)

	converter = OsiConverter.new()
	add_child(converter)
	converter.connect_source(receiver)        # integration wiring
	converter.ground_truth_converted.connect(_on_ground_truth)

	viz = OsiMovingObjectVisualizer.new()
	add_child(viz)
	viz.bind_converter(converter)

	receiver.connect_to_server()
	print("[visual] source=%s %s:%d" % ["mock" if use_mock else "external", host, port])

## Track which object is the host (ego) vehicle.
func _on_ground_truth(snapshot) -> void:
	if snapshot.host_vehicle_id != null:
		_host_id = snapshot.host_vehicle_id.value

func _process(delta: float) -> void:
	if _road_mode:
		_update_road_cars(delta)
		if follow_host and not _road_cars.is_empty():
			var t := (_road_cars[0]["node"] as Node3D).global_transform
			# look_at() points the car's -Z along travel, so forward = -basis.z.
			_update_chase_target(t.origin, -t.basis.z.normalized(), delta)
		if _hud:
			_hud.text = "road-drive: %d cars on road %d (%.0f m)" % [
				_road_cars.size(), _drive_road_id, _drive_len]
		return

	if follow_host:
		_update_chase_camera(delta)
	if _hud and viz:
		_hud.text = "source: %s   objects: %d   host: %s" % [
			"mock" if use_mock else "external",
			viz.tracked_count(),
			str(_host_id) if _host_id >= 0 else "?"]

## Spawn cars on the loaded road's drivable lanes and place them at staggered
## starting distances. Driven each frame by _update_road_cars().
func _setup_road_cars() -> void:
	_drive_road_id = road_net.road_id_at(0)
	if _drive_road_id < 0:
		return
	_drive_len = road_net.road_length(_drive_road_id)
	var lanes := road_net.drivable_lanes(_drive_road_id, 0.0)
	if lanes.is_empty():
		print("[visual] no drivable lanes on road %d" % _drive_road_id)
		return
	var n: int = min(car_count, lanes.size())
	for i in range(n):
		var car := _make_car(Color(0.20, 0.45, 0.90, 1.0))
		add_child(car)
		_road_cars.append({
			"node": car,
			"lane_id": lanes[i],
			# Stagger start positions along the road.
			"s": _drive_len * float(i) / float(n),
		})

## Advance each car along its lane and orient it along the travel direction.
func _update_road_cars(delta: float) -> void:
	if _drive_len <= 1.0:
		return
	for car in _road_cars:
		var s: float = fmod(car["s"] + drive_speed * delta, _drive_len)
		car["s"] = s
		var lane: int = car["lane_id"]
		var pos := road_net.lane_point(_drive_road_id, lane, s)
		var ahead := road_net.lane_point(_drive_road_id, lane, fmod(s + 2.0, _drive_len))
		var node := car["node"] as Node3D
		# lane_point is on the road surface; lift the box by half its height
		# (0.75) so it sits on the road instead of being half-buried.
		node.global_position = pos + Vector3.UP * 0.75
		var dir := (ahead - pos)
		if dir.length() > 0.001:
			node.look_at(node.global_position + dir, Vector3.UP)

## Place the camera behind the host's heading. The host's forward direction in
## Godot space is the first column of its transform basis (see coords.rs: the
## OSI +x/forward axis maps to basis.x).
func _update_chase_camera(delta: float) -> void:
	if _host_id < 0 or viz == null:
		return
	var ego := viz.get_node_or_null(NodePath("osi_mo_%d" % _host_id)) as Node3D
	if ego == null:
		return
	var t := ego.global_transform
	_update_chase_target(t.origin, t.basis.x.normalized(), delta)

## Smoothly trail the camera behind `origin`, looking along `fwd`.
func _update_chase_target(origin: Vector3, fwd: Vector3, delta: float) -> void:
	var eye := origin - fwd * chase_distance + Vector3.UP * chase_height
	var weight: float = clampf(delta * follow_speed, 0.0, 1.0)
	cam.global_position = cam.global_position.lerp(eye, weight)
	cam.look_at(origin + fwd * 3.0, Vector3.UP)

## A simple car box (length along local -Z to match look_at orientation).
func _make_car(color: Color) -> MeshInstance3D:
	var mesh := BoxMesh.new()
	mesh.size = Vector3(2.0, 1.5, 4.5)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	var mi := MeshInstance3D.new()
	mi.mesh = mesh
	mi.material_override = mat
	return mi

## Override the exported defaults from user args passed after `--`, e.g.:
##   Godot ... osi_visual_demo.tscn -- --external --host 127.0.0.1 --port 50051
##   Godot ... osi_visual_demo.tscn -- --mock --no-follow
func _apply_cmdline_overrides() -> void:
	var args := OS.get_cmdline_user_args()
	var i := 0
	while i < args.size():
		match args[i]:
			"--external":
				use_mock = false
			"--mock":
				use_mock = true
			"--follow":
				follow_host = true
			"--no-follow":
				follow_host = false
			"--host":
				i += 1
				if i < args.size():
					host = args[i]
			"--port":
				i += 1
				if i < args.size():
					port = int(args[i])
			"--road":
				i += 1
				if i < args.size():
					road_file = args[i]
			"--no-road":
				road_file = ""
		i += 1

## Load the OpenDRIVE map (if any) and render it under the OSI objects.
func _setup_road() -> void:
	if road_file == "":
		return
	road_net = OsiRoadNetwork.new()
	if not road_net.load(road_file):
		print("[visual] road load failed: %s" % road_file)
		road_net = null
		return
	road_viz = OsiRoadNetworkVisualizer.new()
	add_child(road_viz)
	road_viz.build_from(road_net)
	_road_mode = true
	print("[visual] road=%s roads=%d signs=%d" % [
		road_file, road_net.road_count(), road_net.sign_count()])

func _setup_environment() -> void:
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-55.0, -40.0, 0.0)
	light.shadow_enabled = true
	add_child(light)

	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(200.0, 200.0)
	ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = Color(0.14, 0.15, 0.17)
	ground.material_override = gmat
	add_child(ground)

	cam = Camera3D.new()
	cam.position = Vector3(0.0, 18.0, 26.0)
	add_child(cam)
	cam.look_at(Vector3.ZERO, Vector3.UP)
	cam.current = true

func _setup_hud() -> void:
	var canvas := CanvasLayer.new()
	add_child(canvas)
	_hud = Label.new()
	_hud.position = Vector2(12.0, 10.0)
	canvas.add_child(_hud)

func _exit_tree() -> void:
	if receiver:
		receiver.disconnect_from_server()
	if mock:
		mock.stop()

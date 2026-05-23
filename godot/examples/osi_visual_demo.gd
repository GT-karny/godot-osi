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

## Camera chases the host vehicle from behind when true.
@export var follow_host: bool = true
## Distance the camera trails behind the host (meters).
@export var chase_distance: float = 10.0
## Camera height above the host (meters).
@export var chase_height: float = 4.0
## Follow responsiveness (higher = snappier).
@export var follow_speed: float = 4.0

var receiver: OsiReceiver
var converter: OsiConverter
var viz: OsiMovingObjectVisualizer
var mock                 # OsiMockServer when use_mock
var cam: Camera3D
var _hud: Label
var _host_id: int = -1

func _ready() -> void:
	_apply_cmdline_overrides()
	_setup_environment()
	_setup_hud()

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
	if follow_host:
		_update_chase_camera(delta)
	if _hud and viz:
		_hud.text = "source: %s   objects: %d   host: %s" % [
			"mock" if use_mock else "external",
			viz.tracked_count(),
			str(_host_id) if _host_id >= 0 else "?"]

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
	var fwd := t.basis.x.normalized()
	var eye := t.origin - fwd * chase_distance + Vector3.UP * chase_height
	var weight: float = clampf(delta * follow_speed, 0.0, 1.0)
	cam.global_position = cam.global_position.lerp(eye, weight)
	cam.look_at(t.origin + fwd * 3.0, Vector3.UP)

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
		i += 1

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

## OSI reception sample: drive an OpenDRIVE intersection's traffic lights and
## vehicles from a live OSI stream.
##
## Pipeline:
##   OsiReceiver --OsiFrameBus--> OsiConverter --ground_truth_converted-->
##       - OsiMovingObjectVisualizer : draws each MovingObject as a box
##       - _drive_lights()           : maps GroundTruth.traffic_light[] onto the
##                                     OsiTrafficLightVisualizer heads
##
## The fabriksgatan_traffic_lights.xodr map is rendered with its road surface,
## lane marks, and 3D traffic-light heads (OsiRoadNetworkVisualizer +
## OsiTrafficLightVisualizer). The OSI stream then lights those heads and moves
## the vehicles.
##
## This is primarily an *external* reception sample: point it at a real OSI gRPC
## source (e.g. esmini with OSI output) of the same map with
##   Godot ... osi_traffic_demo.tscn -- --external --host <ip> --port <port>
## Offline, pass nothing: the bundled mock server (use_mock = true) streams
## demo objects + alternating light phases so the scene is self-contained. With
## a real source, traffic lights bind to map signals by world position; the mock
## (whose objects are not tied to the map) falls back to index order.
extends Node3D

@export var use_mock: bool = true
@export var host: String = "127.0.0.1"
@export var port: int = 50051

## OpenDRIVE intersection to render. Its dynamic signals become 3D lamp heads.
@export var road_file: String = "res://examples/roads/fabriksgatan_traffic_lights.xodr"

## Free-fly camera (UE-style): hold right mouse to look, WASD to move,
## E up / Q down, hold Shift to move faster. Takes priority over follow_host.
@export var free_camera: bool = true
@export var fly_speed: float = 20.0       # m/s
@export var fly_fast_mult: float = 4.0    # Shift multiplier
@export var mouse_sensitivity: float = 0.004

## Camera chases the host vehicle (matched by host_vehicle_id) when true and
## free_camera is false.
@export var follow_host: bool = false
@export var chase_distance: float = 12.0
@export var chase_height: float = 6.0
@export var follow_speed: float = 4.0

# OSI TrafficLight.Classification enum values (osi_trafficlight.proto).
const OSI_COLOR := {2: "red", 3: "yellow", 4: "green"}
const OSI_MODE_OFF := 2
# How close (m) an OSI light must be to a head to bind by position.
const LIGHT_MATCH_DIST := 4.0

var receiver: OsiReceiver
var converter: OsiConverter
var obj_viz: OsiMovingObjectVisualizer
var mock                       # OsiMockServer when use_mock
var road_net: OsiRoadNetwork
var road_viz: OsiRoadNetworkVisualizer
var tl_viz: OsiTrafficLightVisualizer
var cam: Camera3D
var _hud: Label
var _host_id: int = -1
var _lit_count: int = 0

# Free-fly camera state.
var _yaw: float = 0.0
var _pitch: float = -0.6
var _looking: bool = false

func _ready() -> void:
	_apply_cmdline_overrides()
	_setup_environment()
	_setup_hud()
	if not _setup_road():
		push_error("[tl-demo] could not load %s" % road_file)
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
	converter.connect_source(receiver)
	converter.ground_truth_converted.connect(_on_ground_truth)

	obj_viz = OsiMovingObjectVisualizer.new()
	add_child(obj_viz)
	obj_viz.bind_converter(converter)

	receiver.connect_to_server()
	print("[tl-demo] source=%s %s:%d  heads=%d" % [
		"mock" if use_mock else "external", host, port, tl_viz.head_count()])

func _on_ground_truth(snapshot) -> void:
	if snapshot.host_vehicle_id != null:
		_host_id = snapshot.host_vehicle_id.value
	_drive_lights(snapshot)

## Map the GroundTruth's traffic_light[] onto the rendered heads. Each lit OSI
## bulb (mode != OFF) binds to the nearest head by world position; if no head is
## close (e.g. the mock's objects are not tied to this map), fall back to index.
func _drive_lights(snapshot) -> void:
	if tl_viz == null:
		return
	var ids := tl_viz.global_ids()
	for id in ids:
		tl_viz.all_off(id)

	var lights = snapshot.traffic_light
	_lit_count = 0
	for i in range(lights.size()):
		var tl = lights[i]
		var cls = tl.classification
		if cls == null or cls.mode == OSI_MODE_OFF:
			continue
		var color: String = OSI_COLOR.get(cls.color, "")
		if color == "":
			continue
		var matched := -1
		if tl.base != null and tl.base.position != null:
			var p = tl.base.position
			matched = tl_viz.set_state_at_osi(p.x, p.y, p.z, color, LIGHT_MATCH_DIST)
		if matched < 0 and i < ids.size():
			tl_viz.set_color_state(ids[i], color)
		_lit_count += 1

func _process(delta: float) -> void:
	if free_camera:
		_update_free_camera(delta)
	elif follow_host:
		_update_chase_camera(delta)
	if _hud:
		_hud.text = "source: %s   objects: %d   heads: %d   lit: %d   host: %s\n%s" % [
			"mock" if use_mock else "external",
			obj_viz.tracked_count() if obj_viz else 0,
			tl_viz.head_count() if tl_viz else 0,
			_lit_count,
			str(_host_id) if _host_id >= 0 else "?",
			"[fly] hold RMB + mouse to look, WASD move, E/Q up/down, Shift faster" if free_camera else ""]

## Free-fly camera input: RMB captures the mouse for look; release frees it.
func _unhandled_input(event: InputEvent) -> void:
	if not free_camera:
		return
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
		_looking = event.pressed
		Input.mouse_mode = Input.MOUSE_MODE_CAPTURED if _looking else Input.MOUSE_MODE_VISIBLE
	elif event is InputEventMouseMotion and _looking:
		_yaw -= event.relative.x * mouse_sensitivity
		_pitch = clampf(_pitch - event.relative.y * mouse_sensitivity, -1.55, 1.55)

## Apply yaw/pitch to the camera and move it with WASDQE (relative to view).
func _update_free_camera(delta: float) -> void:
	if cam == null:
		return
	var basis := Basis(Vector3.UP, _yaw) * Basis(Vector3.RIGHT, _pitch)
	var dir := Vector3.ZERO
	if Input.is_key_pressed(KEY_W): dir -= basis.z      # forward = -Z
	if Input.is_key_pressed(KEY_S): dir += basis.z
	if Input.is_key_pressed(KEY_A): dir -= basis.x
	if Input.is_key_pressed(KEY_D): dir += basis.x
	if Input.is_key_pressed(KEY_E): dir += Vector3.UP
	if Input.is_key_pressed(KEY_Q): dir -= Vector3.UP
	var pos := cam.global_position
	if dir.length() > 0.001:
		var spd := fly_speed * (fly_fast_mult if Input.is_key_pressed(KEY_SHIFT) else 1.0)
		pos += dir.normalized() * spd * delta
	cam.global_transform = Transform3D(basis, pos)

## Chase the host vehicle from behind (its forward axis is basis.x; see coords.rs).
func _update_chase_camera(delta: float) -> void:
	if _host_id < 0 or obj_viz == null:
		return
	var ego := obj_viz.get_node_or_null(NodePath("osi_mo_%d" % _host_id)) as Node3D
	if ego == null:
		return
	var t := ego.global_transform
	var fwd := t.basis.x.normalized()
	var eye := t.origin - fwd * chase_distance + Vector3.UP * chase_height
	var weight: float = clampf(delta * follow_speed, 0.0, 1.0)
	cam.global_position = cam.global_position.lerp(eye, weight)
	cam.look_at(t.origin + fwd * 3.0, Vector3.UP)

## Load the map and render its road + traffic-light heads. Returns false on fail.
func _setup_road() -> bool:
	road_net = OsiRoadNetwork.new()
	if not road_net.load(road_file):
		road_net = null
		return false
	road_viz = OsiRoadNetworkVisualizer.new()
	# The dynamic signals are drawn as 3D traffic-light heads below, so suppress
	# the road visualizer's generic yellow sign-marker pillars at those spots.
	road_viz.show_signs = false
	add_child(road_viz)
	road_viz.build_from(road_net)

	tl_viz = OsiTrafficLightVisualizer.new()
	add_child(tl_viz)
	tl_viz.build_from(road_net)

	print("[tl-demo] road=%s roads=%d signs=%d lights=%d" % [
		road_file, road_net.road_count(), road_net.sign_count(), tl_viz.head_count()])
	return true

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
		i += 1

func _setup_environment() -> void:
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-55.0, -40.0, 0.0)
	light.shadow_enabled = true
	add_child(light)

	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(300.0, 300.0)
	ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = Color(0.14, 0.15, 0.17)
	ground.material_override = gmat
	ground.position = Vector3(0.0, -0.02, 0.0)
	add_child(ground)

	cam = Camera3D.new()
	cam.position = Vector3(0.0, 35.0, 45.0)
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

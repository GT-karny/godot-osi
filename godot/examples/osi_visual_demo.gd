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
## Run this scene (examples/osi_visual_demo.tscn). With `use_mock = true` it
## starts the bundled mock server, so no external server is needed. Set
## `use_mock = false` (and host/port) to visualize a real gRPC OSI source.
extends Node3D

@export var use_mock: bool = true
@export var host: String = "127.0.0.1"
@export var port: int = 50051

var receiver: OsiReceiver
var converter: OsiConverter
var viz: OsiMovingObjectVisualizer
var mock                 # OsiMockServer when use_mock
var _hud: Label

func _ready() -> void:
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

	viz = OsiMovingObjectVisualizer.new()
	add_child(viz)
	viz.bind_converter(converter)

	receiver.connect_to_server()
	print("[visual] source=%s %s:%d" % ["mock" if use_mock else "external", host, port])

func _process(_delta: float) -> void:
	if _hud and viz:
		_hud.text = "source: %s   objects: %d" % [
			"mock" if use_mock else "external", viz.tracked_count()]

func _setup_environment() -> void:
	var light := DirectionalLight3D.new()
	light.rotation_degrees = Vector3(-55.0, -40.0, 0.0)
	light.shadow_enabled = true
	add_child(light)

	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(80.0, 80.0)
	ground.mesh = plane
	var gmat := StandardMaterial3D.new()
	gmat.albedo_color = Color(0.14, 0.15, 0.17)
	ground.material_override = gmat
	add_child(ground)

	var cam := Camera3D.new()
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

## Demo integrator for the reusable OsiSettingsPanel.
##
## Shows how an app wires the decoupled panel to the actual OSI pipeline:
##   - Esc toggles the overlay panel.
##   - Connect (re)builds OsiMockServer/OsiReceiver -> OsiConverter ->
##     OsiMovingObjectVisualizer from the panel's config (reusing the wiring in
##     osi_visual_demo.gd).
##   - Load (re)builds OsiRoadNetwork + OsiRoadNetworkVisualizer from a path.
##   - Receiver status / road-load results are pushed back into the panel.
##
## The panel never touches these nodes itself — all pipeline ownership lives
## here, so the panel stays a drop-in component for any project.
extends Node3D

## When true, auto-apply the panel's last-selected presets on launch (connect
## and, if a road preset is set, load it). Off by default so launching the
## scene doesn't force a network connection.
@export var auto_start: bool = false

@onready var _panel: OsiSettingsPanel = $UiLayer/Panel

var receiver: OsiReceiver
var converter: OsiConverter
var viz: OsiMovingObjectVisualizer
var mock                      # OsiMockServer when use_mock
var road_net: OsiRoadNetwork
var road_viz: OsiRoadNetworkVisualizer
var cam: Camera3D
var _host_id: int = -1

func _ready() -> void:
	if not ClassDB.class_exists("OsiReceiver"):
		_panel.set_status_error("godot_osi native extension not loaded")
		push_error("[settings_demo] OsiReceiver missing — build the GDExtension dll first")
		return

	_setup_environment()

	_panel.apply_connection.connect(_on_apply_connection)
	_panel.disconnect_requested.connect(_on_disconnect_requested)
	_panel.load_road.connect(_on_load_road)
	_panel.hide()

	if auto_start:
		var cfg := _panel.get_config()
		if not cfg.road_path.is_empty():
			_on_load_road(cfg.road_path)
		_on_apply_connection(cfg)

func _unhandled_input(event: InputEvent) -> void:
	# Esc (ui_cancel) is a default action, so no project InputMap edits needed.
	if event.is_action_pressed("ui_cancel"):
		_panel.visible = not _panel.visible
		get_viewport().set_input_as_handled()

# --- Panel signal handlers -----------------------------------------------

## (Re)build the receive/convert/visualize pipeline from the panel config.
## Mirrors osi_visual_demo.gd:76-99, parameterized by `cfg`.
func _on_apply_connection(cfg: OsiSettingsConfig) -> void:
	_teardown_pipeline()

	if cfg.use_mock:
		mock = OsiMockServer.new()
		mock.address = cfg.address
		mock.port = cfg.port
		mock.period_ms = cfg.mock_period_ms
		add_child(mock)
		mock.start()

	receiver = OsiReceiver.new()
	receiver.address = cfg.address
	receiver.port = cfg.port
	receiver.use_tls = cfg.use_tls
	receiver.reconnect = cfg.reconnect
	receiver.reconnect_delay_ms = cfg.reconnect_delay_ms
	add_child(receiver)
	# Live status push-back into the panel.
	receiver.connection_state_changed.connect(_panel.set_connection_state)
	receiver.stream_error.connect(_panel.set_status_error)

	converter = OsiConverter.new()
	add_child(converter)
	converter.connect_source(receiver)              # integration wiring
	converter.ground_truth_converted.connect(_on_ground_truth)

	viz = OsiMovingObjectVisualizer.new()
	add_child(viz)
	viz.bind_converter(converter)

	receiver.connect_to_server()
	print("[settings_demo] source=%s %s:%d" % [
		"mock" if cfg.use_mock else "external", cfg.address, cfg.port])

func _on_disconnect_requested() -> void:
	if receiver:
		receiver.disconnect_from_server()

## (Re)load an OpenDRIVE map and rebuild its visualizer. Mirrors
## osi_visual_demo.gd:229-242, reporting the outcome back to the panel.
func _on_load_road(path: String) -> void:
	if path.is_empty():
		_panel.set_road_result(false, 0, 0, "no path given")
		return
	if road_viz:
		road_viz.queue_free()
		road_viz = null
	road_net = OsiRoadNetwork.new()
	if not road_net.load(path):
		road_net = null
		_panel.set_road_result(false, 0, 0, "load failed: %s" % path)
		return
	road_viz = OsiRoadNetworkVisualizer.new()
	add_child(road_viz)
	road_viz.build_from(road_net)
	_panel.set_road_result(true, road_net.road_count(), road_net.sign_count(), "")
	print("[settings_demo] road=%s roads=%d signs=%d" % [
		path, road_net.road_count(), road_net.sign_count()])

func _on_ground_truth(snapshot) -> void:
	if snapshot.host_vehicle_id != null:
		_host_id = snapshot.host_vehicle_id.value

# --- Lifecycle -----------------------------------------------------------

func _teardown_pipeline() -> void:
	if receiver:
		receiver.disconnect_from_server()
		receiver.queue_free()
		receiver = null
	if mock:
		mock.stop()
		mock.queue_free()
		mock = null
	if converter:
		converter.queue_free()
		converter = null
	if viz:
		viz.queue_free()
		viz = null

func _exit_tree() -> void:
	_teardown_pipeline()

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

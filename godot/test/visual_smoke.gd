# Headless smoke test for the visual helper (OsiMovingObjectVisualizer).
#
#   OsiMockServer -> OsiReceiver -> OsiConverter -> OsiMovingObjectVisualizer
#
# Verifies the visualizer spawns one box (MeshInstance3D) per MovingObject in
# the converted snapshot. Runs headless (node creation works without a display).
#
#   Godot ... --headless --path godot --script res://test/visual_smoke.gd
# Exits 0 once the expected number of boxes are tracked, 1 on timeout.
extends SceneTree

const PORT := 50082
const EXPECT_BOXES := 3      # the demo mock streams 3 actors
const TIMEOUT_S := 15.0

var server
var receiver
var converter
var viz
var elapsed := 0.0
var done := false

func _initialize() -> void:
	server = OsiMockServer.new()
	server.address = "127.0.0.1"
	server.port = PORT
	server.period_ms = 20
	root.add_child(server)
	server.start()

	receiver = OsiReceiver.new()
	receiver.address = "127.0.0.1"
	receiver.port = PORT
	root.add_child(receiver)

	converter = OsiConverter.new()
	converter.auto_poll = true
	root.add_child(converter)
	converter.connect_source(receiver)

	viz = OsiMovingObjectVisualizer.new()
	root.add_child(viz)
	viz.bind_converter(converter)

	receiver.connect_to_server()
	print("[visual] mock+receiver+converter+visualizer started on :%d" % PORT)

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta
	if viz.tracked_count() >= EXPECT_BOXES:
		# Confirm the tracked nodes really are visible mesh instances.
		var boxes := 0
		for child in viz.get_children():
			if child is MeshInstance3D and child.mesh != null:
				boxes += 1
		if boxes >= EXPECT_BOXES:
			print("[visual] OK: %d boxes (MeshInstance3D) spawned" % boxes)
			_teardown()
			quit(0)
		else:
			printerr("[visual] FAIL: tracked=%d but only %d mesh boxes" % [viz.tracked_count(), boxes])
			_teardown()
			quit(1)
		return true
	if elapsed > TIMEOUT_S:
		printerr("[visual] TIMEOUT: tracked=%d" % viz.tracked_count())
		_teardown()
		quit(1)
		return true
	return false

func _teardown() -> void:
	done = true
	if receiver:
		receiver.disconnect_from_server()
	if server:
		server.stop()

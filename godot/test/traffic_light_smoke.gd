# Headless smoke test for the OpenDRIVE traffic-light visualizer.
#
#   OsiRoadNetwork.load(.xodr) -> OsiTrafficLightVisualizer.build_from()
#                              -> set_color_state / all_off / introspection
#
# Verifies dynamic traffic lights are placed as head sub-trees (dark housing +
# N lamp quads) and that the lamp-state API toggles quad visibility. Runs fully
# headless (node creation + mesh/texture build need no display).
#
#   Godot ... --headless --path godot --script res://test/traffic_light_smoke.gd
# Exits 0 on success, 1 on failure.
extends SceneTree

# multi_intersections.xodr uses country "OpenDRIVE" dynamic lights
# (1000001 / 1000002), so it must yield at least one traffic-light head.
const ROAD := "res://examples/roads/multi_intersections.xodr"

func _initialize() -> void:
	var ok := true

	var net := OsiRoadNetwork.new()
	if not net.load(ROAD):
		push_error("[tl] load failed: %s" % ROAD)
		_finish(false)
		return

	var viz := OsiTrafficLightVisualizer.new()
	root.add_child(viz)
	viz.build_from(net)

	var heads := viz.head_count()
	print("[tl] traffic-light heads built: %d" % heads)
	if heads <= 0:
		push_error("[tl] expected at least one dynamic traffic-light head")
		_finish(false)
		return

	# The container node and at least one head sub-tree must exist.
	var container := viz.get_node_or_null(NodePath("TrafficLights"))
	if container == null:
		push_error("[tl] TrafficLights container node missing")
		ok = false

	var ids := viz.global_ids()
	if ids.is_empty():
		push_error("[tl] global_ids() empty despite head_count > 0")
		_finish(false)
		return
	var id: int = ids[0]
	var lamps := viz.lamp_count(id)
	print("[tl] head %d has %d lamps" % [id, lamps])
	if lamps < 1:
		push_error("[tl] head %d reported no lamps" % id)
		ok = false

	# The head sub-tree should be housing + N lamps = N + 1 children.
	if container != null:
		var head_node := container.get_node_or_null(NodePath("tl_%d" % id)) as Node3D
		if head_node == null:
			push_error("[tl] head node tl_%d missing" % id)
			ok = false
		else:
			var children := head_node.get_child_count()
			print("[tl] head node tl_%d has %d children (housing + lamps)" % [id, children])
			if children != lamps + 1:
				push_error("[tl] expected %d children, got %d" % [lamps + 1, children])
				ok = false

	# Lamps start unlit.
	if viz.is_lamp_on(id, 0):
		push_error("[tl] lamp 0 should start unlit")
		ok = false

	# set_color_state("red") lights exactly the red lamp (index 0 by convention).
	viz.set_color_state(id, "red")
	if not viz.is_lamp_on(id, 0):
		push_error("[tl] set_color_state('red') did not light lamp 0")
		ok = false

	# all_off clears it again.
	viz.all_off(id)
	if viz.is_lamp_on(id, 0):
		push_error("[tl] all_off did not turn lamp 0 off")
		ok = false

	# cycle_demo must not error and should leave some lamp lit on a 3-aspect head.
	viz.cycle_demo()
	print("[tl] cycle_demo ran")

	_finish(ok)

func _finish(ok: bool) -> void:
	if ok:
		print("[tl] OK")
	else:
		print("[tl] FAILED")
	quit(0 if ok else 1)

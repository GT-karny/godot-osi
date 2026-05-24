# Headless smoke test for the OSI-driven traffic-light pipeline.
#
#   OsiMockServer -> OsiReceiver -> OsiConverter --ground_truth_converted-->
#       bridge: GroundTruth.traffic_light[] -> OsiTrafficLightVisualizer
#
# Verifies that traffic-light states received over the (mock) OSI stream light
# up the 3D heads built from fabriksgatan_traffic_lights.xodr. The mock emits
# alternating red/green phases; we poll the converter until at least one head
# has a lit lamp, then confirm the phase flips over time.
#
#   Godot ... --headless --path godot --script res://test/traffic_osi_smoke.gd
# Exits 0 on success, 1 on failure.
extends SceneTree

const ROAD := "res://examples/roads/fabriksgatan_traffic_lights.xodr"
const OSI_COLOR := {2: "red", 3: "yellow", 4: "green"}
const OSI_MODE_OFF := 2

var tl: OsiTrafficLightVisualizer
var _ids: PackedInt64Array
var _frames: int = 0

func _initialize() -> void:
	var net := OsiRoadNetwork.new()
	if not net.load(ROAD):
		push_error("[tl-osi] load failed: %s" % ROAD)
		_finish(false)
		return

	tl = OsiTrafficLightVisualizer.new()
	root.add_child(tl)
	tl.build_from(net)
	_ids = tl.global_ids()
	print("[tl-osi] heads=%d ids=%s" % [tl.head_count(), str(_ids)])
	if tl.head_count() <= 0:
		push_error("[tl-osi] no traffic-light heads built")
		_finish(false)
		return

	var mock := OsiMockServer.new()
	mock.address = "127.0.0.1"
	mock.port = 50071           # avoid clashing with a real server on 50051
	mock.period_ms = 33
	root.add_child(mock)
	mock.start()

	var receiver := OsiReceiver.new()
	receiver.address = "127.0.0.1"
	receiver.port = 50071
	root.add_child(receiver)

	var converter := OsiConverter.new()
	root.add_child(converter)
	converter.connect_source(receiver)
	converter.ground_truth_converted.connect(_on_gt)
	receiver.connect_to_server()

	# Poll until a head lights up (frames arrive on a background thread), and
	# capture the colour sequence on the first head to confirm it changes.
	var seen := {}
	var lit_seen := false
	for _i in range(120):                # up to ~6 s at 50 ms steps
		OS.delay_msec(50)
		converter.poll()
		if _ids.size() > 0:
			for c in ["red", "yellow", "green"]:
				if _head_color(_ids[0]) == c:
					seen[c] = true
		if not lit_seen and _any_head_lit():
			lit_seen = true
		# Stop early once we have a lit head and have seen >= 2 distinct colours.
		if lit_seen and seen.size() >= 2:
			break

	var ok := true
	if _frames == 0:
		push_error("[tl-osi] no GroundTruth frames received from mock")
		ok = false
	if not lit_seen:
		push_error("[tl-osi] no head ever lit from the OSI stream")
		ok = false
	if seen.size() < 2:
		push_error("[tl-osi] head colour did not change (saw %s)" % str(seen.keys()))
		ok = false
	print("[tl-osi] frames=%d colours-seen=%s" % [_frames, str(seen.keys())])

	receiver.disconnect_from_server()
	mock.stop()
	_finish(ok)

# Bridge: apply received traffic-light states to the heads (index-mapped, since
# the mock's lights are not tied to this map's signal positions).
func _on_gt(snapshot) -> void:
	_frames += 1
	for id in _ids:
		tl.all_off(id)
	var lights = snapshot.traffic_light
	for i in range(lights.size()):
		var cls = lights[i].classification
		if cls == null or cls.mode == OSI_MODE_OFF:
			continue
		var color: String = OSI_COLOR.get(cls.color, "")
		if color != "" and i < _ids.size():
			tl.set_color_state(_ids[i], color)

func _any_head_lit() -> bool:
	for id in _ids:
		for li in range(tl.lamp_count(id)):
			if tl.is_lamp_on(id, li):
				return true
	return false

# Name of the lit colour on a head (assumes the standard top-down lamp order).
func _head_color(id: int) -> String:
	var order := ["red", "yellow", "green"]
	var n := tl.lamp_count(id)
	for li in range(n):
		if tl.is_lamp_on(id, li):
			# 3-aspect: index->order; 2-aspect: red(0)/green(last).
			if n == 3:
				return order[li]
			return "red" if li == 0 else "green"
	return ""

func _finish(ok: bool) -> void:
	if ok:
		print("[tl-osi] OK")
	else:
		print("[tl-osi] FAILED")
	quit(0 if ok else 1)

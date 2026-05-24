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
const TL_ID_PREFIX := "traffic_light_id:"

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

	# The 3-lamp head is the vehicle signal (id 1) — the only one that ever gets
	# yellow. Seeing yellow there proves colours are routed by signal id, not by
	# position or order.
	var vehicle_id := _head_with_lamps(3)
	var ped_id := _head_with_lamps(2)
	if vehicle_id < 0 or ped_id < 0:
		push_error("[tl-osi] expected a 3-lamp and a 2-lamp head")
		_finish(false)
		return

	# Poll while frames arrive (background thread) and record colours seen.
	var veh_seen := {}
	var ped_lit := false
	for _i in range(160):                # up to ~8 s at 50 ms steps (full cycle)
		OS.delay_msec(50)
		converter.poll()
		var vc := _head_color(vehicle_id)
		if vc != "":
			veh_seen[vc] = true
		if tl.is_lamp_on(ped_id, 0) or (tl.lamp_count(ped_id) > 1 and tl.is_lamp_on(ped_id, 1)):
			ped_lit = true
		if veh_seen.has("yellow") and veh_seen.has("green") and ped_lit:
			break

	var ok := true
	if _frames == 0:
		push_error("[tl-osi] no GroundTruth frames received from mock")
		ok = false
	if not veh_seen.has("yellow"):
		push_error("[tl-osi] vehicle head never showed yellow (id routing broken?)")
		ok = false
	if veh_seen.size() < 2:
		push_error("[tl-osi] vehicle head colour did not change (saw %s)" % str(veh_seen.keys()))
		ok = false
	if not ped_lit:
		push_error("[tl-osi] pedestrian head never lit")
		ok = false
	print("[tl-osi] frames=%d vehicle-colours=%s ped_lit=%s" % [_frames, str(veh_seen.keys()), str(ped_lit)])

	receiver.disconnect_from_server()
	mock.stop()
	_finish(ok)

# Bridge: route each lit OSI bulb to its head by the OpenDRIVE signal id carried
# in source_reference ("traffic_light_id:<N>") — no position/index guessing.
func _on_gt(snapshot) -> void:
	_frames += 1
	for id in _ids:
		tl.all_off(id)
	for light in snapshot.traffic_light:
		var cls = light.classification
		if cls == null or cls.mode == OSI_MODE_OFF:
			continue
		var color: String = OSI_COLOR.get(cls.color, "")
		if color == "":
			continue
		var sig := _signal_id_from_ref(light)
		if sig >= 0:
			tl.set_color_state_by_signal_id(sig, color)

func _signal_id_from_ref(light) -> int:
	for ref in light.source_reference:
		for ident in ref.identifier:
			if (ident as String).begins_with(TL_ID_PREFIX):
				return int((ident as String).substr(TL_ID_PREFIX.length()))
	return -1

# First built head with exactly `n` lamps, or -1.
func _head_with_lamps(n: int) -> int:
	for id in _ids:
		if tl.lamp_count(id) == n:
			return id
	return -1

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

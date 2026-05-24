# End-to-end integration smoke test for the full receiver -> converter wiring.
#
#   OsiMockServer  -> OsiReceiver (gRPC client) -> OsiFrameBus -> OsiConverter
#                                                                    |
#                                  ground_truth_converted / host_vehicle_data_converted
#
# Verifies that OsiConverter.connect_source(receiver) shares the receiver's bus
# so the converter drains real streamed frames and emits typed snapshots.
#
# Run with (see run_integration.ps1, which stages the dll + extension_list):
#   Godot ... --headless --path godot --script res://test/integration_smoke.gd
# Exits 0 once both converted signals fire with sane snapshots, 1 on timeout.
extends SceneTree

const PORT := 50081
const TARGET := 3            # converted frames required per stream
const TIMEOUT_S := 15.0

var server
var receiver
var converter
var ego
var gt_converted := 0
var hvd_converted := 0
var last_gt = null
var elapsed := 0.0
var done := false

func _initialize() -> void:
	print("[integration] starting mock server on 127.0.0.1:%d" % PORT)
	server = OsiMockServer.new()
	server.address = "127.0.0.1"
	server.port = PORT
	server.period_ms = 20
	root.add_child(server)
	server.start()

	receiver = OsiReceiver.new()
	receiver.address = "127.0.0.1"
	receiver.port = PORT
	receiver.reconnect = true
	receiver.reconnect_delay_ms = 200
	root.add_child(receiver)

	converter = OsiConverter.new()
	converter.auto_poll = true
	root.add_child(converter)
	converter.ground_truth_converted.connect(_on_gt)
	converter.host_vehicle_data_converted.connect(_on_hvd)

	# HMI convenience helper: caches host/ground-truth and exposes ego_state().
	ego = OsiHostVehicleState.new()
	root.add_child(ego)
	ego.bind_converter(converter)

	# The wiring under test: hand the converter the receiver's shared bus.
	converter.connect_source(receiver)

	# Order-independence: connect_source was called before connect_to_server.
	receiver.connect_to_server()
	print("[integration] receiver connecting, converter wired & auto-polling...")

func _on_gt(snapshot) -> void:
	gt_converted += 1
	last_gt = snapshot

func _on_hvd(_snapshot) -> void:
	hvd_converted += 1

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta
	if gt_converted >= TARGET and hvd_converted >= TARGET:
		if _validate():
			print("[integration] OK: gt=%d hvd=%d converted frames" % [gt_converted, hvd_converted])
			_teardown()
			quit(0)
		else:
			_teardown()
			quit(1)
		return true
	if elapsed > TIMEOUT_S:
		printerr("[integration] TIMEOUT: gt=%d hvd=%d" % [gt_converted, hvd_converted])
		_teardown()
		quit(1)
		return true
	return false

func _validate() -> bool:
	# The synthetic mock stream carries several MovingObjects circling the origin.
	if last_gt == null:
		printerr("[integration] FAIL: no GroundTruth snapshot captured")
		return false
	if last_gt.moving_object.size() < 1:
		printerr("[integration] FAIL: snapshot has no moving objects")
		return false
	var mo = last_gt.moving_object[0]
	if mo == null or mo.base == null or mo.base.position == null:
		printerr("[integration] FAIL: moving object base/position missing")
		return false
	var pos = mo.base.position
	if not (is_finite(pos.x) and is_finite(pos.y) and is_finite(pos.z)):
		printerr("[integration] FAIL: non-finite converted position")
		return false
	print("[integration] ok  obj0 converted pos=(%f,%f,%f)" % [pos.x, pos.y, pos.z])

	# Ego-state convenience helper (meter-cluster HMI input).
	if not ego.is_ready():
		printerr("[integration] FAIL: OsiHostVehicleState not ready")
		return false
	var st: Dictionary = ego.ego_state()
	print("[integration] ego_state=%s" % str(st))
	if not st.get("valid", false):
		printerr("[integration] FAIL: ego_state invalid")
		return false
	# The synthetic host stream cruises at ~30-65 km/h in a forward gear.
	var kph: float = st["speed_kph"]
	if not (is_finite(kph) and kph > 5.0 and kph < 100.0):
		printerr("[integration] FAIL: implausible ego speed %f km/h" % kph)
		return false
	if int(st["gear"]) <= 0:
		printerr("[integration] FAIL: expected a forward gear, got %d" % int(st["gear"]))
		return false
	print("[integration] ok  ego speed=%.1f km/h gear=%d rpm=%.0f" % [kph, int(st["gear"]), st["rpm"]])
	return true

func _teardown() -> void:
	done = true
	if receiver:
		receiver.disconnect_from_server()
	if server:
		server.stop()

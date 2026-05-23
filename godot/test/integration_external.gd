# End-to-end pipeline against an EXTERNAL gRPC OSI server (e.g. GT_Sim on
# 127.0.0.1:50051), no bundled mock. Retries until the server is up, then
# verifies the full receiver -> shared bus -> converter path produces typed
# snapshots from real frames.
#
#   OsiReceiver -> connect_source -> OsiConverter
#                                        |
#                  ground_truth_converted / host_vehicle_data_converted
#
# Run with:
#   Godot ... --headless --path godot --script res://test/integration_external.gd
# Exits 0 once converted frames are validated, 1 on timeout.
extends SceneTree

const HOST := "127.0.0.1"
const PORT := 50051
const RETRY_MS := 1000
const TARGET := 5            # converted GroundTruth frames before declaring OK
const MAX_WAIT_S := 40.0     # give up if no converted frame arrives by then
const MEASURE_S := 5.0       # keep converting this long after the first frame

var receiver
var converter
var gt_converted := 0
var hvd_converted := 0
var last_gt = null
var elapsed := 0.0
var last_report := 0.0
var first_frame_t := -1.0
var done := false

func _initialize() -> void:
	receiver = OsiReceiver.new()
	receiver.address = HOST
	receiver.port = PORT
	receiver.reconnect = true
	receiver.reconnect_delay_ms = RETRY_MS
	root.add_child(receiver)

	converter = OsiConverter.new()
	converter.auto_poll = true
	root.add_child(converter)
	converter.ground_truth_converted.connect(_on_gt)
	converter.host_vehicle_data_converted.connect(_on_hvd)
	converter.connect_source(receiver)   # share the receiver's bus

	receiver.connection_state_changed.connect(_on_state)
	receiver.connect_to_server()
	print("[integration_ext] connecting to %s:%d (retry %dms), converter wired..."
		% [HOST, PORT, RETRY_MS])

func _on_state(state: int) -> void:
	var names := ["DISCONNECTED", "CONNECTING", "CONNECTED", "RECONNECTING", "ERROR"]
	var label: String = names[state] if state >= 0 and state < names.size() else str(state)
	print("[integration_ext] state -> %s" % label)

func _on_gt(snapshot) -> void:
	gt_converted += 1
	last_gt = snapshot
	if first_frame_t < 0.0:
		first_frame_t = elapsed
		print("[integration_ext] first converted frame at %.1fs" % elapsed)

func _on_hvd(_snapshot) -> void:
	hvd_converted += 1

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta
	if elapsed - last_report >= 1.0:
		last_report = elapsed
		print("[integration_ext] gt=%d hvd=%d (%.0fs)" % [gt_converted, hvd_converted, elapsed])

	if first_frame_t >= 0.0 and gt_converted >= TARGET and elapsed - first_frame_t >= MEASURE_S:
		if _validate():
			print("[integration_ext] OK: converted gt=%d hvd=%d from real server"
				% [gt_converted, hvd_converted])
			_teardown()
			quit(0)
		else:
			_teardown()
			quit(1)
		return true

	if first_frame_t < 0.0 and elapsed > MAX_WAIT_S:
		printerr("[integration_ext] TIMEOUT: no converted frames (gt=%d hvd=%d)"
			% [gt_converted, hvd_converted])
		_teardown()
		quit(1)
		return true
	return false

func _validate() -> bool:
	if last_gt == null:
		printerr("[integration_ext] FAIL: no GroundTruth snapshot")
		return false
	var n: int = last_gt.moving_object.size()
	print("[integration_ext] last snapshot moving_object count = %d" % n)
	if n < 1:
		printerr("[integration_ext] FAIL: snapshot has no moving objects")
		return false
	var mo = last_gt.moving_object[0]
	if mo == null or mo.base == null or mo.base.position == null:
		printerr("[integration_ext] FAIL: moving object base/position missing")
		return false
	var p = mo.base.position
	if not (is_finite(p.x) and is_finite(p.y) and is_finite(p.z)):
		printerr("[integration_ext] FAIL: non-finite converted position")
		return false
	print("[integration_ext] ok  obj0 converted pos=(%f,%f,%f)" % [p.x, p.y, p.z])
	return true

func _teardown() -> void:
	done = true
	if receiver:
		receiver.disconnect_from_server()

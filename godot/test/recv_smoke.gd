# Headless smoke test: OsiMockServer -> OsiReceiver -> signals/bus.
# Run with:
#   Godot ... --headless --path godot --script res://test/recv_smoke.gd
# Exits 0 on success (frames received), 1 on timeout/failure.
extends SceneTree

const PORT := 50071
const TARGET_GT := 5
const TIMEOUT_S := 15.0

var server
var receiver
var gt_frames := 0
var hvd_frames := 0
var elapsed := 0.0
var done := false

func _initialize() -> void:
	print("[recv_smoke] starting mock server on 127.0.0.1:%d" % PORT)
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
	receiver.connection_state_changed.connect(_on_state)
	receiver.ground_truth_received.connect(_on_gt)
	receiver.host_vehicle_data_received.connect(_on_hvd)
	receiver.stream_error.connect(_on_error)
	receiver.connect_to_server()
	print("[recv_smoke] receiver connecting...")

func _on_state(state: int) -> void:
	print("[recv_smoke] connection_state_changed -> %d" % state)

func _on_gt() -> void:
	gt_frames += 1

func _on_hvd() -> void:
	hvd_frames += 1

func _on_error(msg: String) -> void:
	print("[recv_smoke] stream_error: %s" % msg)

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta
	if gt_frames >= TARGET_GT and hvd_frames >= TARGET_GT:
		print("[recv_smoke] OK: gt=%d hvd=%d frames received" % [gt_frames, hvd_frames])
		_teardown()
		quit(0)
		return true
	if elapsed > TIMEOUT_S:
		print("[recv_smoke] TIMEOUT: gt=%d hvd=%d" % [gt_frames, hvd_frames])
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

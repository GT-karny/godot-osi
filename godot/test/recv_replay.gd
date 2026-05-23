# Record->replay round trip: feed recorded .osi traces through OsiMockServer
# and receive them back with an OsiReceiver. No external server needed.
#
# Run with:
#   Godot ... --headless --path godot --script res://test/recv_replay.gd
extends SceneTree

const PORT := 50073
const TARGET := 50
const TIMEOUT_S := 20.0
const GT_TRACE := "E:/Repository/GT-karny/Godot-OSI-plugin/worktrees/receiver/traces/gt.osi"
const HVD_TRACE := "E:/Repository/GT-karny/Godot-OSI-plugin/worktrees/receiver/traces/hvd.osi"

var server
var receiver
var gt := 0
var hvd := 0
var elapsed := 0.0
var done := false

func _initialize() -> void:
	server = OsiMockServer.new()
	server.address = "127.0.0.1"
	server.port = PORT
	server.period_ms = 5
	server.set_traces(GT_TRACE, HVD_TRACE)   # prints how many frames it loaded
	root.add_child(server)
	server.start()

	receiver = OsiReceiver.new()
	receiver.address = "127.0.0.1"
	receiver.port = PORT
	receiver.reconnect_delay_ms = 200
	root.add_child(receiver)
	receiver.ground_truth_received.connect(_on_gt)
	receiver.host_vehicle_data_received.connect(_on_hvd)
	receiver.connect_to_server()
	print("[recv_replay] replaying traces through mock server on port %d..." % PORT)

func _on_gt() -> void:
	gt += 1

func _on_hvd() -> void:
	hvd += 1

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta
	if gt >= TARGET and hvd >= TARGET:
		print("[recv_replay] OK: replayed-and-received gt=%d hvd=%d" % [gt, hvd])
		_teardown()
		quit(0)
		return true
	if elapsed > TIMEOUT_S:
		print("[recv_replay] TIMEOUT: gt=%d hvd=%d" % [gt, hvd])
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

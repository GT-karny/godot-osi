# Minimal end-to-end OSI pipeline sample:
#
#   OsiReceiver (gRPC client) --shared OsiFrameBus--> OsiConverter
#                                                          |
#                                          ground_truth_converted
#                                                          v
#                                          OsiMovingObjectSpawner (Node3D)
#
# Attach this script to a Node3D and run the scene. By default it connects to a
# gRPC OSI server at 127.0.0.1:50051 (e.g. GT_Sim, or the bundled OsiMockServer).
# Set USE_MOCK = true to spin up the bundled mock server instead of needing one.
extends Node3D

const HOST := "127.0.0.1"
const PORT := 50051
const USE_MOCK := false   # true: start a bundled OsiMockServer on HOST:PORT

var receiver: OsiReceiver
var converter: OsiConverter
var spawner: OsiMovingObjectSpawner
var mock              # OsiMockServer when USE_MOCK

func _ready() -> void:
	if USE_MOCK:
		mock = OsiMockServer.new()
		mock.address = HOST
		mock.port = PORT
		mock.period_ms = 33
		add_child(mock)
		mock.start()

	# 1. Receiver: streams raw OSI frames into its OsiFrameBus.
	receiver = OsiReceiver.new()
	receiver.address = HOST
	receiver.port = PORT
	add_child(receiver)

	# 2. Converter: shares the receiver's bus, emits typed Godot snapshots.
	converter = OsiConverter.new()
	add_child(converter)
	converter.connect_source(receiver)   # <-- the integration wiring

	# 3. Spawner: turns each converted GroundTruth into a tracked child Node3D.
	spawner = OsiMovingObjectSpawner.new()
	add_child(spawner)
	spawner.bind_converter(converter)

	receiver.connection_state_changed.connect(_on_state)
	receiver.connect_to_server()
	print("[osi_pipeline] connecting to %s:%d (mock=%s)" % [HOST, PORT, USE_MOCK])

func _on_state(state: int) -> void:
	print("[osi_pipeline] connection state -> %d (tracked objects: %d)"
		% [state, spawner.tracked_count()])

func _exit_tree() -> void:
	if receiver:
		receiver.disconnect_from_server()
	if mock:
		mock.stop()

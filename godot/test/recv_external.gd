# Connect to an EXTERNAL gRPC server (no bundled mock), waiting for it to come
# up by retrying every 1s, and record received frames to .osi traces (§6).
#
# Run with:
#   Godot ... --headless --path godot --script res://test/recv_external.gd
# Exits 0 once frames are received, 1 if none arrive before MAX_WAIT_S.
extends SceneTree

const HOST := "127.0.0.1"
const PORT := 50051
const USE_TLS := false
const RETRY_MS := 1000          # 1s retry while waiting for the server
const MEASURE_S := 6.0          # once frames start, keep receiving this long
const MAX_WAIT_S := 30.0        # give up if no frame arrives within this

# OS-absolute trace paths (Rust writes via std::fs, not res://).
const GT_TRACE := "E:/Repository/GT-karny/Godot-OSI-plugin/worktrees/receiver/traces/gt.osi"
const HVD_TRACE := "E:/Repository/GT-karny/Godot-OSI-plugin/worktrees/receiver/traces/hvd.osi"

var receiver
var gt_frames := 0
var hvd_frames := 0
var elapsed := 0.0
var last_report := 0.0
var first_frame_t := -1.0
var done := false

func _note_frame() -> void:
	if first_frame_t < 0.0:
		first_frame_t = elapsed
		print("[recv_external] first frame at %.1fs, measuring for %.0fs..." % [elapsed, MEASURE_S])

func _initialize() -> void:
	receiver = OsiReceiver.new()
	receiver.address = HOST
	receiver.port = PORT
	receiver.use_tls = USE_TLS
	receiver.reconnect = true
	receiver.reconnect_delay_ms = RETRY_MS
	root.add_child(receiver)
	receiver.connection_state_changed.connect(_on_state)
	receiver.ground_truth_received.connect(_on_gt)
	receiver.host_vehicle_data_received.connect(_on_hvd)
	receiver.stream_error.connect(_on_error)

	# Start recording BEFORE connecting so the first frames are captured.
	receiver.start_recording(GT_TRACE, HVD_TRACE)
	receiver.connect_to_server()
	print("[recv_external] connecting to %s:%d (retry every %dms, waiting for server)..." % [HOST, PORT, RETRY_MS])

func _on_state(state: int) -> void:
	var names := ["DISCONNECTED", "CONNECTING", "CONNECTED", "RECONNECTING", "ERROR"]
	var label: String = names[state] if state >= 0 and state < names.size() else str(state)
	print("[recv_external] state -> %s" % label)

func _on_gt() -> void:
	gt_frames += 1
	_note_frame()

func _on_hvd() -> void:
	hvd_frames += 1
	_note_frame()

func _on_error(msg: String) -> void:
	print("[recv_external] stream_error: %s" % msg)

func _process(delta: float) -> bool:
	if done:
		return true
	elapsed += delta

	if elapsed - last_report >= 1.0:
		last_report = elapsed
		print("[recv_external] gt=%d hvd=%d (%.0fs)" % [gt_frames, hvd_frames, elapsed])

	# Done once we've measured for MEASURE_S after the first frame.
	if first_frame_t >= 0.0 and elapsed - first_frame_t >= MEASURE_S:
		var span: float = elapsed - first_frame_t
		var total: int = gt_frames + hvd_frames
		print("[recv_external] OK: gt=%d hvd=%d total=%d over %.1fs" % [gt_frames, hvd_frames, total, span])
		print("[recv_external] rate: gt=%.1f/s hvd=%.1f/s total=%.1f/s" % [gt_frames / span, hvd_frames / span, total / span])
		_teardown()
		quit(0)
		return true

	# No frame at all within MAX_WAIT_S -> give up.
	if first_frame_t < 0.0 and elapsed > MAX_WAIT_S:
		print("[recv_external] TIMEOUT: no frames (gt=%d hvd=%d)" % [gt_frames, hvd_frames])
		_teardown()
		quit(1)
		return true
	return false

func _teardown() -> void:
	done = true
	if receiver:
		receiver.stop_recording()       # flush .osi traces
		receiver.disconnect_from_server()
	print("[recv_external] traces: %s , %s" % [GT_TRACE, HVD_TRACE])

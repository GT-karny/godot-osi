# Headless smoke test for the runtime settings UI's data + persistence layer.
#
# Covers the new, non-native logic so it can run without a display:
#   1. OsiPresetStore roundtrip: connection and road presets are independent
#      families (same name in both, no collision), survive save/reload, and
#      last-selected meta persists. Deleting a conn preset leaves the road one
#      and clears last_conn.
#   2. OsiSettingsPanel state mapping covers all five OsiReceiver.STATE_* ints.
#   3. (native) OsiMockServer -> OsiReceiver connection-state transition reaches
#      STATE_CONNECTED, validating the int the panel renders.
#
#   Godot ... --headless --path godot --script res://test/ui_preset_smoke.gd
# Exits 0 on success, 1 on failure.
extends SceneTree

const TMP_CFG := "user://osi_settings_test.cfg"
const PORT := 50083
const STATE_CONNECTED := 2
const TIMEOUT_S := 15.0

var receiver
var server
var states_seen := {}
var elapsed := 0.0
var done := false
var _native_phase := false

func _initialize() -> void:
	if not _test_preset_store():
		_finish(false)
		return
	if not _test_state_mapping():
		_finish(false)
		return

	if not ClassDB.class_exists("OsiReceiver"):
		print("[ui] native extension absent; skipping connect-state phase")
		_finish(true)
		return

	_native_phase = true
	_start_connect_phase()

# --- 1. Preset store roundtrip + family independence ---------------------

func _test_preset_store() -> bool:
	_remove_tmp()
	var conn := {
		"use_mock": false, "address": "10.0.0.5", "port": 50061,
		"use_tls": true, "reconnect": false, "reconnect_delay_ms": 2500,
		"mock_period_ms": 33,
	}
	var road := {"road_path": "C:/maps/e6mini.xodr"}

	var store := OsiPresetStore.new(TMP_CFG)
	store.load()
	store.save_conn("A", conn)        # same name "A" in both families...
	store.save_road("A", road)        # ...must not collide
	store.set_last_conn("A")
	store.set_last_road("A")

	# Reload from disk into a fresh store.
	var s2 := OsiPresetStore.new(TMP_CFG)
	s2.load()

	if not _has(s2.conn_names(), "A") or not _has(s2.road_names(), "A"):
		printerr("[ui] FAIL: preset 'A' missing after reload")
		return false

	var got_conn := s2.get_conn("A")
	for k in conn:
		if got_conn.get(k) != conn[k]:
			printerr("[ui] FAIL: conn[%s]=%s expected %s" % [k, str(got_conn.get(k)), str(conn[k])])
			return false
	if s2.get_road("A").get("road_path") != road["road_path"]:
		printerr("[ui] FAIL: road path roundtrip mismatch")
		return false
	# Independence: the conn dict must not have leaked the road key, and v.v.
	if got_conn.has("road_path") or s2.get_road("A").has("address"):
		printerr("[ui] FAIL: conn/road preset families leaked keys")
		return false
	if s2.get_last_conn() != "A" or s2.get_last_road() != "A":
		printerr("[ui] FAIL: last-selected meta not persisted")
		return false

	# Delete conn 'A': road 'A' survives, last_conn clears, last_road intact.
	s2.delete_conn("A")
	var s3 := OsiPresetStore.new(TMP_CFG)
	s3.load()
	if _has(s3.conn_names(), "A"):
		printerr("[ui] FAIL: conn 'A' still present after delete")
		return false
	if not _has(s3.road_names(), "A"):
		printerr("[ui] FAIL: road 'A' wrongly removed by conn delete")
		return false
	if s3.get_last_conn() != "" or s3.get_last_road() != "A":
		printerr("[ui] FAIL: meta not updated correctly after delete")
		return false

	_remove_tmp()
	print("[ui] OK preset store: independent families, roundtrip, meta, delete")
	return true

# --- 2. State int -> label/color mapping coverage ------------------------

func _test_state_mapping() -> bool:
	var text: Dictionary = OsiSettingsPanel.STATE_TEXT
	var color: Dictionary = OsiSettingsPanel.STATE_COLOR
	for state in range(5):   # STATE_DISCONNECTED(0) .. STATE_ERROR(4)
		if not text.has(state) or not color.has(state):
			printerr("[ui] FAIL: STATE map missing entry for %d" % state)
			return false
	print("[ui] OK state mapping covers all 5 OsiReceiver.STATE_* values")
	return true

# --- 3. Native connect-state transition ----------------------------------

func _start_connect_phase() -> void:
	print("[ui] starting mock server on 127.0.0.1:%d" % PORT)
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
	receiver.connect_to_server()

func _on_state(state: int) -> void:
	states_seen[state] = true

func _process(delta: float) -> bool:
	if not _native_phase or done:
		return done
	elapsed += delta
	if states_seen.has(STATE_CONNECTED):
		print("[ui] OK connection reached STATE_CONNECTED (states=%s)" % str(states_seen.keys()))
		_finish(true)
		return true
	if elapsed > TIMEOUT_S:
		printerr("[ui] TIMEOUT: never reached STATE_CONNECTED (states=%s)" % str(states_seen.keys()))
		_finish(false)
		return true
	return false

# --- helpers -------------------------------------------------------------

func _has(arr: PackedStringArray, name: String) -> bool:
	return arr.has(name)

func _remove_tmp() -> void:
	if FileAccess.file_exists(TMP_CFG):
		DirAccess.remove_absolute(ProjectSettings.globalize_path(TMP_CFG))

func _finish(ok: bool) -> void:
	done = true
	if receiver:
		receiver.disconnect_from_server()
	if server:
		server.stop()
	print("[ui] OK" if ok else "[ui] FAILED")
	quit(0 if ok else 1)

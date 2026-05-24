## Reusable runtime settings overlay for the godot_osi plugin.
##
## Drop this panel under any CanvasLayer in an exported game. It edits OSI
## connection settings and an OpenDRIVE map path, persists them as independent
## connection / road presets (see OsiPresetStore), and emits intent signals.
##
## The panel is DECOUPLED from the pipeline: it never creates OsiReceiver /
## OsiConverter / OsiRoadNetwork itself. An integrator connects to the signals
## below, builds the pipeline, and pushes status back via the public setters.
##
##   panel.apply_connection.connect(_on_apply_connection)   # build/reconnect
##   panel.disconnect_requested.connect(_on_disconnect)
##   panel.load_road.connect(_on_load_road)
##   receiver.connection_state_changed.connect(panel.set_connection_state)
class_name OsiSettingsPanel
extends PanelContainer

## Emitted when Connect is pressed. `config` is a fresh duplicate the integrator
## can keep; mirror its fields onto OsiReceiver / OsiMockServer.
signal apply_connection(config: OsiSettingsConfig)
## Emitted when Disconnect is pressed.
signal disconnect_requested()
## Emitted when Load is pressed; `path` is an OS / user:// path to a .xodr.
signal load_road(path: String)

# Source OptionButton item order.
const SRC_MOCK := 0
const SRC_EXTERNAL := 1

# OsiReceiver.STATE_* -> label text / color. Covers all five native constants.
const STATE_TEXT := {
	0: "Disconnected",   # STATE_DISCONNECTED
	1: "Connecting…",    # STATE_CONNECTING
	2: "Connected",      # STATE_CONNECTED
	3: "Reconnecting…",  # STATE_RECONNECTING
	4: "Error",          # STATE_ERROR
}
const STATE_COLOR := {
	0: Color(0.7, 0.7, 0.7),
	1: Color(0.95, 0.85, 0.2),
	2: Color(0.3, 0.85, 0.35),
	3: Color(0.95, 0.6, 0.2),
	4: Color(0.9, 0.3, 0.3),
}

# Connection preset widgets.
@onready var _conn_preset: OptionButton = %ConnPreset
@onready var _conn_name: LineEdit = %ConnName
# Connection setting widgets.
@onready var _source: OptionButton = %Source
@onready var _address: LineEdit = %Address
@onready var _port: SpinBox = %Port
@onready var _use_tls: CheckBox = %UseTls
@onready var _reconnect: CheckBox = %Reconnect
@onready var _reconnect_delay: SpinBox = %ReconnectDelay
@onready var _mock_period_row: Control = %MockPeriodRow
@onready var _mock_period: SpinBox = %MockPeriod
@onready var _status: Label = %Status
# Road preset + setting widgets.
@onready var _road_preset: OptionButton = %RoadPreset
@onready var _road_name: LineEdit = %RoadName
@onready var _road_path: LineEdit = %RoadPath
@onready var _road_result: Label = %RoadResult
@onready var _config_path: Label = %ConfigPath
@onready var _file_dialog: FileDialog = %FileDialog

var _store: OsiPresetStore
var _config := OsiSettingsConfig.new()

func _ready() -> void:
	_store = OsiPresetStore.new()
	_store.load()

	# Connection settings.
	_source.item_selected.connect(_on_source_selected)
	%Connect.pressed.connect(_on_connect_pressed)
	%Disconnect.pressed.connect(_on_disconnect_pressed)
	# Connection presets.
	_conn_preset.item_selected.connect(_on_conn_preset_selected)
	%ConnSaveNew.pressed.connect(_on_conn_save_new)
	%ConnSave.pressed.connect(_on_conn_save)
	%ConnDelete.pressed.connect(_on_conn_delete)
	# Road settings.
	%Browse.pressed.connect(_on_browse_pressed)
	%LoadRoad.pressed.connect(_on_load_road_pressed)
	_file_dialog.file_selected.connect(_on_file_selected)
	# Road presets.
	_road_preset.item_selected.connect(_on_road_preset_selected)
	%RoadSaveNew.pressed.connect(_on_road_save_new)
	%RoadSave.pressed.connect(_on_road_save)
	%RoadDelete.pressed.connect(_on_road_delete)

	_populate_widgets(_config)
	_refresh_conn_presets()
	_refresh_road_presets()
	_select_last_presets()

	var disk := ProjectSettings.globalize_path(_store.config_path())
	_config_path.text = "presets: %s" % disk
	_config_path.tooltip_text = disk

# --- Public API (integrator -> panel) ------------------------------------

## Render an OsiReceiver.STATE_* int onto the status label.
func set_connection_state(state: int) -> void:
	_status.text = STATE_TEXT.get(state, "Unknown (%d)" % state)
	_status.add_theme_color_override("font_color", STATE_COLOR.get(state, Color.WHITE))

## Show a stream error (OsiReceiver.stream_error) on the status label.
func set_status_error(message: String) -> void:
	_status.text = "Error: %s" % message
	_status.add_theme_color_override("font_color", STATE_COLOR[4])

## Report the outcome of an OsiRoadNetwork.load attempt.
func set_road_result(ok: bool, roads: int, signs: int, err: String) -> void:
	if ok:
		_road_result.text = "Loaded: %d roads, %d signs" % [roads, signs]
		_road_result.add_theme_color_override("font_color", STATE_COLOR[2])
	else:
		_road_result.text = "Load failed: %s" % err
		_road_result.add_theme_color_override("font_color", STATE_COLOR[4])

## Current widget state as a fresh config (for launch-time auto-apply).
func get_config() -> OsiSettingsConfig:
	_collect_config()
	return _config.duplicate_config()

## Push a config into the widgets.
func set_config(cfg: OsiSettingsConfig) -> void:
	_config = cfg.duplicate_config()
	_populate_widgets(_config)

# --- Widget <-> config sync ----------------------------------------------

func _collect_config() -> void:
	_config.use_mock = _source.selected == SRC_MOCK
	_config.address = _address.text
	_config.port = int(_port.value)
	_config.use_tls = _use_tls.button_pressed
	_config.reconnect = _reconnect.button_pressed
	_config.reconnect_delay_ms = int(_reconnect_delay.value)
	_config.mock_period_ms = int(_mock_period.value)
	_config.road_path = _road_path.text

func _populate_widgets(cfg: OsiSettingsConfig) -> void:
	_source.selected = SRC_MOCK if cfg.use_mock else SRC_EXTERNAL
	_address.text = cfg.address
	_port.value = cfg.port
	_use_tls.button_pressed = cfg.use_tls
	_reconnect.button_pressed = cfg.reconnect
	_reconnect_delay.value = cfg.reconnect_delay_ms
	_mock_period.value = cfg.mock_period_ms
	_road_path.text = cfg.road_path
	_update_mock_period_visibility()

func _update_mock_period_visibility() -> void:
	_mock_period_row.visible = _source.selected == SRC_MOCK

# --- Connection setting handlers -----------------------------------------

func _on_source_selected(_idx: int) -> void:
	_update_mock_period_visibility()

func _on_connect_pressed() -> void:
	_collect_config()
	apply_connection.emit(_config.duplicate_config())

func _on_disconnect_pressed() -> void:
	disconnect_requested.emit()

# --- Road setting handlers -----------------------------------------------

func _on_browse_pressed() -> void:
	_file_dialog.popup_centered_ratio(0.6)

func _on_file_selected(path: String) -> void:
	_road_path.text = path

func _on_load_road_pressed() -> void:
	_collect_config()
	load_road.emit(_config.road_path)

# --- Connection preset handlers ------------------------------------------

func _refresh_conn_presets(select_name: String = "") -> void:
	_fill_option(_conn_preset, _store.conn_names(), select_name)

func _on_conn_preset_selected(idx: int) -> void:
	var name := _conn_preset.get_item_text(idx)
	_config.apply_conn_dict(_store.get_conn(name))
	_populate_widgets(_config)
	_store.set_last_conn(name)

func _on_conn_save_new() -> void:
	var name := _conn_name.text.strip_edges()
	if name.is_empty():
		return
	_collect_config()
	_store.save_conn(name, _config.to_conn_dict())
	_store.set_last_conn(name)
	_conn_name.clear()
	_refresh_conn_presets(name)

func _on_conn_save() -> void:
	var name := _selected_text(_conn_preset)
	if name.is_empty():
		return
	_collect_config()
	_store.save_conn(name, _config.to_conn_dict())

func _on_conn_delete() -> void:
	var name := _selected_text(_conn_preset)
	if name.is_empty():
		return
	_store.delete_conn(name)
	_refresh_conn_presets()

# --- Road preset handlers ------------------------------------------------

func _refresh_road_presets(select_name: String = "") -> void:
	_fill_option(_road_preset, _store.road_names(), select_name)

func _on_road_preset_selected(idx: int) -> void:
	var name := _road_preset.get_item_text(idx)
	_config.apply_road_dict(_store.get_road(name))
	_road_path.text = _config.road_path
	_store.set_last_road(name)

func _on_road_save_new() -> void:
	var name := _road_name.text.strip_edges()
	if name.is_empty():
		return
	_collect_config()
	_store.save_road(name, _config.to_road_dict())
	_store.set_last_road(name)
	_road_name.clear()
	_refresh_road_presets(name)

func _on_road_save() -> void:
	var name := _selected_text(_road_preset)
	if name.is_empty():
		return
	_collect_config()
	_store.save_road(name, _config.to_road_dict())

func _on_road_delete() -> void:
	var name := _selected_text(_road_preset)
	if name.is_empty():
		return
	_store.delete_road(name)
	_refresh_road_presets()

# --- Preset helpers ------------------------------------------------------

## Select and apply whatever the store recorded as last-used for each family.
func _select_last_presets() -> void:
	var last_conn := _store.get_last_conn()
	if not last_conn.is_empty() and _select_item(_conn_preset, last_conn):
		_on_conn_preset_selected(_conn_preset.selected)
	var last_road := _store.get_last_road()
	if not last_road.is_empty() and _select_item(_road_preset, last_road):
		_on_road_preset_selected(_road_preset.selected)

func _fill_option(opt: OptionButton, names: PackedStringArray, select_name: String) -> void:
	opt.clear()
	for n in names:
		opt.add_item(n)
	if not select_name.is_empty():
		_select_item(opt, select_name)

## Select the item whose text matches `name`. Returns true if found.
func _select_item(opt: OptionButton, name: String) -> bool:
	for i in opt.item_count:
		if opt.get_item_text(i) == name:
			opt.selected = i
			return true
	return false

func _selected_text(opt: OptionButton) -> String:
	if opt.selected < 0:
		return ""
	return opt.get_item_text(opt.selected)

## Persistence for OsiSettingsPanel presets, backed by a single ConfigFile.
##
## Connection presets and road presets are two INDEPENDENT families, namespaced
## by section prefix ("conn:" / "road:") inside one file. A name like "e6mini"
## can exist in both families without colliding. The [meta] section remembers
## the last-selected preset of each family so the UI can re-apply both on launch.
##
## In an exported build the file lives NEXT TO THE EXE (portable: copy the game
## folder and the presets travel with it). In the editor it falls back to
## `user://` so dev runs don't drop files beside the Godot binary. Never uses
## `res://`, which is read-only inside an exported .pck.
## This class has no UI / pipeline knowledge, so it is testable headless.
class_name OsiPresetStore
extends RefCounted

const FILE_NAME := "osi_settings.cfg"
const EDITOR_PATH := "user://osi_settings.cfg"
const SCHEMA_VERSION := 1

## Default location: beside the executable in an exported build, `user://` in
## the editor. ConfigFile accepts the resulting absolute OS path directly.
static func default_path() -> String:
	if OS.has_feature("editor"):
		return EDITOR_PATH
	return OS.get_executable_path().get_base_dir().path_join(FILE_NAME)

const CONN_PREFIX := "conn:"
const ROAD_PREFIX := "road:"
const META := "meta"
const K_LAST_CONN := "last_conn"
const K_LAST_ROAD := "last_road"
const K_VERSION := "version"

var _path: String
var _cfg := ConfigFile.new()

## `path` is overridable so tests can target a temp file; empty uses default_path().
func _init(path: String = "") -> void:
	_path = path if not path.is_empty() else default_path()

## The absolute/`user://` path this store reads and writes (for display).
func config_path() -> String:
	return _path

## Load presets from disk. A missing file is a normal first run (starts empty).
func load() -> void:
	var err := _cfg.load(_path)
	if err != OK and err != ERR_FILE_NOT_FOUND:
		push_warning("[OsiPresetStore] load failed (%d) for %s" % [err, _path])

## Persist the in-memory config to disk, stamping the schema version.
func save() -> void:
	_cfg.set_value(META, K_VERSION, SCHEMA_VERSION)
	var err := _cfg.save(_path)
	if err != OK:
		push_warning("[OsiPresetStore] save failed (%d) for %s" % [err, _path])

# --- Connection family ---------------------------------------------------

func conn_names() -> PackedStringArray:
	return _names_with_prefix(CONN_PREFIX)

func get_conn(name: String) -> Dictionary:
	return _read_section(CONN_PREFIX + name)

func save_conn(name: String, d: Dictionary) -> void:
	_write_section(CONN_PREFIX + name, d)
	save()

func delete_conn(name: String) -> void:
	_erase_section(CONN_PREFIX + name)
	if get_last_conn() == name:
		_cfg.set_value(META, K_LAST_CONN, "")
	save()

# --- Road family ---------------------------------------------------------

func road_names() -> PackedStringArray:
	return _names_with_prefix(ROAD_PREFIX)

func get_road(name: String) -> Dictionary:
	return _read_section(ROAD_PREFIX + name)

func save_road(name: String, d: Dictionary) -> void:
	_write_section(ROAD_PREFIX + name, d)
	save()

func delete_road(name: String) -> void:
	_erase_section(ROAD_PREFIX + name)
	if get_last_road() == name:
		_cfg.set_value(META, K_LAST_ROAD, "")
	save()

# --- Last-selected meta --------------------------------------------------

func get_last_conn() -> String:
	return _cfg.get_value(META, K_LAST_CONN, "")

func set_last_conn(name: String) -> void:
	_cfg.set_value(META, K_LAST_CONN, name)
	save()

func get_last_road() -> String:
	return _cfg.get_value(META, K_LAST_ROAD, "")

func set_last_road(name: String) -> void:
	_cfg.set_value(META, K_LAST_ROAD, name)
	save()

# --- Internal helpers ----------------------------------------------------

func _names_with_prefix(prefix: String) -> PackedStringArray:
	var names := PackedStringArray()
	for section in _cfg.get_sections():
		if section.begins_with(prefix):
			names.append(section.substr(prefix.length()))
	names.sort()
	return names

func _read_section(section: String) -> Dictionary:
	var d := {}
	if not _cfg.has_section(section):
		return d
	for key in _cfg.get_section_keys(section):
		d[key] = _cfg.get_value(section, key)
	return d

func _write_section(section: String, d: Dictionary) -> void:
	for key in d:
		_cfg.set_value(section, key, d[key])

func _erase_section(section: String) -> void:
	if _cfg.has_section(section):
		_cfg.erase_section(section)

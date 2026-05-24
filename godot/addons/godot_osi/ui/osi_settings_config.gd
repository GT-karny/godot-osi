## Typed settings shared between OsiSettingsPanel and an integrator.
##
## Holds both the OSI connection settings (which map 1:1 to OsiReceiver /
## OsiMockServer properties) and the OpenDRIVE road path. The panel hands a
## duplicate of this to the integrator via `apply_connection(config)`; the
## integrator reads the fields straight onto the native nodes.
##
## Connection and road fields are kept as separate dict views (to_conn_dict /
## to_road_dict) so the two preset families can be saved independently.
class_name OsiSettingsConfig
extends Resource

# --- Connection (maps to OsiReceiver + OsiMockServer) ---
## true = run the bundled OsiMockServer; false = connect to an external gRPC OSI source.
@export var use_mock: bool = true
@export var address: String = "127.0.0.1"        # OsiReceiver.address / OsiMockServer.address
@export var port: int = 50051                    # OsiReceiver.port / OsiMockServer.port
@export var use_tls: bool = false                # OsiReceiver.use_tls
@export var reconnect: bool = true               # OsiReceiver.reconnect
@export var reconnect_delay_ms: int = 1000       # OsiReceiver.reconnect_delay_ms
@export var mock_period_ms: int = 50             # OsiMockServer.period_ms

# --- Road (argument to OsiRoadNetwork.load) ---
@export var road_path: String = ""

## Deep copy so callers can store the config without the panel mutating it later.
func duplicate_config() -> OsiSettingsConfig:
	return duplicate(true) as OsiSettingsConfig

## Connection fields as a flat dict for ConfigFile persistence.
func to_conn_dict() -> Dictionary:
	return {
		"use_mock": use_mock,
		"address": address,
		"port": port,
		"use_tls": use_tls,
		"reconnect": reconnect,
		"reconnect_delay_ms": reconnect_delay_ms,
		"mock_period_ms": mock_period_ms,
	}

## Road fields as a flat dict for ConfigFile persistence.
func to_road_dict() -> Dictionary:
	return {"road_path": road_path}

## Apply a connection dict, keeping current values for any missing key
## (forward-compatible with presets written by an older/newer version).
func apply_conn_dict(d: Dictionary) -> void:
	use_mock = d.get("use_mock", use_mock)
	address = d.get("address", address)
	port = int(d.get("port", port))
	use_tls = d.get("use_tls", use_tls)
	reconnect = d.get("reconnect", reconnect)
	reconnect_delay_ms = int(d.get("reconnect_delay_ms", reconnect_delay_ms))
	mock_period_ms = int(d.get("mock_period_ms", mock_period_ms))

## Apply a road dict (only road_path).
func apply_road_dict(d: Dictionary) -> void:
	road_path = d.get("road_path", road_path)

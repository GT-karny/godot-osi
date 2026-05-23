# Headless structural test for the generated converter (run as the SceneTree
# main loop). Asserts the typed snapshot from OsiTestRig.make_sample_ground_truth
# preserves the raw OSI values 1:1 (the "A" geometry policy). Exits 0 on pass.
extends SceneTree

func _initialize() -> void:
	var ok := true
	var rig := OsiTestRig.new()
	var snap = rig.make_sample_ground_truth()

	ok = _eq("moving_object count", snap.moving_object.size(), 1) and ok
	ok = _eq("host_vehicle_id.value", snap.host_vehicle_id.value, 7) and ok

	var mo = snap.moving_object[0]
	ok = _eq("moving_object[0].id.value", mo.id.value, 42) and ok

	var base = mo.base
	# Raw OSI values must be preserved (no coordinate transform in the mirror).
	ok = _approx("base.position.x (raw)", base.position.x, 10.0) and ok
	ok = _approx("base.position.y (raw)", base.position.y, 5.0) and ok
	ok = _approx("base.position.z (raw)", base.position.z, 1.0) and ok
	ok = _approx("base.dimension.length", base.dimension.length, 4.5) and ok
	ok = _approx("base.dimension.width", base.dimension.width, 1.8) and ok
	ok = _approx("base.dimension.height", base.dimension.height, 1.5) and ok

	if ok:
		print("ITEST PASS")
	else:
		printerr("ITEST FAIL")
	quit(0 if ok else 1)

func _eq(name: String, got, want) -> bool:
	if got != want:
		printerr("FAIL %s: got %s, want %s" % [name, got, want])
		return false
	print("ok  %s = %s" % [name, got])
	return true

func _approx(name: String, got: float, want: float) -> bool:
	if abs(got - want) > 1e-9:
		printerr("FAIL %s: got %f, want %f" % [name, got, want])
		return false
	print("ok  %s = %f" % [name, got])
	return true

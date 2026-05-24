# Headless smoke test for the OpenDRIVE road support.
#
#   OsiRoadNetwork.load(.xodr) -> OsiRoadNetworkVisualizer.build_from()
#
# Verifies the native esmini RoadManager loads a bundled map, reports the
# expected road/sign counts, and that the visualizer produces a road surface
# MeshInstance3D. Node creation and mesh building work headless (no display).
#
#   Godot ... --headless --path godot --script res://test/road_smoke.gd
# Exits 0 on success, 1 on failure.
extends SceneTree

# straight_500m_signs.xodr is deterministic: 1 road, 17 signs (matches the
# esmini-rm Rust regression test).
const ROAD := "res://examples/roads/straight_500m_signs.xodr"
const EXPECT_ROADS := 1
const EXPECT_SIGNS := 17

func _initialize() -> void:
	var ok := true

	var net := OsiRoadNetwork.new()
	if not net.load(ROAD):
		push_error("[road] load failed: %s" % ROAD)
		_finish(false)
		return

	var roads := net.road_count()
	var signs := net.sign_count()
	print("[road] loaded %s: roads=%d signs=%d" % [ROAD, roads, signs])
	if roads != EXPECT_ROADS:
		push_error("[road] expected %d roads, got %d" % [EXPECT_ROADS, roads])
		ok = false
	if signs != EXPECT_SIGNS:
		push_error("[road] expected %d signs, got %d" % [EXPECT_SIGNS, signs])
		ok = false

	# Road-driving query path (drivable lanes + lane-center points).
	var rid := net.road_id_at(0)
	var lanes := net.drivable_lanes(rid, 0.0)
	print("[road] road_id=%d drivable_lanes=%s" % [rid, str(lanes)])
	if lanes.is_empty():
		push_error("[road] expected at least one drivable lane")
		ok = false
	else:
		var p := net.lane_point(rid, lanes[0], 250.0)
		print("[road] lane %d center at s=250: %s" % [lanes[0], str(p)])
		if p == Vector3.ZERO:
			push_error("[road] lane_point returned origin (lookup failed)")
			ok = false

	# Extended RoadManager queries (Dictionary/Array surface).
	if not _check_extended(net, rid):
		ok = false

	var viz := OsiRoadNetworkVisualizer.new()
	root.add_child(viz)
	viz.build_from(net)

	if not viz.has_surface():
		push_error("[road] visualizer produced no road surface")
		ok = false

	# The surface child should be a MeshInstance3D carrying a mesh.
	var surface := viz.get_node_or_null(NodePath("RoadSurface")) as MeshInstance3D
	if surface == null or surface.mesh == null:
		push_error("[road] RoadSurface MeshInstance3D missing or empty")
		ok = false
	else:
		var aabb := surface.mesh.get_aabb()
		print("[road] road surface AABB size=%s" % str(aabb.size))
		if aabb.size.length() < 1.0:
			push_error("[road] road surface mesh looks degenerate")
			ok = false

	# Road marks (OpenDRIVE <roadMark>): lane lines built from RoadManager.
	var marks := viz.get_node_or_null(NodePath("RoadMarks")) as MeshInstance3D
	if marks == null or marks.mesh == null:
		push_error("[road] RoadMarks mesh missing (no <roadMark> geometry built)")
		ok = false
	else:
		var verts: int = 0
		var arrs := marks.mesh.surface_get_arrays(0)
		if arrs.size() > Mesh.ARRAY_VERTEX and arrs[Mesh.ARRAY_VERTEX] != null:
			verts = (arrs[Mesh.ARRAY_VERTEX] as PackedVector3Array).size()
		print("[road] road marks: %d vertices, AABB size=%s" % [verts, str(marks.mesh.get_aabb().size)])
		if verts < 3:
			push_error("[road] road-mark mesh has no triangles")
			ok = false

	_finish(ok)

# Exercise the extended Dictionary/Array query surface on the loaded straight
# road (1 road, several lane sections, 13 objects, 17 signals).
func _check_extended(net: OsiRoadNetwork, rid: int) -> bool:
	var ok := true

	var geoms := net.geometries(rid)
	print("[road] geometries=%d" % geoms.size())
	if geoms.is_empty():
		push_error("[road] expected reference-line geometry")
		ok = false

	var sections := net.lane_sections(rid)
	if sections.is_empty():
		push_error("[road] expected lane sections")
		ok = false
	else:
		var lanes := net.lanes(rid, 0)
		if lanes.is_empty():
			push_error("[road] expected lanes in section 0")
			ok = false
		# Reference-line OSI points for the first section.
		var osi := net.lane_osi_points(rid, 0, 0, OsiRoadNetwork.OSI_REF_LINE)
		print("[road] section0 lanes=%d ref-line OSI points=%d" % [lanes.size(), osi.size()])
		if osi.is_empty():
			push_error("[road] expected reference-line OSI points")
			ok = false

	var objects := net.road_objects(rid)
	print("[road] road objects=%d" % objects.size())
	if objects.size() != 13:
		push_error("[road] expected 13 road objects, got %d" % objects.size())
		ok = false

	var signals := net.signals()
	if signals.size() != EXPECT_SIGNS:
		push_error("[road] expected %d detailed signals, got %d" % [EXPECT_SIGNS, signals.size()])
		ok = false

	var info := net.network_info()
	print("[road] network info=%s" % str(info))
	if info.is_empty():
		push_error("[road] expected network info")
		ok = false

	var dist := net.shortest_path_distance(rid, 0.0, rid, 500.0)
	print("[road] shortest path 0->500: %f" % dist)
	if is_nan(dist) or absf(absf(dist) - 500.0) > 5.0:
		push_error("[road] unexpected shortest-path distance %f" % dist)
		ok = false

	return ok

func _finish(ok: bool) -> void:
	if ok:
		print("[road] OK")
	else:
		print("[road] FAILED")
	quit(0 if ok else 1)

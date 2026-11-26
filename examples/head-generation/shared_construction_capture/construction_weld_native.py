"""Observe original native weld operations; admit no downstream field/surgery roles."""
from construction_markers import MARKERS, attach, read, clear_verified, fields
from construction_topology import cycle, digest
from construction_weld_contract import exact_index, validate_weld
from construction_bmesh_state import snapshot, verify_dissolve


def joined_source(obj, packets):
    if not packets or any("native_join" not in packet for packet in packets):
        raise ValueError("Weld requires the actual verified constructor join")
    receipt = packets[0]["native_join"]
    if any(digest(packet["native_join"]) != digest(receipt) for packet in packets):
        raise ValueError("Mixed native join receipts before weld")
    source, maps = receipt["source"], receipt["identity_maps"]
    if digest(source) != receipt["source_sha256"]:
        raise ValueError("Changed native join source receipt")
    domains = {"POINT": len(obj.data.vertices), "FACE": len(obj.data.polygons), "CORNER": len(obj.data.loops)}
    tables = {"POINT": source["points"], "FACE": source["faces"], "CORNER": source["corners"]}
    for domain, count in domains.items():
        if len(maps[domain]) != count or len(tables[domain]) != count:
            raise ValueError("Join-to-weld domain count changed")
        if any(type(i) is not int for i in maps[domain]) or sorted(maps[domain]) != list(range(1, count + 1)):
            raise ValueError("Join-to-weld ordinal permutation changed")
    pids = [source["points"][i - 1] for i in maps["POINT"]]
    cids = [source["corners"][i - 1] for i in maps["CORNER"]]
    face_ids = []
    faces = []
    for face in obj.data.polygons:
        old = source["faces"][maps["FACE"][face.index] - 1]
        current = [(cids[i], pids[obj.data.loops[i].vertex_index]) for i in face.loop_indices]
        if cycle(current) != cycle(list(zip(old["corner_ids"], old["point_ids"]))):
            raise ValueError("Native topology changed between join and weld")
        face_ids.append(old["face_id"])
        faces.append({"points": list(face.vertices), "corners": list(face.loop_indices)})
    return {"point_ids": pids, "face_ids": face_ids, "corner_ids": cids,
            "faces": faces, "edges": [list(e.vertices) for e in obj.data.edges],
            "join_source_sha256": receipt["source_sha256"]}


def bm_markers(bm):
    point = bm.verts.layers.int.get(MARKERS["POINT"])
    face = bm.faces.layers.int.get(MARKERS["FACE"])
    corner = bm.loops.layers.int.get(MARKERS["CORNER"])
    if any(layer is None for layer in (point, face, corner)):
        raise ValueError("Native BMesh lost typed provenance storage")
    return point, face, corner


def decoded_bmesh(bm):
    point, face, corner = bm_markers(bm)
    verts = list(bm.verts)
    indices = {vertex: index for index, vertex in enumerate(verts)}
    return {"point_origins": [v[point] - 1 for v in verts],
            "faces": [{"face_origin": f[face] - 1, "points": [indices[loop.vert] for loop in f.loops],
                       "corner_origins": [loop[corner] - 1 for loop in f.loops]} for f in bm.faces],
            "edges": [[indices[v] for v in e.verts] for e in bm.edges]}


def decoded_mesh(obj):
    markers = read(obj)
    return {"point_origins": [i - 1 for i in markers["POINT"]],
            "faces": [{"face_origin": markers["FACE"][f.index] - 1, "points": list(f.vertices),
                       "corner_origins": [markers["CORNER"][i] - 1 for i in f.loop_indices]} for f in obj.data.polygons],
            "edges": [list(e.vertices) for e in obj.data.edges]}


def execute_native_weld(bm, before, *, weld_distance=0.000001, dissolve_distance=0.0000001):
    """Original operators and ordered inputs stay unchanged; observation is extra."""
    import bmesh
    import bpy
    if tuple(bpy.app.version) != (4, 3, 2):
        raise ValueError("Native weld lineage requires pinned Blender 4.3.2")
    if weld_distance != 0.000001 or dissolve_distance != 0.0000001:
        raise ValueError("Changed native constructor weld/dissolve distances")
    validate_weld(before, [], decoded_bmesh(bm))
    from construction_weld_edges import bind_edges, verify_edge_donors
    edge_layer, original_edges = bind_edges(bm, before)
    original_verts = list(bm.verts)
    point_layer, face_layer, _ = bm_markers(bm)
    vertex_origins = {v: v[point_layer] - 1 for v in original_verts}
    vertex_coordinates = {v: list(v.co) for v in original_verts}
    face_origins = {f: f[face_layer] - 1 for f in bm.faces}
    pre_find, pre_identity = snapshot(bm)
    native_map = bmesh.ops.find_doubles(bm, verts=original_verts, dist=weld_distance)["targetmap"]
    post_find, _ = snapshot(bm, pre_identity)
    if digest(pre_find) != digest(post_find):
        raise ValueError("Native target-map observation mutated BMesh source")
    original_set = set(original_verts)
    if any(source not in original_set or target not in original_set for source, target in native_map.items()):
        raise ValueError("Native weld target is outside the exact operator input")
    pairs = sorted([[source[point_layer] - 1, target[point_layer] - 1] for source, target in native_map.items()])
    from construction_weld_contract import terminal_map, projected_face
    targets = terminal_map(len(before["point_ids"]), pairs)
    for face in before["faces"]:
        projected_face(face["points"], face["corners"], targets)
    expected_live_vertices = original_set - set(native_map)
    bmesh.ops.remove_doubles(bm, verts=original_verts, dist=weld_distance)
    actual_verts = set(bm.verts)
    if actual_verts != expected_live_vertices:
        raise ValueError("Actual live native vertices differ from target-map survivors")
    if any(v[point_layer] - 1 != vertex_origins[v] or digest(list(v.co)) != digest(vertex_coordinates[v]) for v in actual_verts):
        raise ValueError("Native target vertex identity/coordinate changed")
    if any(f not in face_origins or f[face_layer] - 1 != face_origins[f] for f in bm.faces):
        raise ValueError("Unreceipted native weld face-object birth or donor replacement")
    after_weld = decoded_bmesh(bm)
    quotient = validate_weld(before, pairs, after_weld)
    edge_donors = verify_edge_donors(bm, edge_layer, original_edges, quotient)
    welded_state, welded_identity = snapshot(bm)
    bmesh.ops.dissolve_degenerate(bm, edges=list(bm.edges), dist=dissolve_distance)
    noop = verify_dissolve(bm, welded_state, welded_identity, native_version=bpy.app.version)
    return {"before": before, "native_target_pairs": pairs, "after_weld": after_weld,
            "quotient": quotient, "pre_find_state": pre_find, "post_weld_state": welded_state,
            "edge_donors": edge_donors, "dissolve": noop, "operator": {"weld_distance": weld_distance, "dissolve_distance": dissolve_distance,
            "targetmap": "actual native find_doubles with identical ordered verts; original remove_doubles retained"}}


def capture_weld(obj, packets):
    """Root pipeline opt-in; temporary IDs are removed on every failure path."""
    import bmesh
    before = joined_source(obj, packets)
    before_fields = fields(obj)
    from construction_weld_edges import EDGE_MARKER, attach_edge_mesh, clear_edge_mesh, unmarked_fields
    if obj.data.attributes.get(EDGE_MARKER) is not None:
        raise ValueError("Native weld edge marker collision")
    bm = None
    attach(obj, {"POINT": list(range(1, len(before["point_ids"]) + 1)),
                 "FACE": list(range(1, len(before["faces"]) + 1)),
                 "CORNER": list(range(1, len(before["corner_ids"]) + 1))})
    try:
        attach_edge_mesh(obj)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        receipt = execute_native_weld(bm, before)
        bm.to_mesh(obj.data)
        observed = decoded_mesh(obj)
        receipt["mesh_quotient"] = validate_weld(before, receipt["native_target_pairs"], observed)
        receipt["after_mesh"] = observed
        receipt["before_native_fields"] = before_fields
        receipt["after_native_fields"] = unmarked_fields(obj)
        receipt["current_role_admission"] = False
        receipt["scope"] = "verified weld quotient and explicit dissolve scratch-tag receipt; source field donors and later surgery remain pending"
        receipt["sha256"] = digest(receipt)
    finally:
        if bm is not None:
            bm.free()
        try:
            clear_edge_mesh(obj)
        finally:
            clear_verified(obj)
    for packet in packets:
        packet["native_weld"] = receipt
        packet["native_transport"] = {"stage": "weld_verified_dissolve_scratch_receipted", "current_role_admission": False,
            "reason": "No field donor admission or later surgery replacement receipt"}
        packet["solver_ready"] = False
    return receipt

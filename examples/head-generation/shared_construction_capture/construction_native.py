"""Native constructor lineage checkpoints; unreceipted surgery stays unbound."""
from construction_markers import attach, clear_verified, read
from construction_mirror_contract import validate_mirror_lineage
from construction_topology import digest, cycle, stable_id
from construction_boundaries import boundary_facts

HALVES = {}


def _object_key(obj):
    key = obj.as_pointer()
    if type(key) is not int or key <= 0:
        raise ValueError("Invalid native constructor object identity")
    return key


def _state_for(obj):
    key = _object_key(obj)
    state = HALVES.get(key)
    if state is not None:
        if state["object"] is not obj or state["name"] != obj.name:
            raise ValueError("Captured constructor object renamed/replaced/reused")
        if state["phase"] not in ("half_created", "mirrored"):
            raise ValueError("Retired constructor object identity cannot be reused")
    elif any(row["name"] == obj.name for row in HALVES.values()):
        raise ValueError("Replacement object borrowed a captured constructor name")
    return state


def _mesh_key(obj):
    key = obj.data.as_pointer()
    if type(key) is not int or key <= 0:
        raise ValueError("Invalid native constructor mesh identity")
    return key


def _topology_digest(obj):
    data = obj.data
    return digest({"points": len(data.vertices), "edges": [list(edge.vertices) for edge in data.edges],
        "faces": [[face.index, list(face.vertices), list(face.loop_indices)] for face in data.polygons],
        "corners": [[loop.vertex_index, loop.edge_index] for loop in data.loops]})


def _verify_mesh_state(obj, state):
    if state["mesh_key"] != _mesh_key(obj) or state["mesh"].as_pointer() != state["mesh_key"]:
        raise ValueError("Captured constructor mesh datablock replaced")
    if state["topology_sha256"] != _topology_digest(obj):
        raise ValueError("Captured constructor topology changed between verified stages")


def bind_half(obj, packet):
    """Called immediately after actual mesh creation by its generating function."""
    key = _object_key(obj)
    if key in HALVES or any(row["name"] == obj.name for row in HALVES.values()) or len(obj.data.vertices) != len(packet["point_ids"]):
        raise ValueError("Constructor native half identity collision/count mismatch")
    actual = [[packet["point_ids"][i] for i in face.vertices] for face in obj.data.polygons]
    if actual != [row["point_ids"] for row in packet["faces"]]:
        raise ValueError("Native half creation changed constructor topology")
    HALVES[key] = {"object": obj, "name": obj.name, "mesh": obj.data, "mesh_key": _mesh_key(obj),
                   "topology_sha256": _topology_digest(obj), "phase": "half_created", "packet": packet, "source": {key: packet[key] for key in ("source_namespace", "point_ids", "faces")}}
    packet["native_transport"] = {"stage": "half_mesh_created", "current_role_admission": False}


def _decoded(obj, source, markers):
    tables = {"POINT": source["point_ids"], "FACE": [row["face_id"] for row in source["faces"]],
              "CORNER": [cid for row in source["faces"] for cid in row["corner_ids"]]}
    decoded = {}
    for domain, values in markers.items():
        if any(not 1 <= value <= len(tables[domain]) for value in values):
            raise ValueError("Unbound native constructor marker")
        decoded[domain] = [tables[domain][value - 1] for value in values]
    return {"source_namespace": source["source_namespace"],
        "vertices": [{"index": v.index, "source_point_id": decoded["POINT"][v.index]} for v in obj.data.vertices],
        "faces": [{"index": face.index, "source_face_id": decoded["FACE"][face.index],
            "vertex_indices": list(face.vertices), "corner_indices": list(face.loop_indices),
            "source_corner_ids": [decoded["CORNER"][i] for i in face.loop_indices]} for face in obj.data.polygons]}


def apply_modifiers(obj):
    import bpy
    state = _state_for(obj)
    if state is None:
        for modifier in list(obj.modifiers):
            if modifier.type == "SUBSURF" and modifier.levels == 0:
                obj.modifiers.remove(modifier)
            else:
                bpy.ops.object.modifier_apply(modifier=modifier.name)
        return
    _verify_mesh_state(obj, state)
    if state["phase"] != "half_created":
        raise ValueError("Constructor modifiers already consumed")
    mirrors = [m for m in obj.modifiers if m.type == "MIRROR"]
    if (len(mirrors) != 1 or any(m.type != "MIRROR" and not (m.type == "SUBSURF" and m.levels == 0) for m in obj.modifiers)
            or list(mirrors[0].use_axis) != [True, False, False]
            or any(mirrors[0].use_bisect_axis) or not mirrors[0].use_mirror_merge):
        raise ValueError("Unadmitted constructor modifier/axis contract")
    source = state["source"]
    if [[source["point_ids"][i] for i in face.vertices] for face in obj.data.polygons] != [row["point_ids"] for row in source["faces"]]:
        raise ValueError("Constructor half topology changed before Mirror")
    attach(obj, {"POINT": list(range(1, len(source["point_ids"]) + 1)),
        "FACE": list(range(1, len(source["faces"]) + 1)),
        "CORNER": list(range(1, sum(len(row["corner_ids"]) for row in source["faces"]) + 1))})
    try:
        for modifier in list(obj.modifiers):
            if modifier.type == "SUBSURF" and modifier.levels == 0:
                obj.modifiers.remove(modifier)
            else:
                bpy.ops.object.modifier_apply(modifier=modifier.name)
        observed = _decoded(obj, source, read(obj))
        roles = {name + "/segment/" + str(i): path for name, row in state["packet"]["construction_roles"].items()
                 for i, path in enumerate(row["segments"])}
        receipt = validate_mirror_lineage(source, observed, seam_point_ids=state["packet"]["seam_point_ids"], role_paths=roles)
        point_ids = [None] * len(obj.data.vertices)
        for pid, sides in receipt["point_map"].items():
            for side, index in sides.items():
                point_ids[index] = stable_id(source["source_namespace"], "mirrored_point", [pid, "seam" if pid in receipt["seam_equivalence"] else side])
    finally:
        clear_verified(obj)
    state["packet"]["native_mirror"] = receipt
    state["packet"]["native_transport"] = {"stage": "mirror_verified", "current_role_admission": False}
    state["packet"]["post_mirror_boundary_facts"] = boundary_facts(
        [[point_ids[i] for i in face.vertices] for face in obj.data.polygons], point_ids)
    state["mirrored_point_ids"] = point_ids
    state["phase"] = "mirrored"
    state.update(mesh=obj.data, mesh_key=_mesh_key(obj), topology_sha256=_topology_digest(obj))


def join_objects(members):
    """Observe all parts explicitly; unrelated parts acquire no rail authority."""
    import bpy
    source = {"points": [], "faces": [], "corners": [], "parts": []}
    attached = []
    packets = []
    try:
        for part_ordinal, obj in enumerate(members):
            state = _state_for(obj)
            if state is not None and "mirrored_point_ids" not in state:
                raise ValueError("Missing verified constructor Mirror receipt before join")
            if state is not None:
                _verify_mesh_state(obj, state)
                packets.append(state["packet"])
            namespace = digest(["native_component_join", part_ordinal, obj.name])
            pids = state["mirrored_point_ids"] if state else [stable_id(namespace, "point", [i]) for i in range(len(obj.data.vertices))]
            point_offset, face_offset, corner_offset = len(source["points"]), len(source["faces"]), len(source["corners"])
            source["points"].extend(pids)
            face_origins, corner_origins = {}, {}
            if state is not None:
                mirror = state["packet"]["native_mirror"]
                face_origins = {index: [fid, side] for fid, sides in mirror["face_map"].items() for side, index in sides.items()}
                corner_origins = {row["index"]: [cid, side] for cid, sides in mirror["corner_map"].items() for side, row in sides.items()}
                namespace = state["source"]["source_namespace"]
            source["parts"].append({"name": obj.name, "ordinal": part_ordinal,
                "source_namespace": namespace, "constructor_bound": state is not None,
                "point_offset": point_offset, "face_offset": face_offset, "corner_offset": corner_offset,
                "counts": [len(obj.data.vertices), len(obj.data.polygons), len(obj.data.loops)],
                "face_origins": face_origins, "corner_origins": corner_origins})
            for face in obj.data.polygons:
                fid = stable_id(namespace, "mirrored_face" if state else "face", face_origins[face.index] if state else [face.index])
                cids = [stable_id(namespace, "mirrored_corner" if state else "corner", corner_origins[i] if state else [i]) for i in face.loop_indices]
                source["faces"].append({"face_id": fid, "point_ids": [pids[i] for i in face.vertices], "corner_ids": cids})
                source["corners"].extend(cids)
            attach(obj, {"POINT": list(range(point_offset + 1, len(source["points"]) + 1)),
                "FACE": list(range(face_offset + 1, len(source["faces"]) + 1)),
                "CORNER": list(range(corner_offset + 1, len(source["corners"]) + 1))})
            attached.append(obj)
        for ids in (source["points"], [row["face_id"] for row in source["faces"]], source["corners"]):
            if len(ids) != len(set(ids)):
                raise ValueError("Duplicate constructor instance identity before join")
        bpy.ops.object.join()
        target = members[0]
        markers = read(target)
        for domain, rows in (("POINT", source["points"]), ("FACE", source["faces"]), ("CORNER", source["corners"])):
            if sorted(markers[domain]) != list(range(1, len(rows) + 1)):
                raise ValueError("Join lost/merged/duplicated constructor identities")
        for face in target.data.polygons:
            original = source["faces"][markers["FACE"][face.index] - 1]
            actual = [(source["corners"][markers["CORNER"][i] - 1], source["points"][markers["POINT"][target.data.loops[i].vertex_index] - 1]) for i in face.loop_indices]
            if cycle(actual) != cycle(list(zip(original["corner_ids"], original["point_ids"]))):
                raise ValueError("Join changed oriented corner/source ownership")
        receipt = {"source_sha256": digest(source), "identity_maps": markers,
                   "source": source, "scope": "actual native join only; no weld/surgery admission"}
    finally:
        cleanup_errors = []
        for obj in attached:
            try:
                if obj.as_pointer() == 0:
                    continue
            except ReferenceError:
                continue  # A deleted join input has no live marker storage.
            try:
                clear_verified(obj)
            except Exception as error:
                cleanup_errors.append(error)
        if cleanup_errors:
            raise RuntimeError("Constructor join cleanup failed after attempting every live object") from cleanup_errors[0]
    for packet in packets:
        packet["native_join"] = receipt
        packet["native_transport"] = {"stage": "join_verified", "current_role_admission": False}
        for state in HALVES.values():
            if state["packet"] is packet:
                state["phase"] = "joined"
                state.pop("mirrored_point_ids", None)
                state["mesh"] = None
    return packets


def invalidate_before_surgery(packets, stage):
    for packet in packets:
        packet["native_transport"] = {"stage": "unbound_before_" + stage, "current_role_admission": False,
            "reason": "No explicit constructor replacement/merge receipt; previous native indices are historical only"}
        packet["solver_ready"] = False
        for state in HALVES.values():
            if state["packet"] is packet:
                state["phase"] = "invalidated_before_surgery"
                state["object"] = None
                state.pop("mirrored_point_ids", None)
                state["mesh"] = None


def require_current_roles(packet):
    raise ValueError("Constructor roles are not admitted through native weld/surgery and field boundaries")

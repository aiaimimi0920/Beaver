"""Bracket only the existing native normal setter; preserve authored edge authority."""

import hashlib
from shared_capture_contract import encode, same, source_payload, validate_vectors
from shared_capture_native import read_mesh, read_identities, clear_markers, restore_triangulated_editor_attributes

NORMAL_ATTRIBUTES = {"custom_normal": ("CORNER", "INT16_2D"), "sharp_edge": ("EDGE", "BOOLEAN")}


def normal_changes(before, after):
    """Only actual native generated sharp/custom-normal storage may differ here."""
    changes = {"edge_sharp": [], "native_attributes": {}}
    for key in ("vertices", "faces", "corners", "uv_layers", "materials", "object_matrix_world"):
        if not same(before[key], after[key]):
            raise ValueError("Native normal setter altered source " + key)
    if len(before["edges"]) != len(after["edges"]):
        raise ValueError("Native normal setter changed edge count")
    for i, (a, b) in enumerate(zip(before["edges"], after["edges"])):
        if {k: v for k, v in a.items() if k != "sharp"} != {k: v for k, v in b.items() if k != "sharp"}:
            raise ValueError("Native normal setter changed authored non-sharp edge fields")
        if a["sharp"] != b["sharp"]:
            changes["edge_sharp"].append({"runtime_edge": i, "before": a["sharp"], "after": b["sharp"]})
    for name in set(before["attributes"]) | set(after["attributes"]):
        a, b = before["attributes"].get(name), after["attributes"].get(name)
        if same(a, b):
            continue
        if name not in NORMAL_ATTRIBUTES or b is None:
            raise ValueError("Native normal setter changed authored attribute: " + name)
        expected = NORMAL_ATTRIBUTES[name]
        if any((row["domain"], row["type"]) != expected for row in (a, b) if row is not None):
            raise ValueError("Unexpected native generated normal storage")
        changes["native_attributes"][name] = {"before": a, "after": b}
    return changes


def set_captured_normals(obj, capture, requested_normals):
    """Call the identical production setter once and record its isolated effects."""
    data = obj.data
    try:
        if "normal_stage_json" in capture:
            raise ValueError("Native normal capture cannot be rebased")
        identities = read_identities(data)
        before, restoration = restore_triangulated_editor_attributes(obj, source_payload(capture)["mesh"], identities)
        validate_vectors([list(row) for row in requested_normals], 3, "requested native normals")
        data.normals_split_custom_set(requested_normals)
        after = read_mesh(obj)
        changes = normal_changes(before, after)
        stage = {"source_sha256": capture["source_sha256"], "identities": identities,
                 "transport_attribute_restoration": restoration,
                 "before": before, "after": after, "changes": changes,
                 "requested_corner_normals": [list(row) for row in requested_normals],
                 "actual_corner_normals": [list(row.vector) for row in data.corner_normals]}
        raw = encode(stage)
        capture["normal_stage_json"] = raw
        capture["normal_stage_sha256"] = hashlib.sha256(raw.encode()).hexdigest()
    except Exception:
        clear_markers(data)
        raise

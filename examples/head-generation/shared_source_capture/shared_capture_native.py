"""Read real native mesh channels and saved keys; never synthesize missing data."""

from shared_capture_contract import MARKERS, KEYS, freeze_source, complete_capture, encode, digest, same, validate_transport

ATTRIBUTE_PROPERTIES = {
    "FLOAT": "value", "INT": "value", "INT8": "value", "BOOLEAN": "value",
    "STRING": "value", "FLOAT_VECTOR": "vector", "FLOAT2": "vector",
    "FLOAT_COLOR": "color", "BYTE_COLOR": "color", "INT32_2D": "value",
    "INT16_2D": "value", "QUATERNION": "value", "FLOAT4X4": "value",
}


def plain(value):
    if isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, bytes):
        return {"bytes_hex": value.hex()}
    return [plain(item) for item in value]


def read_mesh(obj):
    data = obj.data
    counts = {"POINT": len(data.vertices), "EDGE": len(data.edges),
              "FACE": len(data.polygons), "CORNER": len(data.loops)}
    attributes = {}
    for attr in data.attributes:
        if attr.name in MARKERS.values():
            continue
        prop = ATTRIBUTE_PROPERTIES.get(attr.data_type)
        if prop is None or attr.domain not in counts or len(attr.data) != counts[attr.domain]:
            raise ValueError("Unsupported native attribute: " + attr.name)
        attributes[attr.name] = {"domain": attr.domain, "type": attr.data_type,
                                 "values": [plain(getattr(row, prop)) for row in attr.data]}
    group_names = {group.index: group.name for group in obj.vertex_groups}
    result = {
        "vertices": [{"co": list(v.co), "groups": sorted(
            [group_names[g.group], float(g.weight)] for g in v.groups),
            "select": bool(v.select), "hide": bool(v.hide)} for v in data.vertices],
        "edges": [{"vertices": list(e.vertices), "seam": bool(e.use_seam),
                   "sharp": bool(e.use_edge_sharp), "select": bool(e.select),
                   "hide": bool(e.hide)} for e in data.edges],
        "faces": [{"vertices": list(p.vertices), "loops": list(p.loop_indices),
                   "material": int(p.material_index), "smooth": bool(p.use_smooth),
                   "select": bool(p.select), "hide": bool(p.hide)} for p in data.polygons],
        "corners": [{"vertex": int(loop.vertex_index), "edge": int(loop.edge_index)} for loop in data.loops],
        "native_corner_normals": [list(row.vector) for row in data.corner_normals],
        "attributes": attributes,
        "uv_layers": {layer.name: [{"uv": list(row.uv), "pin": bool(row.pin_uv),
                                    "select": bool(row.select), "select_edge": bool(row.select_edge)}
                                   for row in layer.data] for layer in data.uv_layers},
        "materials": [material.name if material else None for material in data.materials],
        "object_matrix_world": [list(row) for row in obj.matrix_world],
    }
    if len(result["uv_layers"]) != 3:
        raise ValueError("Capture requires the three actual authored UV layers")
    encode(result)
    return result


def read_identities(data):
    identities = {}
    counts = {"POINT": len(data.vertices), "FACE": len(data.polygons), "CORNER": len(data.loops)}
    for domain, name in MARKERS.items():
        attr = data.attributes.get(name)
        if (attr is None or (attr.domain, attr.data_type) != (domain, "INT")
                or len(attr.data) != counts[domain]):
            raise ValueError("Native source identity missing or changed")
        identities[domain] = [int(row.value) for row in attr.data]
    return identities


def restore_triangulated_editor_attributes(obj, source, identities):
    """Reapply authored editor selection after verified native triangulation."""
    before = read_mesh(obj)
    selection = {".select_vert": ("POINT", "vertices"),
                 ".select_edge": ("EDGE", "edges"), ".select_poly": ("FACE", "faces")}
    missing = set(source["attributes"]) - set(before["attributes"])
    if missing - set(selection) - {"sharp_face"}:
        raise ValueError("Unrecognized native attribute omission")
    for name, (domain, collection) in selection.items():
        original = source["attributes"].get(name)
        current = before["attributes"].get(name)
        if original is None and current is not None:
            raise ValueError("Unbound generated native selection storage: " + name)
        if (name != ".select_poly" and original is not None and current is None
                and any(original["values"])):
            raise ValueError("Nondefault native selection storage unexpectedly omitted: " + name)
        for snapshot, attr in ((source, original), (before, current)):
            public = [row["select"] for row in snapshot[collection]]
            if attr is None:
                if any(public):
                    raise ValueError("Selection missing storage but public RNA is nondefault")
            elif ((attr["domain"], attr["type"]) != (domain, "BOOLEAN")
                  or not same(attr["values"], public)):
                raise ValueError("Source/native selection attribute/public RNA disagreement: " + name)
    restored = {}
    if "sharp_face" in missing:
        attr = source["attributes"]["sharp_face"]
        if ((attr["domain"], attr["type"]) != ("FACE", "BOOLEAN")
                or any(not same(value, False) for value in attr["values"])
                or any(not face["smooth"] for face in before["faces"])):
            raise ValueError("Nondefault or mistyped native attribute omission: sharp_face")
        restored["sharp_face"] = {"domain": "FACE", "type": "BOOLEAN",
                                  "values": [False] * len(before["faces"])}
    # These projections validate only unaffected fields. Actual snapshots stay intact.
    # Every selection channel is separately typed/public-checked above and restored
    # from verified source identities below; it is never accepted as a new baseline.
    def unaffected(snapshot, omit_sharp=False):
        result = dict(snapshot, attributes={name: value for name, value in snapshot["attributes"].items()
                      if name not in selection and not (omit_sharp and name == "sharp_face")})
        for collection in ("vertices", "edges", "faces"):
            result[collection] = [dict(row, select=False) for row in snapshot[collection]]
        return result
    _, edge_ids = validate_transport(unaffected(source, "sharp_face" in missing),
                                     unaffected(before), identities)
    maps = dict(identities, EDGE=edge_ids)
    expected = dict(before, attributes=dict(before["attributes"]))
    for name, (domain, collection) in selection.items():
        values = [source[collection][original]["select"] if original is not None else row["select"]
                  for row, original in zip(before[collection], maps[domain])]
        expected[collection] = [dict(row, select=value) for row, value in zip(before[collection], values)]
        if name in source["attributes"]:
            target = {"domain": domain, "type": "BOOLEAN", "values": values}
            if not same(before["attributes"].get(name), target):
                restored[name] = target
    if not restored:
        validate_transport(source, before, identities)
        return before, None
    expected["attributes"].update(restored)
    for name, row in restored.items():
        if obj.data.attributes.get(name) is None:
            obj.data.attributes.new(name=name, type=row["type"], domain=row["domain"])
        for index, value in enumerate(row["values"]):
            obj.data.attributes[name].data[index].value = value
    after = read_mesh(obj)
    if not same(expected, after):
        raise ValueError("Native editor restoration changed other source fields")
    validate_transport(source, after, identities)
    return after, {"policy": "NATIVE_BMESH_EDITOR_SELECTION_RESTORE_V3", "before": before,
                   "before_sha256": digest(before), "after_sha256": digest(after),
                   "restored_attributes": restored,
                   "source_public_and_native_storage_verified": True,
                   "editor_selection_source_ids": {domain: maps[domain] for domain in ("POINT", "EDGE", "FACE")},
                   "derived_edge_selection": "actual native value retained; no source identity assigned"}


def clear_markers(data):
    for name in MARKERS.values():
        attr = data.attributes.get(name)
        if attr is not None:
            data.attributes.remove(attr)


def begin_source_capture(obj, ownership, recipe):
    """Place build-local identity only after source shaping and source blend save."""
    data = obj.data
    if data.shape_keys is not None:
        raise ValueError("Source capture expected pre-morph source mesh")
    if any(data.attributes.get(name) is not None for name in MARKERS.values()):
        raise ValueError("Reserved source-capture identity collision")
    capture = freeze_source(read_mesh(obj), ownership, recipe)
    created = []
    try:
        for domain, name in MARKERS.items():
            attr = data.attributes.new(name=name, type="INT", domain=domain)
            created.append(attr)
            for index, row in enumerate(attr.data):
                row.value = index
    except Exception:
        for attr in created:
            data.attributes.remove(attr)
        raise
    return capture


def finish_source_capture(obj, capture):
    """Capture actual saved native keys, then remove diagnostic IDs before export.

    A failure aborts the native job; it never falls back to recomputed/zero keys.
    No marker is retained in either the runtime blend or the GLB.
    """
    data = obj.data
    try:
        if "normal_stage_json" not in capture:
            raise ValueError("Actual isolated native normal stage is required")
        identities = read_identities(data)
        keys = data.shape_keys
        if keys is None or not keys.use_relative:
            raise ValueError("Actual relative six-key carrier missing")
        blocks = list(keys.key_blocks)
        if [block.name for block in blocks] != ["Basis", *KEYS]:
            raise ValueError("Actual six saved keys missing/changed")
        metadata = {}
        for block in blocks[1:]:
            if (block.relative_key.name != "Basis" or block.value != 0.0
                    or block.mute or block.vertex_group):
                raise ValueError("Unexpected saved key reference/value/mute")
            metadata[block.name] = {"relative_key": block.relative_key.name,
                                    "value": float(block.value), "mute": bool(block.mute),
                                    "interpolation": block.interpolation,
                                    "vertex_group": block.vertex_group,
                                    "slider_min": float(block.slider_min),
                                    "slider_max": float(block.slider_max)}
        shapes = {block.name: [list(point.co) for point in block.data] for block in blocks}
        packet = complete_capture(capture, read_mesh(obj), identities, shapes,
                                  [list(row.vector) for row in data.corner_normals])
        packet["shape_key_metadata"] = metadata
        packet["object_matrix_world"] = [list(row) for row in obj.matrix_world]
        packet["coordinate_frame"] = "native object local; explicit matrix_world retained"
        packet["packet_sha256"] = digest(packet)
        return packet
    finally:
        clear_markers(data)

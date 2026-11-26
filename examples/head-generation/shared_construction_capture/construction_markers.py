"""Temporary typed identity storage, with exact marker-only lifecycle checks."""
from construction_topology import digest

MARKERS = {"POINT": "_BeaverConstructionPoint", "FACE": "_BeaverConstructionFace", "CORNER": "_BeaverConstructionCorner"}
PROPERTIES = {"FLOAT": "value", "INT": "value", "INT8": "value", "BOOLEAN": "value", "STRING": "value",
              "FLOAT_VECTOR": "vector", "FLOAT2": "vector", "FLOAT_COLOR": "color", "BYTE_COLOR": "color",
              "INT32_2D": "value", "INT16_2D": "value", "QUATERNION": "value", "FLOAT4X4": "value"}


def plain(value):
    if type(value) in (str, int, float, bool):
        return value
    if isinstance(value, bytes):
        return {"bytes_hex": value.hex()}
    return [plain(item) for item in value]


def fields(obj):
    data = obj.data
    if data.shape_keys is not None:
        raise ValueError("Constructor identity lifecycle requires the pre-key mesh phase")
    attributes = {}
    for attr in data.attributes:
        if attr.name in MARKERS.values():
            continue
        if attr.data_type not in PROPERTIES:
            raise ValueError("Unsupported constructor native attribute: " + attr.name)
        attributes[attr.name] = {"domain": attr.domain, "type": attr.data_type,
            "values": [plain(getattr(row, PROPERTIES[attr.data_type])) for row in attr.data]}
    active_uv = getattr(data.uv_layers, "active", None)
    colors = getattr(data, "color_attributes", None)
    return {"attributes": attributes,
        "uv_settings": {layer.name: {key: getattr(layer, key) for key in ("active_render", "active_clone") if hasattr(layer, key)} for layer in data.uv_layers},
        "active_uv": None if active_uv is None else active_uv.name,
        "color_settings": {key: getattr(colors, key) for key in ("active_color_name", "default_color_name") if hasattr(colors, key)},
        "vertices": [[list(v.co), bool(v.select), bool(v.hide), [[g.group, g.weight] for g in v.groups]] for v in data.vertices],
        "edges": [[list(e.vertices), bool(e.use_seam), bool(e.use_edge_sharp), bool(e.select), bool(e.hide)] for e in data.edges],
        "faces": [[list(p.vertices), list(p.loop_indices), p.material_index, bool(p.use_smooth), bool(p.select), bool(p.hide)] for p in data.polygons],
        "corners": [[loop.vertex_index, loop.edge_index] for loop in data.loops],
        "corner_normals": [list(row.vector) for row in data.corner_normals],
        "materials": [m.name if m else None for m in data.materials],
        "groups": [[group.index, group.name] for group in obj.vertex_groups],
        "matrix_world": [list(row) for row in obj.matrix_world]}


def remove_markers(data):
    errors = []
    for name in MARKERS.values():
        try:
            attr = data.attributes.get(name)
            if attr is not None:
                data.attributes.remove(attr)
        except Exception as error:
            errors.append(error)
    if errors:
        raise RuntimeError("Failed to remove constructor marker storage") from errors[0]


def clear_verified(obj):
    failure = None
    try:
        before = digest(fields(obj))
    except Exception as error:
        failure = error
    # Removal is mandatory even when an unsupported/malformed field cannot be read.
    remove_markers(obj.data)
    if failure is not None:
        raise failure
    if digest(fields(obj)) != before:
        raise ValueError("Constructor marker cleanup changed native fields")


def attach(obj, values):
    data = obj.data
    counts = {"POINT": len(data.vertices), "FACE": len(data.polygons), "CORNER": len(data.loops)}
    if any(data.attributes.get(name) is not None for name in MARKERS.values()):
        raise ValueError("Constructor marker collision")
    if set(values) != set(MARKERS) or any(len(values[key]) != counts[key] for key in MARKERS):
        raise ValueError("Constructor marker coverage mismatch")
    if any(type(value) is not int or value < 1 for row in values.values() for value in row):
        raise ValueError("Constructor marker requires positive exact INT ordinals")
    before = digest(fields(obj))
    try:
        for domain, name in MARKERS.items():
            data.attributes.new(name=name, type="INT", domain=domain)
            for index, value in enumerate(values[domain]):
                data.attributes[name].data[index].value = value
        if digest(fields(obj)) != before:
            raise ValueError("Constructor marker creation changed native fields")
    except Exception:
        remove_markers(data)
        if digest(fields(obj)) != before:
            raise ValueError("Constructor marker rollback changed native fields")
        raise


def read(obj):
    data = obj.data
    counts = {"POINT": len(data.vertices), "FACE": len(data.polygons), "CORNER": len(data.loops)}
    result = {}
    for domain, name in MARKERS.items():
        attr = data.attributes.get(name)
        if (attr is None or (attr.domain, attr.data_type) != (domain, "INT")
                or len(attr.data) != counts[domain]):
            raise ValueError("Constructor marker schema changed")
        result[domain] = [int(row.value) for row in attr.data]
    return result

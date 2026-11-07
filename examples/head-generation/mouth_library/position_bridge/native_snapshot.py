"""Trusted native-state reader adapted from the verified editable-entry builder."""

import hashlib
import bpy
import numpy as np


def sha_array(a):
    return hashlib.sha256(np.ascontiguousarray(a).tobytes()).hexdigest()


def values(data, prop, width=1, dtype=np.float32):
    out = np.empty(len(data) * width, dtype=dtype)
    data.foreach_get(prop, out)
    return out.reshape(-1, width) if width > 1 else out


def attr_data(a):
    table = {
        "FLOAT": ("value", 1, np.float32),
        "INT": ("value", 1, np.int32),
        "BOOLEAN": ("value", 1, np.bool_),
        "FLOAT_VECTOR": ("vector", 3, np.float32),
        "FLOAT2": ("vector", 2, np.float32),
        "FLOAT_COLOR": ("color", 4, np.float32),
        "BYTE_COLOR": ("color", 4, np.float32),
        "INT32_2D": ("value", 2, np.int32),
    }
    prop, w, dt = table[a.data_type]
    return values(a.data, prop, w, dt)


def rna_value(v):
    if isinstance(v, (str, bool, int, float)) or v is None:
        return v
    if isinstance(v, bpy.types.ID):
        return {"id_type": type(v).__name__, "name": v.name}
    try:
        return [rna_value(i) for i in v]
    except TypeError:
        return str(v)


def material_snapshot(m):
    props = {}
    for p in m.bl_rna.properties:
        if p.type in {
            "BOOLEAN",
            "INT",
            "FLOAT",
            "STRING",
            "ENUM",
        } and p.identifier not in {"rna_type", "session_uid"}:
            try:
                props[p.identifier] = rna_value(getattr(m, p.identifier))
            except Exception:
                pass
    nodes = []
    links = []
    if m.node_tree:
        for n in m.node_tree.nodes:
            nprops = {}
            for p in n.bl_rna.properties:
                if p.type in {
                    "BOOLEAN",
                    "INT",
                    "FLOAT",
                    "STRING",
                    "ENUM",
                } and p.identifier not in {"rna_type", "session_uid"}:
                    try:
                        nprops[p.identifier] = rna_value(getattr(n, p.identifier))
                    except Exception:
                        pass
            ins = []
            outs = []
            for ss, des in [(n.inputs, ins), (n.outputs, outs)]:
                for s in ss:
                    des.append(
                        {
                            "identifier": s.identifier,
                            "name": s.name,
                            "type": s.type,
                            "default": (
                                rna_value(s.default_value)
                                if hasattr(s, "default_value")
                                else None
                            ),
                        }
                    )
            nodes.append(
                {
                    "name": n.name,
                    "type": n.bl_idname,
                    "props": nprops,
                    "image": n.image.name if hasattr(n, "image") and n.image else None,
                    "inputs": ins,
                    "outputs": outs,
                }
            )
        links = sorted(
            [
                [
                    l.from_node.name,
                    l.from_socket.identifier,
                    l.to_node.name,
                    l.to_socket.identifier,
                ]
                for l in m.node_tree.links
            ]
        )
    return {"name": m.name, "props": props, "nodes": nodes, "links": links}


def snapshot(obj, stem):
    m = obj.data
    arrays = {}
    arrays["position"] = values(m.vertices, "co", 3)
    arrays["edges"] = values(m.edges, "vertices", 2, np.int32)
    arrays["loop_vertex"] = values(m.loops, "vertex_index", 1, np.int32)
    arrays["loop_edge"] = values(m.loops, "edge_index", 1, np.int32)
    arrays["polygon_loop_start"] = values(m.polygons, "loop_start", 1, np.int32)
    arrays["polygon_loop_total"] = values(m.polygons, "loop_total", 1, np.int32)
    arrays["triangles"] = np.asarray(
        [list(p.vertices) for p in m.polygons], dtype=np.int32
    )
    arrays["polygon_material"] = values(m.polygons, "material_index", 1, np.int32)
    arrays["polygon_smooth"] = values(m.polygons, "use_smooth", 1, np.bool_)
    arrays["corner_normal"] = values(m.corner_normals, "vector", 3)
    arrays["vertex_normal"] = values(m.vertices, "normal", 3)
    for i, k in enumerate(m.shape_keys.key_blocks):
        arrays["shape_" + str(i)] = values(k.data, "co", 3)
    for i, u in enumerate(m.uv_layers):
        arrays["uv_" + str(i)] = values(u.data, "uv", 2)
    for i, c in enumerate(m.color_attributes):
        arrays["color_" + c.name] = values(c.data, "color", 4)
    for i, a in enumerate(m.attributes):
        arrays["attribute_" + a.name] = attr_data(a)
    for i, im in enumerate(
        sorted(
            (
                i
                for i in bpy.data.images
                if i.type != "RENDER_RESULT" and i.source != "VIEWER"
            ),
            key=lambda i: i.name,
        )
    ):
        arrays["image_pixels_" + str(i)] = np.asarray(im.pixels[:], dtype=np.float32)
    groups = []
    for vg in obj.vertex_groups:
        members = []
        for v in m.vertices:
            for membership in v.groups:
                if membership.group == vg.index:
                    members.append([v.index, membership.weight])
        groups.append({"name": vg.name, "members": members})
    info = {
        "object": obj.name,
        "mesh": m.name,
        "matrix_world": [list(row) for row in obj.matrix_world],
        "counts": {
            "vertices": len(m.vertices),
            "edges": len(m.edges),
            "triangles": len(m.polygons),
            "loops": len(m.loops),
        },
        "has_custom_normals": m.has_custom_normals,
        "shapes": [
            {
                "name": k.name,
                "value": k.value,
                "relative_key": k.relative_key.name,
                "mute": k.mute,
                "slider_min": k.slider_min,
                "slider_max": k.slider_max,
                "vertex_group": k.vertex_group,
            }
            for k in m.shape_keys.key_blocks
        ],
        "uv_layers": [
            {"name": u.name, "active_render": u.active_render} for u in m.uv_layers
        ],
        "color_attributes": [
            {"name": a.name, "domain": a.domain, "data_type": a.data_type}
            for a in m.color_attributes
        ],
        "attributes": [
            {"name": a.name, "domain": a.domain, "data_type": a.data_type}
            for a in m.attributes
        ],
        "materials": [material_snapshot(m) for m in obj.data.materials],
        "images": [
            {
                "name": i.name,
                "size": list(i.size),
                "source": i.source,
                "colorspace": i.colorspace_settings.name,
                "alpha_mode": i.alpha_mode,
                "packed": bool(i.packed_file),
                "packed_sha256": (
                    hashlib.sha256(i.packed_file.data).hexdigest()
                    if i.packed_file
                    else None
                ),
            }
            for i in sorted(bpy.data.images, key=lambda i: i.name)
            if i.type != "RENDER_RESULT" and i.source != "VIEWER"
        ],
        "active_color_name": m.color_attributes.active_color.name,
        "render_color_name": m.color_attributes[
            m.color_attributes.render_color_index
        ].name,
        "object_custom_properties": {k: rna_value(v) for k, v in obj.items()},
        "groups": groups,
        "modifiers": [x.type for x in obj.modifiers],
        "drivers": sum(
            len(x.animation_data.drivers) if x.animation_data else 0
            for x in [obj, m, m.shape_keys]
        ),
        "text_blocks": len(bpy.data.texts),
        "arrays": {
            k: {"shape": list(v.shape), "dtype": str(v.dtype), "sha256": sha_array(v)}
            for k, v in arrays.items()
        },
    }

    return info, arrays

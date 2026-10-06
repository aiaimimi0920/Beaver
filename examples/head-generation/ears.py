"""Shallow open-backed pinna membrane sharing evaluated face-root positions."""

from face import *


def build_ears():
    # Deferred until the facial subdivision is evaluated, so root positions match.
    return None


def membrane_offset(t, u, config, root_depth):
    envelope = sin(pi * t) * (1 + config["ear_upper_fullness"] * (2 * t - 1))
    outward = envelope * (config["ear_outward_span"] * u - config["ear_cup_recess"] * 4 * u * (1 - u))
    outer_depth = (
        config["ear_root_depth_base"]
        + config["ear_root_depth_slope"] * t
        - config["ear_posterior_span"] * envelope
    )
    sweep = u + config["ear_bend_contrast"] * sin(2 * pi * u) / (2 * pi)
    posterior = (root_depth - outer_depth) * sweep
    return outward, posterior


def ear_arc_height(t, low, high):
    return (
        low + (high - low) * (1 - cos(pi * t)) / 2 + H * C["ear_upper_arc_bias"] * t * sin(pi * t)
    )


def project_ear_uv(vertices):
    depths = [p[1] for p in vertices]
    heights = [p[2] for p in vertices]
    low_d, high_d = min(depths), max(depths)
    low_h, high_h = min(heights), max(heights)
    assert high_d - low_d > 1e-8 and high_h - low_h > 1e-8
    return [
        (
            0.02 + 0.96 * (p[1] - low_d) / (high_d - low_d),
            0.02 + 0.96 * (p[2] - low_h) / (high_h - low_h),
        )
        for p in vertices
    ]


def fit_ear_roots(shell):
    bpy.context.view_layer.update()
    evaluated = shell.evaluated_get(bpy.context.evaluated_depsgraph_get())
    data = evaluated.to_mesh()
    reports = []
    try:
        uses = {}
        for poly in data.polygons:
            ids = list(poly.vertices)
            for a, b in zip(ids, ids[1:] + ids[:1]):
                key = tuple(sorted((a, b)))
                uses[key] = uses.get(key, 0) + 1
        boundary_edges = {e for e, n in uses.items() if n == 1}
        boundary_ids = {v for e in boundary_edges for v in e}
        for side in [-1, 1]:
            roots = [
                i
                for i in boundary_ids
                if side * data.vertices[i].co.x > 0
                and C["ear_root_min_height"]
                <= (data.vertices[i].co.z - Y0) / H
                <= C["ear_root_max_height"]
                and abs(data.vertices[i].co.x) > width(data.vertices[i].co.z) - 0.015 * H
            ]
            roots.sort(key=lambda i: data.vertices[i].co.z)
            assert len(roots) >= 5, "Insufficient ear-root boundary samples"
            assert all(
                tuple(sorted((a, b))) in boundary_edges for a, b in zip(roots, roots[1:])
            ), "Ear root must be a connected facial boundary chain"
            root_points = [data.vertices[i].co.copy() for i in roots]
            low, high = root_points[0].z, root_points[-1].z
            vs, fs, rows, uvpoints = [], [], [], []
            root_indices = []
            for j, root in enumerate(root_points):
                t = (root.z - low) / (high - low)
                row = []
                values = [0] if j in [0, len(root_points) - 1] else [0, 0.25, 0.5, 0.78, 1]
                for u in values:
                    outward, posterior = membrane_offset(t, u, C, (-root.y - Z0) / H)
                    row.append(len(vs))
                    lift = (ear_arc_height(t, low, high) - root.z) * sqrt(max(0, sin(pi * u / 2)))
                    vs.append((root.x + side * H * outward, root.y + H * posterior, root.z + lift))
                    uvpoints.append((0.02 + 0.96 * u, 0.02 + 0.96 * t))
                root_indices.append(row[0])
                rows.append(row)
            for a, b in zip(rows, rows[1:]):
                if len(a) == 1:
                    fs.extend((a[0], b[k + 1], b[k]) for k in range(len(b) - 1))
                elif len(b) == 1:
                    fs.extend((a[k], a[k + 1], b[0]) for k in range(len(a) - 1))
                else:
                    fs.extend((a[k], a[k + 1], b[k + 1], b[k]) for k in range(len(a) - 1))
            main_face_count = len(fs)
            # A narrow skin return lip supports back-side visibility without
            # closing the ear basin or creating an independent dark stroke.
            rim = []
            for j, row in enumerate(rows):
                outer = row[-1]
                if j in [0, len(rows) - 1]:
                    rim.append(outer)
                    continue
                t = (root_points[j].z - low) / (high - low)
                x, y, z = vs[outer]
                rim.append(len(vs))
                vs.append(
                    (
                        x - side * H * C["ear_return_lip_width"] * sin(pi * t),
                        y + H * C["ear_return_lip_depth"] * sin(pi * t),
                        z,
                    )
                )
                uvpoints.append((0.86, 0.02 + 0.96 * t))
            for j in range(len(rows) - 1):
                corners = [rows[j][-1], rim[j], rim[j + 1], rows[j + 1][-1]]
                clean = list(dict.fromkeys(corners))
                if len(clean) >= 3:
                    fs.append(tuple(clean))
            uvpoints = project_ear_uv(vs)
            if side < 0:
                fs = [tuple(reversed(f)) for f in fs]
            # A single wrapped surface: the inward return has a free inner edge.
            # Do not duplicate the whole front sheet into a nearly coincident back.
            front_face_count = len(fs)
            obj = mesh("Ear hollow pinna " + str(side), vs, fs, SKIN)
            assert all(poly.area > 1e-12 for poly in obj.data.polygons), "Degenerate ear membrane"
            obj["ear_detail"] = True
            obj["ear_main_faces"] = main_face_count
            obj["ear_front_faces"] = front_face_count
            detail = obj.data.uv_layers.new(name="DetailUV")
            for loop in obj.data.loops:
                detail.data[loop.index].uv = uvpoints[loop.vertex_index]
            delta = obj.data.attributes.new("MouthOpenDelta", "FLOAT_VECTOR", "POINT")
            for value in delta.data:
                value.vector = (0, 0, 0)
            error = max(
                (obj.data.vertices[i].co - r).length for i, r in zip(root_indices, root_points)
            )
            assert error < 1e-7, "Ear-root seam must match actual evaluated skin"
            reports.append(
                {
                    "name": obj.name,
                    "root_vertices": len(roots),
                    "max_root_position_error_m": error,
                    "root_positions": [list(r) for r in root_points],
                    "geometry": "open-backed shallow membrane; no separate closed back shell",
                }
            )
    finally:
        evaluated.to_mesh_clear()
    return reports


def verify_ear_seams(data, reports):
    from mathutils.kdtree import KDTree

    tree = KDTree(len(data.vertices))
    for vertex in data.vertices:
        tree.insert(vertex.co, vertex.index)
    tree.balance()
    uses = {}
    for poly in data.polygons:
        ids = list(poly.vertices)
        for a, b in zip(ids, ids[1:] + ids[:1]):
            key = tuple(sorted((a, b)))
            uses[key] = uses.get(key, 0) + 1
    out = []
    for report in reports:
        ids = []
        for x, y, z in report["root_positions"]:
            _, index, distance = tree.find(Vector((x, y, z - 0.021)))
            assert distance < 0.000002, "Missing joined ear-root vertex"
            ids.append(index)
        counts = [uses.get(tuple(sorted((a, b))), 0) for a, b in zip(ids, ids[1:])]
        assert all(c == 2 for c in counts), "Ear-root edges must connect exactly two surfaces"
        out.append(
            {
                "name": report["name"],
                "shared_manifold_root_edges": len(counts),
                "edge_face_usage": sorted(set(counts)),
            }
        )
    return out

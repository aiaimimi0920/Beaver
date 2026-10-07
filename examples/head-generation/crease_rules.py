"""Semantic edge crease groups and evaluated-geometry ablation."""


def regional_weight(v, points):
    if v < points[0][0] or v > points[-1][0]:
        return 0.0
    for (a, wa), (b, wb) in zip(points, points[1:]):
        if v <= b:
            t = (v - a) / (b - a)
            return wa * (1 - t) + wb * t
    return points[-1][1]


def semantic_weight(a, b, wa, wb, height, origin, config):
    if abs(a[2] - b[2]) < 0.002 * height:
        return 0, 0.0
    v = ((a[2] + b[2]) / 2 - origin) / height
    if max(abs(a[0]), abs(b[0])) < 1e-7:
        return 1, regional_weight(v, config["face_center_crease_profile"])
    target = config["face_side_crease_lateral_ratio"]
    tolerance = config["face_side_crease_selection_tolerance"]
    if abs(abs(a[0]) / wa - target) <= tolerance and abs(abs(b[0]) / wb - target) <= tolerance:
        return 2, regional_weight(v, config["face_side_crease_profile"])
    return 0, 0.0


def assign_creases(shell, front_count, height, origin, width_fn, config):
    uses = {tuple(sorted(e.vertices)): 0 for e in shell.data.edges}
    front_edges = set()
    for poly in shell.data.polygons:
        ids = list(poly.vertices)
        for a, b in zip(ids, ids[1:] + ids[:1]):
            key = tuple(sorted((a, b)))
            uses[key] += 1
            if poly.index < front_count:
                front_edges.add(key)
    crease = shell.data.attributes.new("crease_edge", "FLOAT", "EDGE")
    groups = shell.data.attributes.new("FacialCreaseGroup", "INT", "EDGE")
    selected = []
    counts = {"center": 0, "side": 0, "aperture": 0}
    rear_ids = set(shell.get("rear_return_outer_vertex_ids", []))
    for edge in shell.data.edges:
        key = tuple(sorted(edge.vertices))
        a, b = [shell.data.vertices[i].co for i in edge.vertices]
        x, y = (a.x + b.x) / 2, (a.z + b.z) / 2
        aperture = uses[key] == 1 and (
            (0.018 < x < 0.087 and 1.610 < y < 1.671)
            or (
                x > 0.00001
                and x < 0.021
                and abs(y - origin - height * config["mouth_height"]) < 0.002
            )
        )
        if set(key).issubset(rear_ids):
            crease.data[edge.index].value = config["rear_return_rim_crease"]
            groups.data[edge.index].value = 4
        elif aperture:
            crease.data[edge.index].value = 1.0
            groups.data[edge.index].value = 3
            counts["aperture"] += 1
        elif key in front_edges:
            role, value = semantic_weight(
                a, b, width_fn(a.z), width_fn(b.z), height, origin, config
            )
            if value > 0:
                crease.data[edge.index].value = value
                groups.data[edge.index].value = role
                selected.append((edge.index, value))
                counts["center" if role == 1 else "side"] += 1
    assert counts["center"] > 0 and counts["side"] > 0, "Missing semantic crease group"
    return selected, counts


def measure_crease_effect(shell, selected):
    import bpy

    def evaluated_positions():
        shell.data.update()
        bpy.context.view_layer.update()
        graph = bpy.context.evaluated_depsgraph_get()
        graph.update()
        evaluated = shell.evaluated_get(graph)
        data = evaluated.to_mesh()
        try:
            return [tuple(v.co) for v in data.vertices]
        finally:
            evaluated.to_mesh_clear()

    crease = shell.data.attributes["crease_edge"]
    active = evaluated_positions()
    try:
        for index, _ in selected:
            crease.data[index].value = 0
        plain = evaluated_positions()
    finally:
        for index, value in selected:
            crease.data[index].value = value
        shell.data.update()
        bpy.context.view_layer.update()
    assert len(active) == len(plain), "Crease ablation changed topology"
    distances = [sum((a - b) ** 2 for a, b in zip(p, q)) ** 0.5 for p, q in zip(active, plain)]
    maximum = max(distances, default=0)
    assert maximum > 1e-7, "Stored crease weights had no evaluated geometry effect"
    return {
        "compared_vertices": len(active),
        "changed_vertices": sum(d > 1e-7 for d in distances),
        "maximum_displacement_m": maximum,
        "mean_displacement_m": sum(distances) / len(distances),
    }

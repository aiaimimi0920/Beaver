"""Parameterized expanding orbital pocket and independent concave iris."""

from face import *


def iris_depth(x, y):
    dx = abs(x) / H - C["iris_center_x"]
    dy = (y - Y0) / H - C["iris_center_y"]
    r2 = (dx / C["iris_half_width"]) ** 2 + (dy / C["iris_half_height"]) ** 2
    return Z0 + H * (
        C["iris_rim_depth"] + C["iris_plane_slope"] * dx - C["iris_concavity"] * max(0, 1 - r2)
    )


def iris_grid_point(u, v):
    return (u * sqrt(max(0, 1 - v * v / 2)), v * sqrt(max(0, 1 - u * u / 2)))


def build_socket(side):
    n = 48
    vs = []
    fs = []
    cx = side * H * C["eye_center_x"]
    cy = Y0 + H * C["eye_corner_y"]
    sections = [
        (1, 1, 0),
        (1.07, 1.15, 0.03),
        (C["pocket_width_ratio"], C["pocket_height_ratio"], C["pocket_back"]),
        (
            C["pocket_back_width_ratio"],
            C["pocket_back_height_ratio"],
            C["pocket_back"] + C["pocket_back_extra"],
        ),
    ]
    for ring_index, (sx, sy, back) in enumerate(sections):
        for k in range(n):
            a = 2 * pi * k / n
            x, y = eye_contour(side, cos(a), sin(a) >= 0)
            xx = cx + (x - cx) * sx
            yy = cy + (y - cy) * sy
            rim = rim_depth(cx, cy) if ring_index == len(sections) - 1 else rim_depth(x, y)
            vs.append(coord(xx, yy, rim - back * H - 0.0001))
    for j in range(len(sections) - 1):
        for k in range(n):
            q = (k + 1) % n
            f = (j * n + k, j * n + q, (j + 1) * n + q, (j + 1) * n + k)
            fs.append(f if side > 0 else tuple(reversed(f)))
    last = (len(sections) - 1) * n
    cap = tuple(last + k for k in range(n))
    fs.append(cap if side > 0 else tuple(reversed(cap)))
    obj = mesh("Expanding recessed eye pocket " + str(side), vs, fs, WHITE)
    obj["ocular_surface"] = True
    obj.data.polygons[-1].use_smooth = False
    obj["rear_cap_flat_shading"] = True
    obj["pocket_back_start"] = last
    obj["pocket_back_count"] = n
    obj["rear_cap_planarity_before_m"] = max(vs[i][1] for i in cap) - min(vs[i][1] for i in cap)
    assert obj["rear_cap_planarity_before_m"] < 1e-8
    assert all(p.area > 1e-12 for p in obj.data.polygons), "Degenerate pocket face"
    iv = []
    rows = cols = C["iris_grid_resolution"]
    for j in range(rows):
        sy = -1 + 2 * j / (rows - 1)
        for i in range(cols):
            tx = -1 + 2 * i / (cols - 1)
            u, v = iris_grid_point(tx, sy)
            x = side * H * C["iris_center_x"] + H * C["iris_half_width"] * u
            y = Y0 + H * (C["iris_center_y"] + C["iris_half_height"] * v)
            iv.append(coord(x, y, iris_depth(x, y)))
    iris = mesh("Independent concave iris " + str(side), iv, grid_faces(rows, cols), WHITE)
    iris["iris_detail"] = True
    iris["ocular_surface"] = True
    uv = iris.data.uv_layers.new(name="DetailUV")
    for loop in iris.data.loops:
        v = iris.data.vertices[loop.vertex_index].co
        uv.data[loop.index].uv = (
            (v.x - side * H * C["iris_center_x"]) / (2 * STYLE["eye_half_width_m"]) + 0.5,
            (v.z - Y0 - H * C["iris_center_y"]) / 0.044 + 0.5,
        )


def fit_pocket_clearance(shell):
    """Keep hidden walls inside the evaluated skin, including outline clearance."""
    from mathutils.bvhtree import BVHTree

    bpy.context.view_layer.update()
    graph = bpy.context.evaluated_depsgraph_get()
    evaluated = shell.evaluated_get(graph)
    data = evaluated.to_mesh()
    margin = H * C["pocket_skin_clearance"]
    count = 0
    max_move = 0.0
    minimum = H
    try:
        tree = BVHTree.FromPolygons(
            [v.co.copy() for v in data.vertices], [tuple(p.vertices) for p in data.polygons]
        )
        for obj in PARTS:
            if not obj.name.startswith("Expanding recessed eye pocket"):
                continue
            for vertex in obj.data.vertices:
                if vertex.index < 48:
                    continue
                x, y = (abs(vertex.co.x), vertex.co.z)
                t = (x / H - C["eye_center_x"]) / C["eye_half_width"]
                cy = Y0 + H * (C["eye_corner_y"] + C["eye_corner_slope"] * t)
                arch = C["eye_upper_arch"] if y >= cy else C["eye_lower_arch"]
                power = C["eye_upper_power"] if y >= cy else C["eye_lower_power"]
                radius = sqrt(t * t + abs((y - cy) / (H * arch)) ** (1 / power))
                if radius <= 1.01:
                    continue
                old = vertex.co.copy()
                for _ in range(3):
                    point, normal, _, distance = tree.find_nearest(vertex.co)
                    if point is None:
                        break
                    signed = (vertex.co - point).dot(normal)
                    if signed <= -margin:
                        break
                    vertex.co -= normal * (signed + margin)
                moved = (vertex.co - old).length
                if moved > 1e-07:
                    count += 1
                    max_move = max(max_move, moved)
                point, normal, _, distance = tree.find_nearest(vertex.co)
                if point is not None:
                    minimum = min(minimum, distance)
            obj.data.update()
            start = obj["pocket_back_start"]
            cap_vertices = list(obj.data.vertices)[start : start + obj["pocket_back_count"]]
            obj["rear_cap_planarity_after_m"] = max(v.co.y for v in cap_vertices) - min(
                v.co.y for v in cap_vertices
            )
            assert all(p.area > 1e-12 for p in obj.data.polygons), "Degenerate fitted pocket"
        assert max_move < 0.04 * H, "Pocket clearance needs a structural redesign"
    finally:
        evaluated.to_mesh_clear()
    return {
        "adjusted_vertices": count,
        "maximum_move_m": max_move,
        "requested_margin_m": margin,
        "minimum_sampled_distance_m": minimum,
    }

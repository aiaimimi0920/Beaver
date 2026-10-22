"""Native original nose attachments; no third-party geometry is embedded."""

NOSE_MESH_REPORT = []
NOSE_BACK_CAPS = []


def orient_and_check(vertices, faces):
    from mathutils import Vector

    points = [Vector(v) for v in vertices]
    edges = {}
    directed = {}
    for face in faces:
        a, b, c = (points[i] for i in face)
        if (b - a).cross(c - a).length < 1e-12:
            raise ValueError("Degenerate nose attachment triangle")
        for i, j in zip(face, face[1:] + face[:1]):
            edge = tuple(sorted((i, j)))
            edges[edge] = edges.get(edge, 0) + 1
            directed[(i, j)] = directed.get((i, j), 0) + 1
    if any(v != 2 for v in edges.values()) or any(
        directed.get((b, a), 0) != 1 or count != 1 for (a, b), count in directed.items()
    ):
        raise ValueError("Nose attachment must be closed two-manifold")
    origin = points[0]
    volume = (
        sum(
            (points[a] - origin).dot((points[b] - origin).cross(points[c] - origin))
            for a, b, c in faces
        )
        / 6
    )
    if abs(volume) < 1e-13:
        raise ValueError("Nose attachment has no usable volume")
    return ([tuple(reversed(f)) for f in faces] if volume < 0 else faces), abs(volume)


def paired_caps(front, back, triangles):
    count = len(front)
    faces = list(triangles) + [tuple(i + count for i in t[::-1]) for t in triangles]
    boundary = {}
    for t in triangles:
        for a, b in zip(t, t[1:] + t[:1]):
            boundary.setdefault(tuple(sorted((a, b))), []).append((a, b))
    for pairs in boundary.values():
        if len(pairs) == 1:
            a, b = pairs[0]
            faces.extend([(a, a + count, b + count), (a, b + count, b)])
    vertices = front + back
    faces, volume = orient_and_check(vertices, faces)
    return vertices, faces, volume


def pad_geometry(d, support=None):
    from math import sin, cos, pi, sqrt
    from face import H, Y0, depth, coord

    support = depth if support is None else support

    xy = [(0.0, Y0 + H * d["tip_height"])]
    radii = [0.0]
    n, rings = 32, 4
    for ring in range(1, rings + 1):
        r = ring / rings
        for i in range(n):
            a = 2 * pi * i / n
            xy.append(
                (
                    H * d["pad_width"] * r * cos(a) * (1 + 0.12 * sin(a)),
                    Y0 + H * (d["tip_height"] + d["pad_height"] * r * sin(a)),
                )
            )
            radii.append(r)
    triangles = [(0, 1 + i, 1 + (i + 1) % n) for i in range(n)]
    for ring in range(rings - 1):
        start = 1 + ring * n
        for i in range(n):
            a, b = start + i, start + (i + 1) % n
            c, e = a + n, b + n
            triangles.extend([(a, c, e), (a, e, b)])
    front = [
        coord(
            x,
            y,
            support(x, y) + H * d["pad_projection"] * sqrt(max(0, 1 - r * r)) - 0.0002,
        )
        for (x, y), r in zip(xy, radii)
    ]
    back = [coord(x, y, support(x, y) - 0.001) for x, y in xy]
    return paired_caps(front, back, triangles)


def faceted_geometry(d, support=None):
    from face import H, Y0, depth, coord

    support = depth if support is None else support

    # Five cross-ridges create genuine planar, flat-shaded triangles.
    hs = [
        0.205,
        0.245,
        d["tip_height"],
        (d["tip_height"] + d["root_height"]) / 2,
        d["root_height"],
    ]
    width = [0.55, 0.85, 1.0, 0.68, 0.38]
    relief = [0.04, 0.40, 1.0, 0.48, 0.025]
    front = []
    back = []
    for h, w, z in zip(hs, width, relief):
        for t in (-1.0, 0.0, 1.0):
            x = H * d["half_width"] * w * t
            y = Y0 + H * h
            offset = H * d["projection"] * z * (1 - abs(t) * 0.92) - 0.0002
            front.append(coord(x, y, support(x, y) + offset))
            back.append(coord(x, y, support(x, y) - 0.001))
    triangles = []
    for row in range(4):
        for col in range(2):
            a = row * 3 + col
            b = a + 1
            c = a + 3
            e = c + 1
            triangles.extend([(a, c, e), (a, e, b)])
    return paired_caps(front, back, triangles)


def evaluated_skin_support(shell):
    """Use the evaluated neutral shell rather than assuming analytic tessellation parity."""
    import bpy
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree

    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = shell.evaluated_get(depsgraph)
    surface = evaluated.to_mesh()
    try:
        points = [evaluated.matrix_world @ v.co for v in surface.vertices]
        polygons = [tuple(p.vertices) for p in surface.polygons]
        tree = BVHTree.FromPolygons(points, polygons, all_triangles=False)
    finally:
        evaluated.to_mesh_clear()

    def support(x, y):
        hit, normal, index, distance = tree.ray_cast(
            Vector((x, -1.0, y)), Vector((0, 1, 0)), 2.0
        )
        if hit is None:
            raise ValueError(
                "No evaluated facial shell under researched nose attachment"
            )
        return -hit.y

    return support


def build_attachment(design, shell):
    from face import mesh, STYLE
    from mouth import store_mouth_deltas

    if design["kind"] == "fused":
        return
    support = evaluated_skin_support(shell)
    if design["kind"] == "pad-muzzle":
        name = "Researched nasal pad"
        color = (0.18, 0.13, 0.13)
        vertices, faces, volume = pad_geometry(design, support)
    else:
        name = "Researched faceted nasal module"
        color = tuple(v * 0.92 for v in STYLE["skin_rgb"])
        vertices, faces, volume = faceted_geometry(design, support)
    NOSE_BACK_CAPS.append({"name": name, "points": vertices[len(vertices) // 2 :]})
    obj = mesh(name, vertices, faces, color)
    if design["kind"] == "faceted-candidate":
        for poly in obj.data.polygons:
            poly.use_smooth = False
    store_mouth_deltas(obj)
    NOSE_MESH_REPORT.append(
        {
            "name": name,
            "kind": design["kind"],
            "vertices": len(vertices),
            "triangles": len(faces),
            "signed_volume_m3": volume,
            "closed_two_manifold": True,
            "flat_faces": sum(not p.use_smooth for p in obj.data.polygons),
            "attachment_policy": (
                "Back cap embedded1mm; pad perimeter embedded0.2mm"
                if design["kind"] == "pad-muzzle"
                else "Back cap embedded1mm; exposed peripheral facet ridges joined by closed sidewalls"
            ),
            "topology_policy": "Separate closed component, not boolean union",
            "support_source": "Evaluated neutral facial shell before final regional fairing",
            "scope": "Original authored approximation, not reconstructed reference mesh; final surface attachment and poses require native view checks",
        }
    )


def validate_final_attachments(data):
    """Sample actual triangulated skin, not merely the analytic support field."""
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree

    regions = data.attributes["NormalRegion"]
    triangles = [
        tuple(p.vertices) for p in data.polygons if regions.data[p.index].value == 1
    ]
    tree = BVHTree.FromPolygons(
        [v.co.copy() for v in data.vertices], triangles, all_triangles=True
    )
    results = []
    for entry in NOSE_BACK_CAPS:
        distances = []
        for x, y, z in entry["points"]:
            # Match the established head assembly translation.
            origin = Vector((x, -1.0, z - 0.021))
            hit, normal, index, distance = tree.ray_cast(origin, Vector((0, 1, 0)), 2.0)
            if hit is None:
                raise ValueError("Nose back-cap sample has no underlying skin")
            distances.append(y - hit.y)
        minimum, maximum = min(distances), max(distances)
        if minimum < -0.00001 or maximum > 0.008:
            raise ValueError(
                f"Nose attachment back-cap not seated in final skin: {minimum}, {maximum}"
            )
        results.append(
            {
                "name": entry["name"],
                "samples": len(distances),
                "min_back_cap_embedding_m": minimum,
                "max_back_cap_embedding_m": maximum,
                "target_region": 1,
                "scope": "Back-cap vertices against final triangulated front skin; not continuous global intersection or posed acceptance",
            }
        )
    return results

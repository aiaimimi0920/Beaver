"""Packaged native runtime; exactly one closed-mesh brow object per side."""

BROW_GEOMETRY_REPORT = []


def brow_refine(points, triangles):
    points, cache, result = list(points), {}, []
    def midpoint(a, b):
        key = tuple(sorted((a, b)))
        if key not in cache:
            cache[key] = len(points)
            points.append(tuple((points[a][k] + points[b][k]) / 2 for k in (0, 1)))
        return cache[key]
    for a, b, c in triangles:
        ab, bc, ca = midpoint(a, b), midpoint(b, c), midpoint(c, a)
        result.extend([(a, ab, ca), (ab, b, bc), (ca, bc, c), (ab, bc, ca)])
    return points, result


def brow_surface_points(points, side, design):
    from face import H, C, Y0, depth, coord
    span = H * (C["brow_outer_x"] - C["brow_inner_x"])
    vertices = []
    for layer in (0, 1):
        for local_x, local_y in points:
            x = side * (H * C["brow_inner_x"] + local_x)
            y = Y0 + H * (C["brow_height"] + C["brow_rise"] * local_x / span) + local_y
            z = depth(x, y) + design["clearance_m"] - layer * design["thickness_m"]
            vertices.append(coord(x, y, z))
    return vertices


def brow_sample_clearance(vertices, triangles):
    from face import depth
    count = len(vertices) // 2
    samples = list(vertices[count:])
    for tri in triangles:
        p = [vertices[i + count] for i in tri]
        samples.append(tuple(sum(v[k] for v in p) / 3 for k in range(3)))
        samples.extend(tuple((p[a][k] + p[b][k]) / 2 for k in range(3)) for a, b in ((0, 1), (1, 2), (2, 0)))
    return min(-v[1] - depth(v[0], v[2]) for v in samples), len(samples)


def build_designed_brow(side, design):
    from mathutils import Vector
    from mathutils.geometry import tessellate_polygon
    from face import H, C, STYLE, mesh

    span = H * (C["brow_outer_x"] - C["brow_inner_x"])
    polygons = brow_polygons(design, span, H * C["brow_thickness"])
    vertices, faces = [], []
    areas, refinements, clearances, sample_counts = [], [], [], []
    for polygon in polygons:
        vectors = [Vector((x, y, 0)) for x, y in polygon]
        triangles = [tuple(tri) for tri in tessellate_polygon([vectors])]
        if any(len(tri) != 3 or any(type(i) is not int or not 0 <= i < len(polygon) for i in tri) for tri in triangles):
            raise ValueError("Unexpected Blender tessellation index contract")
        if len(triangles) != len(polygon) - 2:
            raise ValueError("Incomplete brow triangulation")
        triangles = [tri if brow_cross(*(polygon[i] for i in tri)) > 0 else tri[::-1] for tri in triangles]
        area = sum(abs(brow_cross(*(polygon[i] for i in tri))) / 2 for tri in triangles)
        if abs(area - brow_area(polygon)) > max(1e-10, area * 1e-5):
            raise ValueError("Brow triangulation coverage mismatch")
        for refinement in range(4):
            local_vertices = brow_surface_points(polygon, side, design)
            clearance, samples = brow_sample_clearance(local_vertices, triangles)
            if clearance >= max(.00005, (design["clearance_m"] - design["thickness_m"]) * .25):
                break
            if refinement == 3:
                raise ValueError("Brow sampled skin clearance remains insufficient")
            polygon, triangles = brow_refine(polygon, triangles)
        refinements.append(refinement)
        clearances.append(clearance)
        sample_counts.append(samples)
        start, count = len(vertices), len(polygon)
        vertices.extend(local_vertices)
        local_faces = triangles + [tuple(i + count for i in tri[::-1]) for tri in triangles]
        boundary = {}
        for tri in triangles:
            for i, j in zip(tri, tri[1:] + tri[:1]):
                key = tuple(sorted((i, j)))
                boundary.setdefault(key, []).append((i, j))
        for occurrences in boundary.values():
            if len(occurrences) == 1:
                i, j = occurrences[0]
                local_faces.extend([(i, i + count, j + count), (i, j + count, j)])
        faces.extend(tuple(start + i for i in (tri if side > 0 else tri[::-1])) for tri in local_faces)
        areas.append(area)
    points = [Vector(v) for v in vertices]
    if any(
        (points[b] - points[a]).cross(points[c] - points[a]).length < 1e-12
        for a, b, c in faces
    ):
        raise ValueError("Degenerate native brow triangle")
    edges = {}
    for tri in faces:
        for a, b in zip(tri, tri[1:] + tri[:1]):
            edge = tuple(sorted((a, b)))
            edges[edge] = edges.get(edge, 0) + 1
    if any(count != 2 for count in edges.values()):
        raise ValueError("Brow must have closed two-manifold islands")
    origin = points[0]
    volume = sum(
        (points[a] - origin).dot((points[b] - origin).cross(points[c] - origin))
        for a, b, c in faces
    ) / 6
    if volume <= 1e-13:
        raise ValueError("Invalid outward brow orientation or volume")
    obj = mesh("Tapered brow " + str(side), vertices, faces, tuple(STYLE["brow_rgb"]))
    obj["brow_design_kind"] = design["kind"]
    BROW_GEOMETRY_REPORT.append({
        "side": side,
        "kind": design["kind"],
        "vertices": len(vertices),
        "triangles": len(faces),
        "islands": len(polygons),
        "local_areas_m2": areas,
        "signed_volume_m3": volume,
        "manifold_edges": True,
        "requested_vertex_depth_clearance_m": (
            design["clearance_m"] - design["thickness_m"]
        ),
        "sampled_min_depth_clearance_m": min(clearances),
        "clearance_samples": sum(sample_counts),
        "cap_refinement_rounds": refinements,
        "clearance_scope": (
            "back-cap vertices, edge midpoints and centroids; "
            "not a continuous global minimum"
        ),
    })

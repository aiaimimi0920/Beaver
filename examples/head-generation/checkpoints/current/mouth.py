"""A real lip opening, oral bag and thick tongue; deterministic jaw morph."""

from face import *

MOUTH_Y = Y0 + H * C["mouth_height"]
REST_GAP = H * C["mouth_rest_half_gap"]


def rest_seam_y(x):
    t = max(-1, min(1, x / (H * C["mouth_half_width"])))
    return MOUTH_Y + H * C["mouth_corner_lift"] * t * t


def mouth_half_contour(a):
    x = H * C["mouth_half_width"] * cos(a)
    return (x, rest_seam_y(x) + REST_GAP * sin(a))


def boundary_mouth_angles(points):
    half_width = max(abs(x) for x, y in points)
    vertical_span = max(y for x, y in points)-min(y for x, y in points)
    assert half_width>0 and vertical_span>0
    return [math.atan2((y-MOUTH_Y)/(vertical_span/2), x/half_width) for x,y in points]


def jaw_delta(x, y, z):
    side = exp(-((abs(x) / (H * C["mouth_lateral_falloff"])) ** 4))
    relative = y - rest_seam_y(x)
    upper = max(0, min(1, relative / REST_GAP))
    lower = max(0, min(1, -relative / REST_GAP))
    corner = -H * C["mouth_corner_down"]
    if relative >= 0:
        dy = (corner + (H * C["mouth_open_upper"] - corner) * upper) * exp(
            -((relative / (H * C["mouth_upper_falloff"])) ** 2)
        )
    else:
        dy = (corner + (-H * C["mouth_open_lower"] - corner) * lower) * exp(
            -((relative / (H * C["mouth_lower_falloff"])) ** 4)
        )
    height = (y - Y0) / H
    gate = max(
        0,
        min(
            1,
            (height - C["chin_morph_fixed_height"])
            / (C["chin_morph_blend_height"] - C["chin_morph_fixed_height"]),
        ),
    )
    gate = gate * gate * (3 - 2 * gate)
    follow = C["chin_follow_down_m"] * exp(-((height / C["chin_follow_height"]) ** 4))
    follow *= exp(-((abs(x) / (H * C["chin_follow_lateral"])) ** 4))
    return Vector((0, 0.0035 * lower * side * gate, dy * side * gate - follow))


def store_mouth_deltas(obj, override=None):
    attr = obj.data.attributes.new("MouthOpenDelta", "FLOAT_VECTOR", "POINT")
    for v in obj.data.vertices:
        attr.data[v.index].vector = override(v) if override else jaw_delta(v.co.x, v.co.z, -v.co.y)


def oral_ring_xy(x, y, index, count, ring, scale, upper_h, lower_h):
    t = (y-rest_seam_y(x))/REST_GAP
    xx = x*scale
    yy = y+t*H*(upper_h if t>0 else lower_h)
    blend = ring/max(1,len(C["oral_rings"])-1)
    blend = blend*blend*(3-2*blend)
    angle = -pi/2+2*pi*index/count
    target_x = H*C["mouth_half_width"]*scale*cos(angle)
    sy = sin(angle)
    target_y = MOUTH_Y+sy*(REST_GAP+H*(upper_h if sy>0 else lower_h))
    return xx*(1-blend)+target_x*blend, yy*(1-blend)+target_y*blend


def build_oral_cavity(half_edge):
    # Half opening starts at bottom center and ends at top center. Mirror it
    # without duplicating either seam; all subsequent rings have equal counts.
    edge = list(half_edge) + [(-v[0], v[1], v[2]) for v in half_edge[-2:0:-1]]
    n = len(edge)
    vs = []
    fs = []
    deltas = []
    colors = [(0.58, 0.27, 0.25), (0.26, 0.065, 0.072), (0.115, 0.019, 0.026)]
    # Lips attach exactly to the shell; interior expands behind the tiny rest gap.
    rings = []
    for ring, (back_h, scale, upper_h, lower_h) in enumerate(C["oral_rings"]):
        back = back_h * H
        ids = []
        for k, (x, negz, y) in enumerate(edge):
            t = (y - rest_seam_y(x)) / REST_GAP
            xx, yy = oral_ring_xy(x, y, k, n, ring, scale, upper_h, lower_h)
            rear_depth = -depth(0, MOUTH_Y) if ring >= 2 else negz
            p = (xx, rear_depth + back, yy)
            ids.append(len(vs))
            vs.append(p)
            d = jaw_delta(x, y, -negz)
            d *= 1 - 0.25 * ring / (len(C["oral_rings"]) - 1)
            deltas.append(d)
        rings.append(ids)
    for j in range(len(rings) - 1):
        for k in range(n):
            q = (k + 1) % n
            fs.append((rings[j][q], rings[j + 1][q], rings[j + 1][k], rings[j][k]))
    # Back cap is behind the tongue, not a high-valence fan on visible facial skin.
    fs.append(tuple(rings[-1]))
    oral = mesh("Oral cavity inner lips and back wall", vs, fs, colors[1])
    facing = sum(
        poly.normal.dot(Vector((-poly.center.x, 0, MOUTH_Y - poly.center.z))) * poly.area
        for poly in list(oral.data.polygons)[:-1]
    )
    assert facing > 0, "Oral side walls must face the cavity interior"
    oral["inward_facing_measure"] = facing
    oral.data.polygons[-1].use_smooth = False
    cap_depths = [vs[i][1] for i in rings[-1]]
    oral["rear_cap_planarity_m"] = max(cap_depths) - min(cap_depths)
    assert oral["rear_cap_planarity_m"] < 1e-8
    assert all(p.area > 1e-12 for p in oral.data.polygons), "Degenerate oral wall"

    attr = oral.data.attributes.new("MouthOpenDelta", "FLOAT_VECTOR", "POINT")
    for i, d in enumerate(deltas):
        attr.data[i].vector = d
    # Closed ellipsoid tongue: regular bands, poles hidden at the rear/bottom.
    tv = []
    tf = []
    cols = 16
    rows = 9
    for j in range(rows):
        a = -pi / 2 + pi * j / (rows - 1)
        rr = max(0.04, cos(a))
        for k in range(cols):
            t = 2 * pi * k / cols
            tv.append(
                coord(
                    0.013 * rr * cos(t),
                    1.576 + 0.0035 * sin(a),
                    depth(0, MOUTH_Y) - 0.023 + 0.012 * rr * sin(t),
                )
            )
    for j in range(rows - 1):
        for k in range(cols):
            tf.append(
                (
                    j * cols + k,
                    j * cols + (k + 1) % cols,
                    (j + 1) * cols + (k + 1) % cols,
                    (j + 1) * cols + k,
                )
            )
    tf.extend([tuple(reversed(range(cols))), tuple((rows - 1) * cols + k for k in range(cols))])
    tongue = mesh("Tongue with thickness", tv, [tuple(reversed(f)) for f in tf], (0.66, 0.24, 0.28))
    store_mouth_deltas(tongue, lambda v: Vector((0, 0.002, -H * C["tongue_open_down"])))


def validate_oral_clearance(shell):
    """Check neutral hidden wall samples against actual evaluated facial skin."""
    from mathutils.bvhtree import BVHTree

    oral = next(o for o in PARTS if o.name == "Oral cavity inner lips and back wall")
    bpy.context.view_layer.update()
    evaluated = shell.evaluated_get(bpy.context.evaluated_depsgraph_get())
    data = evaluated.to_mesh()
    count = len(oral.data.vertices) // len(C["oral_rings"])
    signed_distances = []
    try:
        tree = BVHTree.FromPolygons(
            [v.co.copy() for v in data.vertices], [tuple(p.vertices) for p in data.polygons]
        )
        for vertex in list(oral.data.vertices)[2 * count :]:
            point, normal, _, _ = tree.find_nearest(vertex.co)
            assert point is not None, "Missing oral containment surface"
            signed_distances.append((vertex.co - point).dot(normal))
        assert max(signed_distances) < 0, "Hidden oral wall sample lies outside facial skin"
    finally:
        evaluated.to_mesh_clear()
    return {
        "scope": "neutral hidden wall vertices against evaluated skin; not a global collision test",
        "sampled_vertices": len(signed_distances),
        "maximum_signed_distance_m": max(signed_distances),
        "rear_cap_planarity_m": oral["rear_cap_planarity_m"],
    }

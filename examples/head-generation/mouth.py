"""A real lip opening, oral bag and thick tongue; deterministic jaw morph."""
from face import *
MOUTH_Y = Y0 + H * C['mouth_height']

def mouth_half_contour(a):
    return (0.0195 * cos(a), MOUTH_Y + H * C['mouth_corner_lift'] * cos(a) ** 2 + 0.00035 * sin(a))

def jaw_delta(x, y, z):
    side = exp(-(abs(x) / 0.049) ** 4)
    upper = max(0, min(1, (y - MOUTH_Y) / 0.00035))
    lower = max(0, min(1, (MOUTH_Y - y) / 0.00035))
    if y >= MOUTH_Y:
        dy = (-0.0065 + 0.0095 * upper) * exp(-((y - MOUTH_Y) / 0.012) ** 2)
    else:
        dy = (-0.0065 - 0.0095 * lower) * exp(-((y - MOUTH_Y) / 0.066) ** 4)
    return Vector((0, 0.0035 * lower * side, dy * side))

def store_mouth_deltas(obj, override=None):
    attr = obj.data.attributes.new('MouthOpenDelta', 'FLOAT_VECTOR', 'POINT')
    for v in obj.data.vertices:
        attr.data[v.index].vector = override(v) if override else jaw_delta(v.co.x, v.co.z, -v.co.y)

def build_oral_cavity(half_edge):
    edge = list(half_edge) + [(-v[0], v[1], v[2]) for v in half_edge[-2:0:-1]]
    n = len(edge)
    vs = []
    fs = []
    deltas = []
    colors = [(0.58, 0.27, 0.25), (0.26, 0.065, 0.072), (0.115, 0.019, 0.026)]
    rings = []
    for ring, (back, scale, height) in enumerate([(0, 1, 0), (0.0025, 0.98, 0.0005), (0.01, 1.45, 0.009), (0.031, 1.28, 0.013), (0.049, 0.52, 0.008)]):
        ids = []
        for k, (x, negz, y) in enumerate(edge):
            t = (y - MOUTH_Y - H * C['mouth_corner_lift'] * (x / 0.0195) ** 2) / 0.00035
            xx = x * scale
            yy = y + t * height * (1.12 if t > 0 else 0.85)
            p = (xx, negz + back, yy)
            ids.append(len(vs))
            vs.append(p)
            d = jaw_delta(x, y, -negz)
            d *= 1 - 0.25 * ring / 4
            deltas.append(d)
        rings.append(ids)
    for j in range(len(rings) - 1):
        for k in range(n):
            q = (k + 1) % n
            fs.append((rings[j][q], rings[j + 1][q], rings[j + 1][k], rings[j][k]))
    fs.append(tuple(rings[-1]))
    oral = mesh('Oral cavity inner lips and back wall', vs, fs, colors[1])
    facing = sum((poly.normal.dot(Vector((-poly.center.x, 0, MOUTH_Y - poly.center.z))) * poly.area for poly in list(oral.data.polygons)[:-1]))
    assert facing > 0, 'Oral side walls must face the cavity interior'
    oral['inward_facing_measure'] = facing
    attr = oral.data.attributes.new('MouthOpenDelta', 'FLOAT_VECTOR', 'POINT')
    for i, d in enumerate(deltas):
        attr.data[i].vector = d
    tv = []
    tf = []
    cols = 16
    rows = 9
    for j in range(rows):
        a = -pi / 2 + pi * j / (rows - 1)
        rr = max(0.04, cos(a))
        for k in range(cols):
            t = 2 * pi * k / cols
            tv.append(coord(0.013 * rr * cos(t), 1.576 + 0.0035 * sin(a), depth(0, MOUTH_Y) - 0.023 + 0.012 * rr * sin(t)))
    for j in range(rows - 1):
        for k in range(cols):
            tf.append((j * cols + k, j * cols + (k + 1) % cols, (j + 1) * cols + (k + 1) % cols, (j + 1) * cols + k))
    tf.extend([tuple(reversed(range(cols))), tuple(((rows - 1) * cols + k for k in range(cols)))])
    tongue = mesh('Tongue with thickness', tv, [tuple(reversed(f)) for f in tf], (0.66, 0.24, 0.28))
    store_mouth_deltas(tongue, lambda v: Vector((0, 0.002, -0.01)))

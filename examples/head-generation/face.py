import bpy, math
from mathutils import Vector
from math import sin, cos, pi, exp, sqrt
C = STYLE['calibration']
H = C['height_m']
Y0 = C['origin_y_m']
Z0 = C['chin_depth_m']

def curve(y, axis, points):
    xs = [p[0] for p in points]
    ys = [p[axis] for p in points]
    slopes = [(ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]) for i in range(len(xs) - 1)]
    ds = [slopes[0]] + [0 if a * b <= 0 else 2 * a * b / (a + b) for a, b in zip(slopes, slopes[1:])] + [slopes[-1]]
    i = next((i for i in range(len(xs) - 1) if y <= xs[i + 1]), len(xs) - 2)
    t = max(0, min(1, (y - xs[i]) / (xs[i + 1] - xs[i])))
    h = xs[i + 1] - xs[i]
    return (2 * t ** 3 - 3 * t * t + 1) * ys[i] + (t ** 3 - 2 * t * t + t) * h * ds[i] + (-2 * t ** 3 + 3 * t * t) * ys[i + 1] + (t ** 3 - t * t) * h * ds[i + 1]

def width(y):
    return H * curve((y - Y0) / H, 1, C['width_stations'])

def nose(x, y):
    v = (y - Y0) / H
    pts = C['nose_stations']
    if v <= pts[0][0] or v >= pts[-1][0]:
        return 0
    h = curve(v, 1, pts) * H
    w = curve(v, 2, pts) * H
    t = max(0, min(1, abs(x) / w))
    return h * (1 - 3 * t * t + 2 * t * t * t)

def base_depth(x, y):
    v = (y - Y0) / H
    q = min(1, abs(x) / max(0.001, width(y)))
    sections = []
    for height, points in C['cross_sections']:
        section_x = q * points[-1][0]
        sections.append((height, curve(section_x, 1, points)))
    z = Z0 + H * curve(v, 1, sections) + nose(x, y)
    lip = exp(-(x / 0.02) ** 4) * H
    z += lip * C['lower_lip_relief'] * exp(-((y - Y0 - H * C['lower_lip_height']) / (H * C['lower_lip_width'])) ** 2)
    z += lip * C['upper_lip_relief'] * exp(-((y - Y0 - H * C['upper_lip_height']) / (H * C['upper_lip_width'])) ** 2)
    z -= lip * C['mouth_seam_recess'] * exp(-((y - Y0 - H * C['mouth_height']) / (H * C['mouth_seam_width'])) ** 2)
    return z

def rim_depth(x, y):
    t = (abs(x) / H - C['eye_center_x']) / C['eye_half_width']
    z = C['eye_inner_depth'] * (1 - t) / 2 + C['eye_outer_depth'] * (1 + t) / 2
    z += C['eye_rim_vertical_bulge'] * max(0, 1 - t * t)
    return Z0 + H * z

def depth(x, y):
    raw = base_depth(x, y)
    t = (abs(x) / H - C['eye_center_x']) / C['eye_half_width']
    cy = Y0 + H * (C['eye_corner_y'] + C['eye_corner_slope'] * t)
    arch = C['eye_upper_arch'] if y >= cy else C['eye_lower_arch']
    power = C['eye_upper_power'] if y >= cy else C['eye_lower_power']
    r = sqrt(t * t + abs((y - cy) / (H * arch)) ** (1 / power))
    weight = exp(-((r - 1) / C['orbital_band_width']) ** 2)
    return raw + (rim_depth(x, y) - raw) * weight

def coord(x, y, z):
    return (x, -z, y)

def faceuv(x, y):
    return (max(0, min(1, 0.5 + x / 0.235)), max(0, min(1, (y - 1.5) / 0.31)))
PARTS = []

def mesh(name, verts, faces, color, role='Face', mirror=False):
    data = bpy.data.meshes.new(name)
    data.from_pydata(verts, [], faces)
    data.update()
    obj = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(obj)
    mat = bpy.data.materials.get(role) or bpy.data.materials.new(role)
    data.materials.append(mat)
    uv = data.uv_layers.new(name='UVMap')
    uv2 = data.uv_layers.new(name='UV2')
    attr = data.color_attributes.new(name='Color', type='FLOAT_COLOR', domain='POINT')
    for v in data.vertices:
        attr.data[v.index].color = (*color, 1)
    for poly in data.polygons:
        poly.use_smooth = True
        for li in poly.loop_indices:
            v = data.vertices[data.loops[li].vertex_index].co
            uv.data[li].uv = faceuv(v.x, v.z)
            uv2.data[li].uv = faceuv(v.x, v.z)
    if mirror:
        mod = obj.modifiers.new('Editable left-right mirror', 'MIRROR')
        mod.use_clip = True
        mod.use_mirror_merge = True
        mod.merge_threshold = 1e-05
    obj['npr_role'] = role
    obj['skin_detail'] = role == 'Face' and color == SKIN
    PARTS.append(obj)
    return obj

def grid_faces(rows, cols, reverse=False):
    fs = []
    for j in range(rows - 1):
        for i in range(cols - 1):
            q = (j * cols + i, j * cols + i + 1, (j + 1) * cols + i + 1, (j + 1) * cols + i)
            fs.append(tuple(reversed(q)) if reverse else q)
    return fs
SKIN = tuple(STYLE['skin_rgb'])
WHITE = (0.96, 0.945, 0.88)
INK = (0.055, 0.035, 0.045)

def eye_contour(side, t, upper):
    x = side * H * (C['eye_center_x'] + C['eye_half_width'] * t)
    cy = C['eye_corner_y'] + C['eye_corner_slope'] * t
    h = C['eye_upper_arch'] if upper else -C['eye_lower_arch']
    power = C['eye_upper_power'] if upper else C['eye_lower_power']
    return (x, Y0 + H * (cy + h * max(0, 1 - t * t) ** power))

def eye_surface(side, x, y):
    t = max(-1, min(1, (abs(x) - STYLE['eye_center_x_m']) / STYLE['eye_half_width_m']))
    _, hi = eye_contour(side, t, True)
    _, lo = eye_contour(side, t, False)
    v = max(0, min(1, (y - lo) / max(1e-05, hi - lo)))
    return depth(x, y) - 0.0006 + STYLE['eye_lens_bulge_m'] * (1 - t * t) * sin(pi * v)

def build_eyes():
    for side in [-1, 1]:
        cols = 17
        from eye_socket import build_socket
        build_socket(side)
        for upper in [False, True]:
            rim = []
            ink = []
            for lane in range(4):
                for i in range(cols):
                    t = -1 + 2 * i / (cols - 1)
                    x, y = eye_contour(side, t, upper)
                    a = max(0, 1 - t * t) ** 0.6
                    cover = H * (C['upper_cover'] if upper else C['lower_cover'])
                    offsets = [0.008 * H, 0.002 * H, -cover * 0.6, -cover]
                    relief = [0, 0.0003, -0.0004, -0.0012]
                    ry = y + (1 if upper else -1) * offsets[lane] * a
                    rim.append(coord(x, ry, (depth(x, ry) if lane < 2 else depth(x, y)) + relief[lane] * a))
            mesh(('Upper' if upper else 'Lower') + ' skin lid ' + str(side), rim, grid_faces(4, cols, side > 0 if upper else side < 0), SKIN)
            for lane in range(2):
                for i in range(cols):
                    t = -1 + 2 * i / (cols - 1)
                    x, y = eye_contour(side, t, upper)
                    y += (-1 if upper else 1) * lane * (STYLE['upper_lash_thickness_m'] if upper else C['lower_cover'] * H) * max(0, 1 - t * t) ** 0.55 * (0.85 + 0.35 * (t + 1) / 2)
                    ink.append(coord(x, y, depth(x, y) + 0.0015))
            mesh(('Upper' if upper else 'Lower') + ' lash ' + str(side), ink, grid_faces(2, cols, side > 0 if upper else side < 0), INK if upper else (0.12, 0.08, 0.075))
            if upper:
                x, y = eye_contour(side, 0.92, True)
                tip = (x + side * 0.006, y + 0.0035)
                wing = [coord(x, y, depth(x, y) + 0.0015), coord(tip[0], tip[1], depth(tip[0], tip[1]) + 0.0009), coord(x, y + 0.0025, depth(x, y + 0.0025) + 0.0015)]
                mesh('Soft outer lash flick ' + str(side), wing, [(0, 1, 2) if side > 0 else (2, 1, 0)], INK)
        for upper, ts in [(True, [0.6, 0.8]), (False, [0.64, 0.83])]:
            for t in ts:
                x, y = eye_contour(side, t, upper)
                scale = STYLE['lash_fan_scale']
                direction = 1 if upper else -1
                points = [(x - side * 0.0015, y), (x + side * 0.0015, y), (x + side * 0.004 * scale, y + direction * 0.0035 * scale)]
                vs = [coord(xx, yy, depth(xx, yy) + 0.0018) for xx, yy in points]
                mesh('Attached lash accent ' + str((side, upper, t)), vs, [(0, 1, 2) if (side > 0) == upper else (2, 1, 0)], INK)
        brow = []
        for lane in range(2):
            for i in range(25):
                t = i / 24
                x = side * (0.018 + 0.064 * t)
                y = Y0 + H * C['brow_height'] + STYLE['brow_arch_m'] * sin(pi * t * 0.88) - 0.003 * t
                y += lane * H * C['brow_thickness'] * sin(pi * t) ** 0.7
                brow.append(coord(x, y, depth(x, y) + 0.001))
        mesh('Tapered brow ' + str(side), brow, grid_faces(2, 25, side < 0), (0.13, 0.105, 0.1))

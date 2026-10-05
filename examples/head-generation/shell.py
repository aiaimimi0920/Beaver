from face import *

def build_face():
    levels = [1.544, 1.551, 1.56, 1.57, 1.58, 1.59, 1.599, 1.608, 1.614, 1.623, 1.633, 1.644, 1.655, 1.664, 1.673, 1.685, 1.701, 1.72, 1.74, 1.76, 1.78]
    assert min((b - a for a, b in zip(levels, levels[1:]))) >= 0.0059
    lateral = [0, 0.06, 0.12, 0.18, 0.25, 0.34, 0.44, 0.55, 0.67, 0.78, 0.87, 0.94, 1]
    rows, cols = (len(levels), len(lateral))
    verts = []
    for j in range(rows):
        y = levels[j]
        for i in range(cols):
            focus = max(0, min(1, (y - 1.544) / 0.026)) * max(0, min(1, (1.71 - y) / 0.028))
            q = lateral[i] * focus + i / (cols - 1) * (1 - focus)
            x = width(y) * q
            top_blend = max(0, min(1, (y - 1.74) / 0.04))
            top_blend = top_blend * top_blend * (3 - 2 * top_blend)
            yy = y - STYLE['forehead_edge_drop_m'] * q ** 4 * top_blend + 0.002 * q * q * exp(-(y - 1.544) / 0.008)
            x = width(yy) * q
            verts.append(coord(x, yy, depth(x, yy)))
    ej0 = levels.index(1.614)
    ej1 = levels.index(1.673)
    ei0, ei1 = (3, 11)
    mj0 = levels.index(1.57)
    mj1 = levels.index(1.599)
    mi1 = 6
    shell_faces = []
    for j in range(rows - 1):
        for i in range(cols - 1):
            if ej0 <= j < ej1 and ei0 <= i < ei1:
                continue
            if mj0 <= j < mj1 and i < mi1:
                continue
            shell_faces.append((j * cols + i, j * cols + i + 1, (j + 1) * cols + i + 1, (j + 1) * cols + i))

    def patch(boundary, kind, closed):
        inner = []
        for vi in boundary:
            x, _, y = verts[vi]
            if kind == 'eye':
                a = math.atan2((y - (Y0 + H * C['eye_corner_y'])) / 0.03, (x - STYLE['eye_center_x_m']) / 0.04)
                t = cos(a)
                xx, yy = eye_contour(1, t, sin(a) >= 0)
            else:
                a = math.atan2((y - 1.581) / 0.018, x / 0.035)
                xx = 0.0195 * cos(a)
                yy = 1.581 + 0.0011 * sin(a)
            inner.append((xx, yy))
        if kind == 'mouth':
            from mouth import mouth_half_contour
            inner = [mouth_half_contour(-pi / 2 + pi * k / (len(boundary) - 1)) for k in range(len(boundary))]
        rings = []
        for blend in [0, 0.3, 0.65] if kind == 'eye' else [0, 0.3, 0.65]:
            ring = []
            for k, vi in enumerate(boundary):
                bx, _, by = verts[vi]
                ix, iy = inner[k]
                x = ix * (1 - blend) + bx * blend
                y = iy * (1 - blend) + by * blend
                dz = 0
                ring.append(len(verts))
                verts.append(coord(x, y, depth(x, y) + dz))
            rings.append(ring)
        rings.append(boundary)
        for ra, rb in zip(rings, rings[1:]):
            for k in range(len(boundary) if closed else len(boundary) - 1):
                kn = (k + 1) % len(boundary)
                shell_faces.append((ra[k], rb[k], rb[kn], ra[kn]))
        if kind == 'mouth':
            from mouth import build_oral_cavity
            build_oral_cavity([verts[i] for i in rings[0]])
    eye_boundary = [ej0 * cols + i for i in range(ei0, ei1 + 1)] + [j * cols + ei1 for j in range(ej0 + 1, ej1 + 1)] + [ej1 * cols + i for i in range(ei1 - 1, ei0 - 1, -1)] + [j * cols + ei0 for j in range(ej1 - 1, ej0, -1)]
    patch(eye_boundary, 'eye', True)
    mouth_boundary = [mj0 * cols + i for i in range(mi1 + 1)] + [j * cols + mi1 for j in range(mj0 + 1, mj1 + 1)] + [mj1 * cols + i for i in range(mi1 - 1, -1, -1)]
    patch(mouth_boundary, 'mouth', False)
    face = mesh('Face shell editable half', verts, shell_faces, SKIN, mirror=True)
    group = face.vertex_groups.new(name='Continuous facial shell')
    group.add(list(range(len(verts))), 1.0, 'REPLACE')
    from ears import build_ears
    build_ears()
    return {'face_authoring_quads': sum((len(q) == 4 for q in shell_faces)), 'face_authoring_triangles': sum((len(q) == 3 for q in shell_faces)), 'semantic_rows': levels}

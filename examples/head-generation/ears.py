"""Original ear rings: root, lobe, concha and raised helix, with back shell."""
from face import *

def build_ears():
    for side in [-1, 1]:
        vs = []
        fs = []
        n = 24
        for r, relief in [(0.12, -H * C['ear_concha_depth']), (0.38, -H * C['ear_concha_depth'] * 0.9), (0.62, -H * C['ear_concha_depth'] * 0.4), (0.8, 0.009 * H), (1.0, 0)]:
            for k in range(n):
                a = 2 * pi * k / n
                y = Y0 + H * (C['ear_center_y'] + C['ear_half_height'] * r * sin(a))
                rootx = H * C['ear_center_x']
                rootz = Z0 + H * C['ear_depth']
                x = side * (rootx + H * (C['ear_half_width'] * r * cos(a) + C['ear_upper_outward'] * r * sin(a)))
                z = rootz + relief + C['ear_depth_tilt'] * (abs(x) - rootx)
                vs.append(coord(x, y, z))
        for j in range(4):
            for k in range(n):
                q = (k + 1) % n
                f = (j * n + k, (j + 1) * n + k, (j + 1) * n + q, j * n + q)
                fs.append(f if side > 0 else tuple(reversed(f)))
        fs.append(tuple(range(n)) if side > 0 else tuple(reversed(range(n))))
        back = []
        for k in range(n):
            x, z, y = vs[4 * n + k]
            back.append(len(vs))
            vs.append((x, z + H * C['ear_back_thickness'], y))
        for k in range(n):
            q = (k + 1) % n
            fs.append((4 * n + k, back[k], back[q], 4 * n + q))
        fs.append(tuple(reversed(back)))
        obj = mesh('Ear sculpted helix concha ' + str(side), vs, fs, SKIN)
        import bmesh
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
        if bm.calc_volume(signed=True) < 0:
            bmesh.ops.reverse_faces(bm, faces=list(bm.faces))
        bm.to_mesh(obj.data)
        bm.free()
        obj['ear_detail'] = True
        detail = obj.data.uv_layers.new(name='DetailUV')
        for loop in obj.data.loops:
            v = obj.data.vertices[loop.vertex_index].co
            span = sqrt(C['ear_half_width'] ** 2 + C['ear_upper_outward'] ** 2)
            u = (abs(v.x) / H - C['ear_center_x']) / (2 * span) + 0.5
            w = ((v.z - Y0) / H - C['ear_center_y']) / (2 * C['ear_half_height']) + 0.5
            detail.data[loop.index].uv = (max(0.02, min(0.98, u)), max(0.02, min(0.98, w)))

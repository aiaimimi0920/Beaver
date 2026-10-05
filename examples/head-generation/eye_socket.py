"""Parameterized expanding orbital pocket and independent concave iris."""
from face import *

def iris_depth(x, y):
    dx = abs(x) / H - C['iris_center_x']
    dy = (y - Y0) / H - C['iris_center_y']
    r2 = (dx / C['iris_half_width']) ** 2 + (dy / C['iris_half_height']) ** 2
    return Z0 + H * (C['iris_rim_depth'] + C['iris_plane_slope'] * dx - C['iris_concavity'] * max(0, 1 - r2))

def build_socket(side):
    n = 48
    vs = []
    fs = []
    cx = side * H * C['eye_center_x']
    cy = Y0 + H * C['eye_corner_y']
    for sx, sy, back in [(1, 1, 0), (1.07, 1.15, 0.03), (C['pocket_width_ratio'], C['pocket_height_ratio'], C['pocket_back']), (1.08, 1.12, C['pocket_back'] + 0.03), (0.18, 0.18, C['pocket_back'] + 0.045)]:
        for k in range(n):
            a = 2 * pi * k / n
            x, y = eye_contour(side, cos(a), sin(a) >= 0)
            xx = cx + (x - cx) * sx
            yy = cy + (y - cy) * sy
            vs.append(coord(xx, yy, rim_depth(x, y) - back * H - 0.0001))
    for j in range(4):
        for k in range(n):
            q = (k + 1) % n
            f = (j * n + k, j * n + q, (j + 1) * n + q, (j + 1) * n + k)
            fs.append(f if side > 0 else tuple(reversed(f)))
    fs.append(tuple((4 * n + k for k in range(n))) if side > 0 else tuple(reversed([4 * n + k for k in range(n)])))
    obj = mesh('Expanding recessed eye pocket ' + str(side), vs, fs, WHITE)
    obj['ocular_surface'] = True
    iv = []
    rows = 13
    cols = 17
    for j in range(rows):
        sy = -1 + 2 * j / (rows - 1)
        for i in range(cols):
            tx = -1 + 2 * i / (cols - 1)
            x = side * H * C['iris_center_x'] + H * C['iris_half_width'] * tx * sqrt(max(0.002, 1 - sy * sy))
            y = Y0 + H * (C['iris_center_y'] + C['iris_half_height'] * sy)
            iv.append(coord(x, y, iris_depth(x, y)))
    iris = mesh('Independent concave iris ' + str(side), iv, grid_faces(rows, cols), WHITE)
    iris['iris_detail'] = True
    iris['ocular_surface'] = True
    uv = iris.data.uv_layers.new(name='DetailUV')
    for loop in iris.data.loops:
        v = iris.data.vertices[loop.vertex_index].co
        uv.data[loop.index].uv = ((v.x - side * H * C['iris_center_x']) / (2 * STYLE['eye_half_width_m']) + 0.5, (v.z - Y0 - H * C['iris_center_y']) / 0.044 + 0.5)

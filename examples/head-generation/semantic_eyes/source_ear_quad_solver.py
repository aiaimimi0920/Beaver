"""Bounded normalized quad-ear layout from current procedural source inputs."""
import numpy as np
from types import SimpleNamespace
def ear_quad_least_squares(fun, x, max_nfev=500, ftol=1e-07, bounds=None, project=None):
    x = np.array(x, dtype=float)
    if project is not None: x = project(x)
    lo, hi = map(np.asarray, bounds)
    lam = 1.0
    r = fun(x)
    cost = np.dot(r, r) / 2
    n = len(x)
    calls = 1
    for iteration in range(max_nfev):
        h = 1e-05
        J = np.empty((len(r), n))
        for k in range(n):
            xp = x.copy()
            xm = x.copy()
            xp[k] = min(hi[k], x[k] + h)
            xm[k] = max(lo[k], x[k] - h)
            J[:, k] = (fun(xp) - fun(xm)) / (xp[k] - xm[k])
            calls += 2
        H = J.T @ J
        g = J.T @ r
        accepted = False
        for _ in range(15):
            dx = np.linalg.solve(H + lam * np.diag(np.maximum(np.diag(H), 0.0001)), -g)
            xn = np.clip(x + dx, lo, hi)
            if project is not None: xn = project(xn)
            rn = fun(xn)
            cn = np.dot(rn, rn) / 2
            calls += 1
            if cn < cost:
                change = cost - cn
                x, r, cost = (xn, rn, cn)
                lam = max(lam * 0.5, 1e-09)
                accepted = True
                break
            lam = min(lam * 4, 1000000000000.0)
        if not accepted or np.linalg.norm(g, np.inf) < 1e-08:
            break
    return SimpleNamespace(x=x, cost=cost, nfev=iteration + 1, function_calls=calls)
import numpy as np
from collections import Counter
EAR_QUAD_EXPECTED_FACES = [[0, 2, 1], [0, 3, 2], [0, 4, 3], [1, 2, 6], [1, 6, 5], [2, 3, 6], [3, 7, 6], [3, 4, 7], [4, 8, 7], [5, 6, 9], [6, 7, 9], [7, 10, 9], [7, 11, 10], [7, 8, 11], [8, 12, 11], [8, 13, 12], [8, 14, 13], [9, 10, 16, 15], [10, 11, 17, 16], [11, 12, 18, 17], [12, 13, 19, 18], [13, 14, 20, 19], [15, 16, 22, 21], [16, 17, 23, 22], [17, 18, 24, 23], [18, 19, 25, 24], [19, 20, 26, 25], [21, 22, 27], [22, 23, 27], [23, 24, 27], [24, 25, 27], [25, 26, 27], [0, 29, 4], [0, 28, 29], [4, 29, 31, 8], [29, 28, 30, 31], [8, 31, 33, 14], [31, 30, 32, 33], [14, 33, 35, 20], [33, 32, 34, 35], [20, 35, 37, 26], [35, 34, 36, 37], [26, 37, 27], [37, 36, 27], [38, 28, 0], [39, 27, 36]]
def ear_quad_cycle(q):
    q = tuple(q)
    r = tuple(reversed(q))
    return min([q[i:] + q[:i] for i in range(len(q))] + [r[i:] + r[:i] for i in range(len(r))])
import numpy as np
def ear_quad_measures(v, ff):
    ps = v[np.array(ff)][:, np.array([(0, 1, 2), (0, 2, 3), (0, 1, 3), (1, 2, 3)])]
    a = ps[:, :, 1] - ps[:, :, 0]
    b = ps[:, :, 2] - ps[:, :, 0]
    c = ps[:, :, 2] - ps[:, :, 1]
    n = np.cross(a, b)
    area = np.linalg.norm(n, axis=-1)
    n /= np.maximum(area[:, :, None], 1e-09)
    dots = np.stack([np.sum(n[:, 0] * n[:, 1], axis=-1), np.sum(n[:, 2] * n[:, 3], axis=-1)], axis=1)
    angles = []
    for u, w in [(a, b), (-a, c), (-b, -c)]:
        angles.append(np.degrees(np.arccos(np.clip(np.sum(u * w, axis=-1) / (np.linalg.norm(u, axis=-1) * np.linalg.norm(w, axis=-1)), -1, 1))))
    return (dots, np.stack(angles, axis=-1), area)

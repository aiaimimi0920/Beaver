from paths import candidate_root, validation_root
import numpy as np


def closest(points, p, F):
    a = p[F[:, 0]]
    b = p[F[:, 1]]
    c = p[F[:, 2]]
    u = b - a
    v = c - a
    uu = np.einsum("ij,ij->i", u, u)
    vv = np.einsum("ij,ij->i", v, v)
    uv = np.einsum("ij,ij->i", u, v)
    den = uu * vv - uv * uv
    results = []
    for start in range(0, len(points), 64):
        q = points[start : start + 64]
        d = q[:, None] - a
        du = np.einsum("pfc,fc->pf", d, u)
        dv = np.einsum("pfc,fc->pf", d, v)
        s = (du * vv - dv * uv) / np.maximum(den, 1e-40)
        t = (dv * uu - du * uv) / np.maximum(den, 1e-40)
        inside = (s >= 0) & (t >= 0) & (s + t <= 1) & (den > 1e-35)
        bc = np.stack([1 - s - t, s, t], -1)
        proj = a + s[:, :, None] * u + t[:, :, None] * v
        dist = np.sum((q[:, None] - proj) ** 2, -1)
        dist[~inside] = np.inf
        for i, j in [(0, 1), (1, 2), (2, 0)]:
            x = p[F[:, i]]
            y = p[F[:, j]]
            e = y - x
            f = np.clip(
                np.einsum("pfc,fc->pf", q[:, None] - x, e)
                / np.maximum(np.einsum("fc,fc->f", e, e), 1e-40),
                0,
                1,
            )
            z = x + f[:, :, None] * e
            dd = np.sum((q[:, None] - z) ** 2, -1)
            better = dd < dist
            dist[better] = dd[better]
            eb = np.zeros_like(bc)
            eb[:, :, i] = 1 - f
            eb[:, :, j] = f
            bc[better] = eb[better]
        idx = dist.argmin(1)
        ar = np.arange(len(q))
        results.extend(
            [
                (float(np.sqrt(dist[k, idx[k]])), int(idx[k]), bc[k, idx[k]].tolist())
                for k in ar
            ]
        )
    return results

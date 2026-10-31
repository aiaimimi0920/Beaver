"""Bounded paired aperture depth field with fixed mouth corners."""


def shift_lip_seam(verts, rings, angles, guarded_depth, height, calibration,
                   forward_m=.00075):
    import numpy as np
    ids = np.asarray(rings, dtype=int)
    a = np.asarray(angles, dtype=float)
    if ids.shape != (5, len(a)) or len(a) < 3:
        raise ValueError("Unexpected seam ring contract")
    base = np.asarray(verts, dtype=float)[ids]
    if not np.isfinite(base).all() or not np.isfinite(forward_m) or abs(forward_m) > .0015:
        raise ValueError("Seam displacement exceeds the approved finite bound")
    half_width = height * calibration["mouth_half_width"]
    if not np.isfinite(half_width) or half_width <= 0:
        raise ValueError("Invalid mouth half width")
    lateral = np.clip(np.abs(base[0, :, 0])/half_width, 0., 1.)
    moves = forward_m * (1-lateral**4)**2
    points = base.copy()
    points[0, :, 1] -= moves
    quantized = points.astype(np.float32).astype(float)
    for candidate in (points, quantized):
        for k, angle in enumerate(a):
            x, negz, y = candidate[0, k]
            key = "upper_lip_edge_depth_delta" if angle >= 0 else "lower_lip_edge_depth_delta"
            dz = height * calibration[key] * np.sin(angle)**2
            # Keep the original aperture 4.2 mm reference bound, not the
            # internal upper support's exceptional 5.2 mm allowance.
            checked = guarded_depth(float(x), float(y),
                                    interpolated_depth=float(-negz-dz))
            if abs(checked+dz+negz) > 1e-10:
                raise ValueError("Seam original-reference reconstruction mismatch")
        for k in range(len(a)-1):
            selection = [(0, k), (1, k), (1, k+1), (0, k+1)]
            q = np.array([candidate[p] for p in selection])
            old = np.array([base[p] for p in selection])
            normals, old_normals = [], []
            for i, j, z in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0)):
                normals.append(np.cross(q[j]-q[i], q[z]-q[i]))
                old_normals.append(np.cross(old[j]-old[i], old[z]-old[i]))
            lengths = np.linalg.norm(normals, axis=1)
            if np.min(lengths) <= 1e-12:
                raise ValueError("Seam shift degenerates a source triangle")
            for n, previous in zip(normals, old_normals):
                if np.dot(n, previous) <= 0:
                    raise ValueError("Seam shift reverses a source triangle")
            for u, v in ((0, 1), (2, 3)):
                if np.dot(normals[u], normals[v])/(lengths[u]*lengths[v]) <= .1:
                    raise ValueError("Seam shift folds a source quad")
    # Transactional commit: cavity rings 0/1 subsequently inherit this boundary.
    for k, i in enumerate(ids[0]):
        verts[int(i)] = tuple(float(v) for v in points[0, k])
    return {"version": 2, "peak_forward_m": forward_m,
            "profile": "(1-(abs(x)/half_width)^4)^2; paired upper/lower; corners fixed", "approved_limit_m": .0015,
            "xy_fixed": True, "support_rings_fixed": True,
            "original_aperture_bound_unchanged": True,
            "cavity_policy": "rings 0/1 inherit aperture; deeper rings unchanged",
            "visual_acceptance": False}

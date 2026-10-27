"""Joint depth-only support refinement; source rays and every XY station stay fixed."""


def refine_lip_depth(verts, rings, angles, terms, guarded_depth, height, calibration):
    import numpy as np
    from scipy.optimize import minimize
    ids = np.asarray(rings, dtype=int)
    base = np.asarray(verts, dtype=float)[ids]
    a = np.asarray(angles, dtype=float)
    if base.shape != (5, len(a), 3) or not np.all(np.isfinite(base)):
        raise ValueError("Unexpected joint depth ring contract")
    count = len(a)
    s = np.sin(a)
    active = [(r, k) for k in range(1, count - 1) if 1e-7 < s[k] < .99999
              for r in (1, 2)]
    if not active:
        raise ValueError("No upper support rays available")
    limits, provenance = [], []
    for r, k in active:
        p, inner, outer = base[r, k], base[0, k], base[-1, k]
        ray = outer[[0, 2]] - inner[[0, 2]]
        t = float(np.dot(p[[0, 2]] - inner[[0, 2]], ray) / np.dot(ray, ray))
        if not 0 < t < 1:
            raise ValueError("Support point left its semantic ray")
        dz = height * calibration["upper_lip_edge_depth_delta"] * s[k]**2 * (1-t)**2
        raw, original = terms(float(p[0]), float(p[2]))
        depth = -p[1]
        lower = max(original-.0052+dz-depth, raw-.0015+dz-depth)
        upper = min(original+.0052+dz-depth, raw+.0015+dz-depth)
        if not np.isfinite(lower+upper) or lower >= upper:
            raise ValueError("Empty support depth interval")
        limits.append((lower*1000+1e-5, upper*1000-1e-5))
        provenance.append((raw, original, dz))
    selected = np.array(sorted({k for _, k in active}), dtype=int)

    def turns(points):
        mirrored = points[-2:0:-1].copy()
        mirrored[:, 0] *= -1
        loop = np.concatenate((points, mirrored))
        u, v = loop-np.roll(loop, 1, axis=0), np.roll(loop, -1, axis=0)-loop
        denominator = np.linalg.norm(u, axis=1)*np.linalg.norm(v, axis=1)
        return np.degrees(np.arccos(np.clip(np.sum(u*v, axis=1)/denominator, -1, 1)))[:count]

    def radial(points):
        d = np.diff(points[:, selected], axis=0)
        cosine = np.sum(d[:-1]*d[1:], axis=2)/(np.linalg.norm(d[:-1], axis=2)*np.linalg.norm(d[1:], axis=2))
        return np.degrees(np.arccos(np.clip(cosine, -1, 1))).T

    quads = [(r, k) for r in range(3) for k in range(count-1)
             if any((rr, kk) in active for rr in (r, r+1) for kk in (k, k+1))]
    edges = set()
    for r, k in quads:
        q = [(r, k), (r+1, k), (r+1, k+1), (r, k+1)]
        for p, z in zip(q, q[1:]+q[:1]):
            edges.add(tuple(sorted((p, z))))
    edges = sorted(edges)

    def lengths(points):
        return np.array([np.linalg.norm(points[p]-points[z])*1000 for p, z in edges])

    def diagonal_quality(points):
        q = np.array([[points[r, k], points[r+1, k], points[r+1, k+1], points[r, k+1]]
                      for r, k in quads])
        normals = [np.cross(q[:, b]-q[:, a], q[:, c]-q[:, a])
                   for a, b, c in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0))]
        return np.concatenate([np.sum(normals[u]*normals[v], axis=1) /
                               (np.linalg.norm(normals[u], axis=1)*np.linalg.norm(normals[v], axis=1))
                               for u, v in ((0, 1), (2, 3))])

    old_turns = np.array([turns(base[r]) for r in (1, 2)])
    old_radial, old_lengths = radial(base), lengths(base)
    cache = {}

    def evaluate(values):
        key = tuple(values)
        if key in cache:
            return cache[key]
        points = base.copy()
        for delta, (r, k) in zip(values, active):
            points[r, k, 1] -= delta/1000
        circum = np.array([turns(points[r]) for r in (1, 2)])
        rad, length = radial(points), lengths(points)
        checks = np.concatenate(((old_turns-circum).ravel()/10,
                                 (old_radial[:, 0]-rad[:, 0])/10,
                                 (old_radial[:, 1:]+3-.02-rad[:, 1:]).ravel()/10,
                                 length-np.maximum(.9*old_lengths, np.minimum(old_lengths, 1.)),
                                 1.1*old_lengths-length, diagonal_quality(points)-.1))
        objective = float(np.mean(rad[:, 0]**2)+.1*np.mean(values**2))
        result = objective, checks, points, rad, circum
        cache.clear()
        cache[key] = result
        return result

    solved = minimize(lambda x: evaluate(x)[0], np.zeros(len(active)), method="SLSQP",
                      bounds=limits, constraints=[{"type": "ineq", "fun": lambda x: evaluate(x)[1]}],
                      options={"maxiter": 150, "ftol": 1e-8})
    objective, checks, points, rad, circum = evaluate(solved.x)
    if not solved.success or not np.isfinite(objective) or not np.all(np.isfinite(checks)) or min(checks) < -1e-7:
        raise ValueError("Joint depth constraints failed: "+solved.message)
    quantized = points.astype(np.float32).astype(float)
    circum_q = np.array([turns(quantized[r]) for r in (1, 2)])
    radial_q = radial(quantized)
    baseline_q = base.astype(np.float32).astype(float)
    baseline_radial_q = radial(baseline_q)
    baseline_turns_q = np.array([turns(baseline_q[r]) for r in (1, 2)])
    baseline_lengths_q = lengths(baseline_q)
    # Numerical storage comparison only; this is not an extra artistic angle allowance.
    if np.any(circum_q > baseline_turns_q+.001) or np.any(radial_q[:, 1:] > baseline_radial_q[:, 1:]+3):
        raise ValueError("Float32 joint depth tangent check failed")
    if np.any(radial_q[:, 0] > baseline_radial_q[:, 0]) or min(diagonal_quality(quantized)) <= .1:
        raise ValueError("Float32 joint depth quad check failed")
    length_q = lengths(quantized)
    if np.any(length_q < np.maximum(.9*baseline_lengths_q, np.minimum(baseline_lengths_q, 1.))) or np.any(length_q > 1.1*baseline_lengths_q):
        raise ValueError("Float32 joint depth spacing check failed")
    for (r, k), (_, _, dz) in zip(active, provenance):
        for value in (points[r, k], quantized[r, k]):
            x, y, depth = float(value[0]), float(value[2]), float(-value[1])
            raw, _ = terms(x, y)
            move = depth-dz-raw
            guarded_depth(x, y)
            checked = guarded_depth(x, y, move, upper_lip_local=True)+dz
            if abs(checked-depth) > 1e-10:
                raise ValueError("Depth reconstruction mismatch")
    for r, k in active:
        verts[int(ids[r, k])] = tuple(float(v) for v in points[r, k])
    return {"version": 1, "iterations": int(solved.nit), "xy_fixed": True,
            "circumferential_float32_tolerance_deg": .001,
            "max_move_m": float(np.max(abs(solved.x))/1000),
            "radial_before_deg": old_radial[:, 0].tolist(),
            "radial_after_deg": rad[:, 0].tolist(), "visual_acceptance": False}

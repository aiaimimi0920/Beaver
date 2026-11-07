"""Bounded first-support-ring fairness on current semantic mouth rays.

This solver never changes the aperture, outer rings, topology or vertex count.
It evaluates proposals separately, then calls the strict source depth guard
before writing any accepted coordinate. It does not establish visual likeness.
"""


def refine_lip_support(verts, rings, angles, fraction, terms, guarded_depth,
                       height, calibration, ridge_height):
    import numpy as np
    from scipy.optimize import minimize

    if len(rings) != 5 or any(len(r) != len(angles) for r in rings):
        raise ValueError("Unexpected mouth support-ring contract")
    ids = np.asarray(rings, dtype=int)
    base = np.asarray(verts, dtype=float)[ids]
    a = np.asarray(angles, dtype=float)
    if not np.all(np.isfinite(base)) or not np.all(np.isfinite(a)):
        raise ValueError("Non-finite mouth support input")
    if not np.all(np.diff(a) > 0):
        raise ValueError("Mouth rays must be ordered")
    count = len(a)
    if count < 5 or max(abs(base[0, 0, 0]), abs(base[0, -1, 0])) > 1e-8:
        raise ValueError("Expected a half-mouth with pinned midline ends")
    s = np.sin(a)
    old_t = np.array([fraction(.5, angle) for angle in a])
    second_t = np.array([fraction(.75, angle) for angle in a])
    free = np.arange(1, count - 1)
    corner = int(np.argmin(abs(s)))
    if abs(s[corner]) > 1e-8:
        raise ValueError("Mouth corner ray is missing")
    inner, first, second, outer = base[0], base[1], base[2], base[-1]
    keys = ["upper_lip_edge_depth_delta" if v >= 0 else
            "lower_lip_edge_depth_delta" for v in s]
    rim = np.array([height * calibration[k] * v * v for k, v in zip(keys, s)])

    def close_half(points):
        mirrored = points[-2:0:-1].copy()
        mirrored[:, 0] *= -1
        return np.concatenate((points, mirrored))

    def turns(points):
        loop = close_half(points)
        left = loop - np.roll(loop, 1, axis=0)
        right = np.roll(loop, -1, axis=0) - loop
        lengths = np.linalg.norm(left, axis=1) * np.linalg.norm(right, axis=1)
        if np.any(lengths <= 1e-20):
            raise ValueError("Collapsed mouth support segment")
        cosine = np.sum(left * right, axis=1) / lengths
        return np.degrees(np.arccos(np.clip(cosine, -1, 1)))[:count]

    def triangle_areas(points):
        quads = []
        for left, right in ((inner, points), (points, second)):
            for k in range(count - 1):
                quads.append([left[k], right[k], right[k + 1], left[k + 1]])
        q = np.asarray(quads)[:, :, [0, 2]]
        values = []
        for order in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0)):
            tri = q[:, order]
            u, v = tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0]
            values.extend(u[:, 0] * v[:, 1] - u[:, 1] * v[:, 0])
        return np.asarray(values)

    old_turns = turns(first)
    limits = old_turns + np.where(old_turns < 30, 3., 1.)
    limits[corner] = old_turns[corner] + 1.
    # Reserve a small positive angle margin for Blender float32 storage.
    solve_limits = limits - .02
    areas = triangle_areas(first)
    if np.any(abs(areas) < 1e-14):
        raise ValueError("Degenerate source mouth support quad")
    sign = np.sign(areas)
    min_inner = np.minimum(np.linalg.norm(first - inner, axis=1), .001)
    min_second = np.minimum(np.linalg.norm(first - second, axis=1), .001)
    cache = {}

    def evaluate(values):
        key = tuple(values)
        if key in cache:
            return cache[key]
        t = old_t.copy()
        t[free] = values
        points = first.copy()
        checks, details, errors = [], [], []
        for k in range(count):
            point = first[k] + (t[k] - old_t[k]) * (outer[k] - inner[k])
            x, y = float(point[0]), float(point[2])
            raw, original = terms(x, y)
            dz = rim[k] * (1 - t[k]) ** 2
            move = .33 * max(0., s[k]) ** 2 * (
                -inner[k, 1] * (1 - t[k]) - outer[k, 1] * t[k] - raw - dz)
            # Midline endpoints are pinned, including their original .35 fairing.
            if k not in free:
                point = first[k].copy()
                move = -point[1] - raw - dz
            else:
                point[1] = -(raw + move + dz)
            bound = .0052 if s[k] > 0 else .0042
            checks.extend(((bound - abs(raw + move - original)) * 1000,
                           (.0042 - abs(raw - original)) * 1000,
                           (.0015 - abs(move)) * 1000))
            points[k] = point
            details.append((raw, original, move, dz))
            if s[k] > .2:
                errors.append((point[2] - ridge_height(x)) * 1000)
        turn = turns(points)
        checks.extend((solve_limits - turn) / 10)
        checks.extend((triangle_areas(points) * sign - abs(areas) * .05) * 1e6)
        checks.extend((np.linalg.norm(points - inner, axis=1) - min_inner) * 1000)
        checks.extend((np.linalg.norm(points - second, axis=1) - min_second) * 1000)
        objective = (np.mean(np.square(errors)) + .005 * np.mean(turn ** 2)
                     + .05 * np.mean(((values - old_t[free]) * 10) ** 2))
        result = (float(objective), np.asarray(checks), points, details, turn)
        cache.clear()
        cache[key] = result
        return result

    result = minimize(lambda v: evaluate(v)[0], old_t[free], method="SLSQP",
                      bounds=[(.03, second_t[k] - .03) for k in free],
                      constraints=[{"type": "ineq", "fun": lambda v: evaluate(v)[1]}],
                      options={"maxiter": 150, "ftol": 1e-8})
    objective, checks, points, details, new_turns = evaluate(result.x)
    if not result.success or not np.isfinite(objective) or np.min(checks) < -1e-7:
        raise ValueError("Joint lip support did not satisfy bounded constraints: " + result.message)
    quantized = points.astype(np.float32).astype(float)
    if np.any(turns(quantized) > limits + 1e-6):
        raise ValueError("Float32 lip support angle margin failed")
    if np.any(triangle_areas(quantized) * sign <= 0):
        raise ValueError("Float32 lip support changed quad orientation")
    for adjacent in (inner, second):
        if np.any(np.linalg.norm(quantized - adjacent, axis=1) <
                  np.minimum(np.linalg.norm(first - adjacent, axis=1), .001) - 1e-7):
            raise ValueError("Float32 mouth support gap decreased")
    for left, right in ((inner, quantized), (quantized, second)):
        for k in range(count - 1):
            q = np.array([left[k], right[k], right[k + 1], left[k + 1]])
            normals = [np.cross(q[b] - q[a], q[c] - q[a])
                       for a, b, c in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0))]
            for n, m in ((normals[0], normals[1]), (normals[2], normals[3])):
                if np.linalg.norm(n) <= 1e-14 or np.linalg.norm(m) <= 1e-14 or np.dot(n, m) <= 0:
                    raise ValueError("Joint mouth support folds in 3D")
    for k in free:
        xq, yq = float(quantized[k, 0]), float(quantized[k, 2])
        rawq, originalq = terms(xq, yq)
        dzq = details[k][3]
        moveq = -quantized[k, 1] - dzq - rawq
        guarded_depth(xq, yq)
        guarded_depth(xq, yq, moveq if s[k] > 0 else 0.,
                      upper_lip_local=bool(s[k] > 0),
                      interpolated_depth=-quantized[k, 1] - dzq)
    for k in free:
        x, y = float(points[k, 0]), float(points[k, 2])
        raw, original, move, dz = details[k]
        guarded_depth(x, y)
        final = guarded_depth(x, y, move, upper_lip_local=bool(s[k] > 0)) + dz
        if abs(final + points[k, 1]) > 1e-10:
            raise ValueError("Joint lip source depth reconstruction differs")
    # Transactional write only after every scalar/geometry guard succeeds.
    for k in free:
        verts[int(ids[1, k])] = tuple(float(v) for v in points[k])
    return {"version": 1, "iterations": int(result.nit),
            "first_ring_upper_fairing": .33, "midline_fixed": True,
            "corner_turn_before_deg": float(old_turns[corner]),
            "corner_turn_after_deg": float(new_turns[corner]),
            "max_ring_depth_m": max(abs(r + m - o) for r, o, m, _ in details),
            "visual_acceptance": False}

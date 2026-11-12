"""Exact-primal analytic branches; ambiguous nonsmooth terms flag fallback."""
import numpy as np

def forehead_cd_scatter_edges(ge, n):
    g = np.zeros((n, ge.shape[1]))
    g[:-1] -= ge
    g[1:] += ge
    return g

def forehead_cd_variation(points, frame=None):
    p = np.asarray(points, float)
    q = p if frame is None else p @ np.asarray(frame).T
    e = np.diff(q, axis=0)
    l = np.linalg.norm(e, axis=1)
    if len(p) < 4 or np.any(l <= 0):
        raise ValueError('Collapsed/short curve')
    L = l.sum()
    u = e / l[:, None]
    dual = (l[:-1] + l[1:]) / 2
    between = (dual[:-1] + dual[1:]) / 2
    if frame is None:
        k = np.diff(u, axis=0) / dual[:, None]
    else:
        theta = np.arctan2(u[:-1, 0] * u[1:, 1] - u[:-1, 1] * u[1:, 0], np.sum(u[:-1] * u[1:], axis=1))
        if np.any(np.pi - np.abs(theta) < 1e-08):
            raise ValueError('Angle branch cut')
        k = (theta / dual)[:, None]
    dk = np.diff(k, axis=0)
    square = np.sum(dk * dk, axis=1)
    value = float(np.sum(square / between) * L ** 3)
    z = 2 * dk / between[:, None] * L ** 3
    gk = np.zeros_like(k)
    gk[:-1] -= z
    gk[1:] += z
    gb = -square / between ** 2 * L ** 3
    gd = np.zeros_like(dual)
    gd[:-1] += 0.5 * gb
    gd[1:] += 0.5 * gb
    gd -= np.sum(gk * k, axis=1) / dual
    gl = np.full_like(l, 3 * value / L)
    gl[:-1] += 0.5 * gd
    gl[1:] += 0.5 * gd
    if frame is None:
        gu = np.zeros_like(u)
        z = gk / dual[:, None]
        gu[:-1] -= z
        gu[1:] += z
        ge = (gu - u * np.sum(gu * u, axis=1)[:, None]) / l[:, None] + gl[:, None] * u
    else:
        gt = gk[:, 0] / dual
        ge = gl[:, None] * u
        ge[:-1] += gt[:, None] * np.c_[e[:-1, 1], -e[:-1, 0]] / l[:-1, None] ** 2
        ge[1:] += gt[:, None] * np.c_[-e[1:, 1], e[1:, 0]] / l[1:, None] ** 2
    g = forehead_cd_scatter_edges(ge, len(p))
    return (value, g if frame is None else g @ frame)

def forehead_cd_weighted_cv(values, weights):
    x = np.asarray(values)
    w = np.asarray(weights)
    W = w.sum()
    mu = float(np.sum(w * x) / W)
    var = float(np.sum(w * (x - mu) ** 2) / W)
    if mu <= 0:
        raise ValueError('Nonpositive mean')
    sd = np.sqrt(var)
    value = float(sd / mu)
    # A rounded positive variance can still be numerical zero. Keep an
    # input-scale roundoff band for weighted sums and normalization; never
    # manufacture an analytic CV gradient from that cancellation noise.
    roundoff = 32 * x.size * np.finfo(float).eps * max(float(np.max(abs(x))), abs(mu))
    if sd <= roundoff:
        return (value, None, None)
    gx = w / W * ((x - mu) / (sd * mu) - sd / (mu * mu))
    gw = ((x - mu) ** 2 - var) / (2 * W * sd * mu) - sd * (x - mu) / (W * mu * mu)
    return (value, gx, gw)

def forehead_cd_support_cv(points, quads):
    p = np.asarray(points)
    g = np.zeros_like(p)
    value = 0.0
    flags = set()
    for q in quads:
        q = np.asarray(q)
        e = np.roll(p[q], -1, axis=0) - p[q]
        l = np.linalg.norm(e, axis=1)
        if np.any(l <= 0):
            raise ValueError('Collapsed support')
        cv, dl, _ = forehead_cd_weighted_cv(l, np.ones(len(l)))
        value += cv / len(quads)
        if dl is None:
            flags.update(q.tolist())
            continue
        ge = (dl / l)[:, None] * e / len(quads)
        np.add.at(g, q, -ge)
        np.add.at(g, np.roll(q, -1), ge)
    return (value, g, flags)

def forehead_cd_contact_gradient(a, b, c, d, s, t, vector, distance):
    """Canonical derivative for the same endpoint/segment contact feature.

    Adjacent rail segments can represent one shared endpoint as t=1 and t=0.
    Recompute that feature from its actual endpoint, not r+s*e-t*f, so the
    scattered derivatives are exactly equal without relaxing tie tolerances.
    The candidate distance and parallel/degenerate branch checks stay unchanged.
    """
    if distance == 0:
        return None
    if s not in (0., 1.) and t not in (0., 1.):
        return np.array([(1-s)*vector, s*vector, -(1-t)*vector, -t*vector])/distance
    gradient = np.zeros((4, 3))
    if s in (0., 1.) and t in (0., 1.):
        vector = (a, b)[int(s)] - (c, d)[int(t)]
    elif t in (0., 1.):
        endpoint = (c, d)[int(t)]
        edge = b-a
        s = np.clip((endpoint-a) @ edge / (edge @ edge), 0, 1)
        vector = (a-endpoint)+s*edge
    else:
        endpoint = (a, b)[int(s)]
        edge = d-c
        t = np.clip((endpoint-c) @ edge / (edge @ edge), 0, 1)
        vector = (endpoint-c)-t*edge
    length = np.linalg.norm(vector)
    if not np.isfinite(length) or length == 0:
        return None
    unit = vector/length
    gradient[0] = (1-s)*unit
    gradient[1] = s*unit
    gradient[2] = -(1-t)*unit
    gradient[3] = -t*unit
    return gradient

def forehead_cd_pair_candidates(a, b, c, d):
    e = b - a
    f = d - c
    r = a - c
    aa = e @ e
    bb = e @ f
    cc = f @ f
    dd = e @ r
    ee = f @ r
    if min(aa, cc) <= 0:
        raise ValueError('Collapsed rail')
    pairs = [(0.0, np.clip(ee / cc, 0, 1)), (1.0, np.clip((ee + bb) / cc, 0, 1)), (np.clip(-dd / aa, 0, 1), 0.0), (np.clip((bb - dd) / aa, 0, 1), 1.0)]
    det = aa * cc - bb * bb
    safe = det > np.finfo(float).eps * aa * cc
    if safe:
        s = (bb * ee - cc * dd) / det
        t = (aa * ee - bb * dd) / det
        if 0 <= s <= 1 and 0 <= t <= 1:
            pairs.append((s, t))
    out = []
    for s, t in pairs:
        v = r + s * e - t * f
        dist = np.linalg.norm(v)
        grad = forehead_cd_contact_gradient(a, b, c, d, s, t, v, dist)
        out.append((float(dist), grad, not safe))
    return out

def forehead_cd_rail_cv(first, second):
    a = np.asarray(first)
    b = np.asarray(second)
    p = np.r_[a, b]
    n = len(p)
    m = len(a)
    matrix = {}
    flags = []
    for i in range(len(a) - 1):
        for j in range(len(b) - 1):
            cand = []
            for value, local, parallel in forehead_cd_pair_candidates(a[i], a[i + 1], b[j], b[j + 1]):
                g = np.zeros_like(p)
                if local is not None:
                    np.add.at(g, [i, i + 1, m + j, m + j + 1], local)
                cand.append((value, g, parallel or local is None))
            matrix[i, j] = cand

    def select(candidates):
        best = min((v for v, g, f in candidates))
        tol = 32 * np.finfo(float).eps * max(best, 1e-15)
        tied = [(g, f) for v, g, f in candidates if abs(v - best) <= tol]
        g = tied[0][0]
        ambiguous = any((f or not np.array_equal(gg, g) for gg, f in tied))
        return (best, g, ambiguous)
    rows = []
    for i in range(len(a) - 1):
        rows.append(select([x for j in range(len(b) - 1) for x in matrix[i, j]]))
    for j in range(len(b) - 1):
        rows.append(select([x for i in range(len(a) - 1) for x in matrix[i, j]]))
    distances = np.array([r[0] for r in rows])
    e = np.r_[np.diff(a, axis=0), np.diff(b, axis=0)]
    lengths = np.linalg.norm(e, axis=1)
    value, dx, dw = forehead_cd_weighted_cv(distances, lengths)
    g = np.zeros_like(p)
    # Equal scattered gradients of homogeneous distances imply equal geometric
    # distances (Euler's identity). Scalar cancellation must not manufacture a
    # nonzero CV derivative for this identical-contact, zero-variance branch.
    same_contact = all(np.array_equal(gg, rows[0][1]) for _, gg, _ in rows)
    if dx is None or same_contact or any((f for _, _, f in rows)):
        return (value, g, set(range(n)), {'ambiguous_minima': sum((f for _, _, f in rows)),
                'zero_variance': dx is None or same_contact,
                'identical_minimum_gradients': same_contact})
    for coefficient, (_, gg, _) in zip(dx, rows):
        g += coefficient * gg
    ge = dw[:, None] * e / lengths[:, None]
    g[:m] += forehead_cd_scatter_edges(ge[:m - 1], m)
    g[m:] += forehead_cd_scatter_edges(ge[m - 1:], len(b))
    return (value, g, set(), {'ambiguous_minima': 0, 'zero_variance': False})

def forehead_cd_objective(stage, points):
    p = np.asarray(points, float)
    g = np.zeros_like(p)
    value = 0.0
    fallback = set()
    details = {}
    for ri, path in enumerate(stage.paths.values()):
        ids = np.asarray(path)
        q = p[ids]
        for offset, frame in [(0, None)] + [(i, frame) for i, frame in zip((5, 7, 9), stage.frames.values())]:
            v, gg = forehead_cd_variation(q, frame)
            scale = max(stage.reference[11 * ri + offset], 1e-12)
            value += v / scale
            np.add.at(g, ids, gg / scale)
    ai = np.asarray(stage.paths['blue'])
    bi = np.asarray(stage.paths['purple'])
    v, gg, flags, detail = forehead_cd_rail_cv(p[ai], p[bi])
    joined = np.r_[ai, bi]
    value += v
    np.add.at(g, joined, gg)
    fallback.update((int(joined[i]) for i in flags))
    details['rail'] = detail
    v, gg, flags = forehead_cd_support_cv(p, stage.quads)
    value += v
    g += gg
    fallback.update(flags)
    stage.charge('objective_differential_production_primal')
    actual = stage.energy_from(stage.measure(p))
    if not np.isfinite(value) or not np.isfinite(actual) or not np.isclose(value, actual, rtol=2e-12, atol=1e-10):
        raise ValueError('Complete-guide objective differential/primal mismatch')
    details['analytic_primal'] = float(value)
    details['analytic_primal_abs_difference'] = abs(float(value) - actual)
    return (actual, g, sorted(fallback), details)

def forehead_cd_chain(world, ids, jacobian, world_fallback, donor_flags=None):
    ids = np.asarray(ids)
    J = np.asarray(jacobian)
    if J.shape != (len(ids), 3, 2):
        raise ValueError('Source differential shape')
    flags = np.isin(ids, world_fallback)
    if donor_flags is not None:
        flags |= np.asarray(donor_flags, bool)
    return (np.einsum('ni,nia->na', world[ids], J).reshape(-1), np.repeat(flags, 2))

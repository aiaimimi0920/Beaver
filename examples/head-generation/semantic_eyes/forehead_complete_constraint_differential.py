from forehead_complete_world_differential import forehead_cd_variation, forehead_cd_weighted_cv, forehead_cd_scatter_edges, forehead_cd_rail_cv, forehead_cd_pair_candidates, forehead_cd_objective
"""Metric derivatives with explicit unsupported coordinate masks."""
import numpy as np

def forehead_cd_extremum(values, gradients, invalid=None, maximum=True, branch_radius=0.0):
    values = np.asarray(values)
    k = int(np.argmax(values) if maximum else np.argmin(values))
    v = values[k]
    roundoff = np.sqrt(np.finfo(float).eps) * max(abs(v), 1e-15)
    sensitivity = np.array([np.linalg.norm(g.reshape(-1, 3), axis=1).sum() if g.size % 3 == 0 else np.linalg.norm(g) for g in gradients])
    tied = np.flatnonzero(abs(values - v) <= roundoff + branch_radius * (sensitivity + sensitivity[k]))
    bad = any((invalid is not None and invalid[i] or not np.array_equal(gradients[i], gradients[k]) for i in tied))
    return (float(v), gradients[k], bad)

def forehead_cd_edges(points, closed=False):
    p = np.asarray(points)
    e = np.roll(p, -1, axis=0) - p if closed else np.diff(p, axis=0)
    l = np.linalg.norm(e, axis=1)
    if np.any(l <= 0):
        raise ValueError('Collapsed edge')
    g = np.zeros((len(e), len(p), 3))
    for i in range(len(e)):
        g[i, i] = -e[i] / l[i]
        g[i, (i + 1) % len(p)] = e[i] / l[i]
    return (e, l, g)

def forehead_cd_peak(points, frame=None, branch_radius=0.0):
    p = np.asarray(points)
    q = p if frame is None else p @ frame.T
    e = np.diff(q, axis=0)
    l = np.linalg.norm(e, axis=1)
    if np.any(l <= 0):
        raise ValueError('Collapsed peak edge')
    u = e / l[:, None]
    dot = np.sum(u[:-1] * u[1:], axis=1)
    grads = []
    bad = []
    if frame is None:
        angle = np.arccos(np.clip(dot, -1, 1))
        den = np.sqrt(np.maximum(0, 1 - dot * dot))
        for i in range(len(angle)):
            ge = np.zeros_like(e)
            invalid = den[i] <= 1e-10
            if not invalid:
                ge[i] = -(u[i + 1] - dot[i] * u[i]) / (l[i] * den[i])
                ge[i + 1] = -(u[i] - dot[i] * u[i + 1]) / (l[i + 1] * den[i])
            grads.append(forehead_cd_scatter_edges(ge, len(p)) * 180 / np.pi)
            bad.append(invalid)
    else:
        signed = np.arctan2(u[:-1, 0] * u[1:, 1] - u[:-1, 1] * u[1:, 0], dot)
        angle = abs(signed)
        for i in range(len(angle)):
            ge = np.zeros_like(e)
            invalid = angle[i] <= 1e-12 or np.pi - angle[i] <= 1e-08
            if not invalid:
                ge[i] = np.sign(signed[i]) * np.array([e[i, 1], -e[i, 0]]) / l[i] ** 2
                ge[i + 1] = np.sign(signed[i]) * np.array([-e[i + 1, 1], e[i + 1, 0]]) / l[i + 1] ** 2
            grads.append(forehead_cd_scatter_edges(ge, len(p)) @ frame * 180 / np.pi)
            bad.append(invalid)
    return forehead_cd_extremum(np.degrees(angle), np.asarray(grads), bad, branch_radius=branch_radius)

def forehead_cd_metric_rows(stage, points, branch_radius=0.0):
    p = np.asarray(points, float)
    rows = []
    grads = []
    flags = []
    deps = []

    def add(value, local, ids, bad=False, fallback=None):
        ids = np.asarray(ids, int)
        g = np.zeros_like(p)
        np.add.at(g, ids, local)
        rows.append(float(value))
        grads.append(g)
        deps.append(set(ids.tolist()))
        flags.append(set(ids.tolist()) if bad else set() if fallback is None else set(fallback))
    for path in stage.paths.values():
        ids = np.asarray(path)
        q = p[ids]
        e, l, lg = forehead_cd_edges(q)
        v, g = forehead_cd_variation(q)
        add(v, g, ids)
        v, g, b = forehead_cd_peak(q, branch_radius=branch_radius)
        add(v, g, ids, b)
        v, dl, _ = forehead_cd_weighted_cv(l, np.ones(len(l)))
        add(v, np.zeros_like(q) if dl is None else np.einsum('i,ijk->jk', dl, lg), ids, dl is None)
        v, g, b = forehead_cd_extremum(l, lg, maximum=False, branch_radius=branch_radius)
        add(-v * 1000, -g * 1000, ids, b)
        v, g, b = forehead_cd_extremum(l, lg, branch_radius=branch_radius)
        add(v * 1000, g * 1000, ids, b)
        for f in stage.frames.values():
            v, g = forehead_cd_variation(q, f)
            add(v, g, ids)
            v, g, b = forehead_cd_peak(q, f, branch_radius=branch_radius)
            add(v, g, ids, b)
    a = np.asarray(stage.paths['blue'])
    b = np.asarray(stage.paths['purple'])
    ids = np.r_[a, b]
    q = p[ids]
    m = len(a)
    candidates = []
    gg = []
    invalid = []
    for i in range(len(a) - 1):
        for j in range(len(b) - 1):
            for v, g, parallel in forehead_cd_pair_candidates(p[a[i]], p[a[i + 1]], p[b[j]], p[b[j + 1]]):
                full = np.zeros_like(q)
                if g is not None:
                    np.add.at(full, [i, i + 1, m + j, m + j + 1], g)
                candidates.append(v)
                gg.append(full)
                invalid.append(parallel or g is None)
    v, g, bad = forehead_cd_extremum(candidates, np.asarray(gg), invalid, maximum=False, branch_radius=branch_radius)
    add(-v * 1000, -g * 1000, ids, bad)
    v, g, f, _ = forehead_cd_rail_cv(p[a], p[b])
    add(v, g, ids, fallback={int(ids[i]) for i in f})
    cv = []
    cg = []
    cb = []
    alllengths = []
    edgeowners = []
    edgegrads = []
    for quad in stage.quads:
        e, l, lg = forehead_cd_edges(p[quad], True)
        v, dl, _ = forehead_cd_weighted_cv(l, np.ones(4))
        cv.append(v)
        cg.append(np.zeros((4, 3)) if dl is None else np.einsum('i,ijk->jk', dl, lg))
        cb.append(dl is None)
        for k in range(4):
            alllengths.append(l[k])
            edgeowners.append(quad)
            edgegrads.append(lg[k])
    support = np.unique(stage.quads)
    index = {int(x): i for i, x in enumerate(support)}

    def expand(owners, g):
        out = np.zeros((len(support), 3))
        np.add.at(out, [index[int(i)] for i in owners], g)
        return out

    def sparse_extreme(values, owners, gs, bads=None, maximum=True):
        values = np.asarray(values)
        k = int(np.argmax(values) if maximum else np.argmin(values))
        v = values[k]
        g = expand(owners[k], gs[k])
        sensitivity = np.asarray([np.linalg.norm(z, axis=1).sum() for z in gs])
        tied = np.flatnonzero(abs(values - v) <= np.sqrt(np.finfo(float).eps) * max(abs(v), 1e-15) + branch_radius * (sensitivity + sensitivity[k]))
        bad = any((bads is not None and bads[i] or not np.array_equal(expand(owners[i], gs[i]), g) for i in tied))
        return (float(v), g, bad)
    v, g, bad = sparse_extreme(cv, stage.quads, cg, cb)
    add(v, g, support, bad)
    g = np.zeros((len(support), 3))
    fb = set()
    for quad, local, bad in zip(stage.quads, cg, cb):
        g += expand(quad, local) / len(stage.quads)
        if bad:
            fb.update(quad.tolist())
    add(np.mean(cv), g, support, fallback=fb)
    v, g, bad = sparse_extreme(alllengths, edgeowners, edgegrads, maximum=False)
    add(-v * 1000, -g * 1000, support, bad)
    v, g, bad = sparse_extreme(alllengths, edgeowners, edgegrads)
    add(v * 1000, g * 1000, support, bad)
    stage.charge('constraint_differential_production_primal')
    actual = stage.measure(p)
    if not np.allclose(rows, actual, rtol=2e-12, atol=1e-10):
        raise ValueError('Metric arithmetic mismatch')
    return (actual, np.asarray(grads), flags, deps, float(np.max(abs(np.asarray(rows) - actual))))

def forehead_cd_parameter_constraints(stage, points, ids, differential, boundary):
    radius = 1e-07 * float(np.max(np.linalg.norm(np.asarray(differential), axis=(1, 2))))
    actual, world, flags, deps, error = forehead_cd_metric_rows(stage, points, branch_radius=radius)
    value, eg, ef, _ = forehead_cd_objective(stage, points)
    factors = np.maximum(abs(np.asarray(stage.reference)), 1.0)
    world = -world / factors[:, None, None]
    world = np.concatenate([world, -eg[None]], axis=0)
    flags.append(set(ef))
    deps.append(set(stage.nodes))
    ids = np.asarray(ids)
    J = np.asarray(differential) / 1000
    jac = np.einsum('rni,nia->rna', world[:, ids], J).reshape(len(world), -1)
    pattern = np.zeros_like(jac, dtype=bool)
    fallback = np.zeros_like(jac, dtype=bool)
    for r, (dep, flag) in enumerate(zip(deps, flags)):
        pattern[r] = np.repeat(np.isin(ids, list(dep)), 2)
        fallback[r] = np.repeat(np.isin(ids, list(flag)) | np.asarray(boundary, bool) & np.isin(ids, list(dep)), 2)
    if np.any(fallback & ~pattern):
        raise ValueError('Fallback exceeds dependency proof')
    values = np.r_[(stage.caps - actual) / factors, stage.target - value]
    return (values, jac, fallback, pattern, {'metric_arithmetic_max_abs_error': error, 'fallback_rows': np.flatnonzero(fallback.any(1)).tolist()})

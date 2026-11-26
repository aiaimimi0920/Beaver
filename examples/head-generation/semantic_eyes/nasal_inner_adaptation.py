"""Bounded source-relative new-inner-point fitting along existing real-edge rails.
Pure CPU. Final bidirectional surface/field/native guards remain caller obligations.
"""
import copy
import heapq
import numpy as np
from scipy.spatial import cKDTree
from scipy.optimize import least_squares

def inner_rail_triangle_nearest(points, triangles):
    tree = cKDTree(triangles.mean(1))
    distances = []
    owners = []
    for start in range(0, len(points), 1500):
        ps = points[start:start + 1500]
        _, ix = tree.query(ps, k=min(32, len(triangles)))
        if ix.ndim == 1:
            ix = ix[:, None]
        q = triangles[ix]
        p = ps[:, None, :]
        a = q[:, :, 0]
        ab = q[:, :, 1] - a
        ac = q[:, :, 2] - a
        ap = p - a
        aa = np.sum(ab * ab, 2)
        cc = np.sum(ac * ac, 2)
        bb = np.sum(ab * ac, 2)
        d = np.sum(ap * ab, 2)
        e = np.sum(ap * ac, 2)
        den = aa * cc - bb * bb
        u = (cc * d - bb * e) / np.maximum(den, 1e-30)
        w = (aa * e - bb * d) / np.maximum(den, 1e-30)
        foot = a + u[:, :, None] * ab + w[:, :, None] * ac
        ds = np.linalg.norm(foot - p, axis=2)
        ds[(u < 0) | (w < 0) | (u + w > 1)] = np.inf
        for j in range(3):
            x = q[:, :, j]
            edge = q[:, :, (j + 1) % 3] - x
            t = np.clip(np.sum((p - x) * edge, 2) / np.maximum(np.sum(edge * edge, 2), 1e-30), 0, 1)
            ds = np.minimum(ds, np.linalg.norm(x + t[:, :, None] * edge - p, axis=2))
        best = np.argmin(ds, 1)
        distances.extend(ds[np.arange(len(ps)), best])
        owners.extend(ix[np.arange(len(ps)), best])
    return (np.asarray(distances), np.asarray(owners, int))

def adapt_inner_real_rails(source_vertices, source_quads, source_regions, donor_vertices, donor_quads, donor_regions, inner_spec, proposed_vertices, final_faces=None):
    source = np.asarray(source_vertices, float)
    v = np.asarray(donor_vertices, float)
    base = np.asarray(proposed_vertices, float)
    removed = set(inner_spec['removed_faces'])
    grid = np.array(inner_spec['grid_positive'], int)
    newquads = [list(map(int, q)) for q in inner_spec['new_quads']]
    skin = [list(map(int, q)) for i, (q, r) in enumerate(zip(donor_quads, donor_regions)) if r == 1 and i not in removed] + newquads
    if final_faces is not None:
        expected = [list(map(int, q)) for i, q in enumerate(donor_quads) if i not in removed] + newquads

        def oriented(q):
            q = list(map(int, q))
            return min((tuple(q[k:] + q[:k]) for k in range(len(q))))
        if sorted(map(oriented, expected)) != sorted(map(oriented, final_faces)):
            raise ValueError('Final inner connectivity differs from declared replacement')
    skin = np.array(skin, int)
    newset = {frozenset(q) for q in newquads}
    newids = sorted(map(int, inner_spec.get('new_point_ids', inner_spec['new_point_lineage'].keys())))
    boundary = np.asarray(inner_spec['boundary_positive'], int)
    extent = v[boundary]
    lo = extent.min(0) - 0.002
    hi = extent.max(0) + 0.002
    lo[0] = 0
    sq = source[np.array([q for q, r in zip(source_quads, source_regions) if r == 1 and len(q) == 4])]
    sq = sq[np.all(sq.max(1) >= lo, 1) & np.all(sq.min(1) <= hi, 1)]
    weights = np.array([[(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w] for u in np.linspace(0, 1, 9) for w in np.linspace(0, 1, 9)])
    samples = np.einsum('wj,tjk->twk', weights, sq).reshape(-1, 3)
    samples = samples[np.all(samples >= lo, 1) & np.all(samples <= hi, 1)]
    if not len(samples):
        raise ValueError('Missing current-source nasal probes')
    diagonals = [np.array([[0, 1, 2], [0, 2, 3]]), np.array([[0, 1, 3], [1, 2, 3]])]
    failures = {}
    before = []
    for ix in diagonals:
        distances, owners = inner_rail_triangle_nearest(samples, base[skin][:, ix].reshape(-1, 3, 3))
        before.append(float(max(distances)))
        for k in np.flatnonzero(distances > 0.00100001):
            q = skin[owners[k] // 2]
            key = tuple(map(int, q))
            if frozenset(q) not in newset or not set(q) <= set(newids):
                raise ValueError('Source-shape miss is outside editable inner rail cells')
            if key not in failures or distances[k] > failures[key][0]:
                failures[key] = (float(distances[k]), samples[k].copy())
    if not failures:
        return (inner_spec, proposed_vertices, dict(status='unchanged_source_probe_gate_passed', before_max_m=before))
    if len(failures) > 2:
        raise ValueError('Too many failing inner rail cells')
    var = sorted({i for q in failures for i in q}, key=lambda i: tuple(base[i]))
    lineage = {int(i): L for i, L in inner_spec['new_point_lineage'].items()}
    if len(var) > 8 or any((np.count_nonzero(grid == i) != 1 for i in var)):
        raise ValueError('Unsupported inner rail variables')
    md, mi = cKDTree(base[newids]).query(base[var] * [-1, 1, 1])
    if max(md) > 1e-07:
        raise ValueError('Missing symmetric new-point ownership')
    mirrors = {i: newids[int(j)] for i, j in zip(var, mi)}
    adj = {}
    for fi in removed:
        q = list(donor_quads[fi])
        for a, b in zip(q, q[1:] + q[:1]):
            adj.setdefault(a, set()).add(b)
            adj.setdefault(b, set()).add(a)
    rails = {}
    for i in var:
        row, col = np.argwhere(grid == i)[0]
        start = int(grid[0, col])
        end = int(grid[4, col])
        heap = [(0.0, tuple(v[start]), start, [start])]
        seen = set()
        path = None
        while heap:
            cost, _, x, current = heapq.heappop(heap)
            if x in seen:
                continue
            seen.add(x)
            if x == end:
                path = current
                break
            for y in adj[x]:
                if v[y, 2] >= v[x, 2] - 1e-10:
                    continue
                penalty = np.linalg.norm(v[y] - v[x]) + 100 * abs(v[y, 0] - v[x, 0])
                heapq.heappush(heap, (cost + penalty, tuple(v[y]), y, current + [y]))
        if path is None:
            raise ValueError('Disconnected real nasal rail')
        length = np.r_[0, np.cumsum(np.linalg.norm(np.diff(v[path], axis=0), axis=1))]
        donors = list(lineage[i]['donors'])
        hits = [k for k in range(len(path) - 1) if set(path[k:k + 2]) == set(donors)]
        if len(hits) != 1:
            raise ValueError('Ambiguous current rail lineage')
        k = hits[0]
        t = lineage[i]['weights'][donors.index(path[k + 1])]
        s0 = length[k] + t * (length[k + 1] - length[k])
        rails[i] = (path, length, s0)

    def proposal(z):
        out = base.copy()
        changed = {}
        for k, i in enumerate(var):
            path, length, s0 = rails[i]
            s = s0 + z[k] * 0.0005
            if not 0 <= s <= length[-1]:
                raise ValueError('Bounded nasal rail escaped source')
            j = min(int(np.searchsorted(length, s, side='right') - 1), len(path) - 2)
            t = (s - length[j]) / (length[j + 1] - length[j])
            donors = path[j:j + 2]
            out[i] = (1 - t) * v[donors[0]] + t * v[donors[1]]
            out[mirrors[i]] = out[i] * [-1, 1, 1]
            changed[i] = {'donors': donors, 'weights': [float(1 - t), float(t)]}
        return (out, changed)

    def objective(z):
        out, _ = proposal(z)
        errors = []
        for q, (_, point) in failures.items():
            for ix in diagonals:
                errors.append(inner_rail_triangle_nearest(point[None], out[list(q)][ix])[0][0])
        return np.r_[z * 0.002, np.maximum(0, np.array(errors) - 0.00095) * 1000]
    lower = np.array([max(-1.0, -rails[i][2] / 0.0005) for i in var])
    upper = np.array([min(1.0, (rails[i][1][-1] - rails[i][2]) / 0.0005) for i in var])
    fit = least_squares(objective, np.zeros(len(var)), bounds=(lower, upper), max_nfev=30, diff_step=0.001, ftol=1e-09, xtol=1e-09, gtol=1e-09)
    out, changed = proposal(fit.x)
    out = out.astype(np.float32).astype(float)
    after = []
    for ix in diagonals:
        distances, _ = inner_rail_triangle_nearest(samples, out[skin][:, ix].reshape(-1, 3, 3))
        after.append(float(max(distances)))
    if max(after) > 0.00100001:
        raise ValueError('Bounded real-rail fit failed unchanged1mm source-probe gate')
    if np.max(np.linalg.norm(out - base, axis=1)) > 0.0005002:
        raise ValueError('Real-rail displacement exceeded float32 budget')
    spec = copy.deepcopy(inner_spec)
    newL = {int(i): L for i, L in spec['new_point_lineage'].items()}
    mirror_tree = cKDTree(v)
    for i, L in changed.items():
        j = mirrors[i]
        dist, donors = mirror_tree.query(v[L['donors']] * [-1, 1, 1])
        if max(dist) > 1e-07:
            raise ValueError('Mirrored donor edge unavailable')
        newL[i] = dict(L, source_edge=L['donors'])
        newL[j] = {'donors': list(map(int, donors)), 'weights': L['weights'], 'source_edge': list(map(int, donors))}
    spec['new_point_lineage'] = newL
    report = dict(status='bounded_existing_rail_walk', source_probe_count=len(samples), before_max_m=before, after_max_m=after, variable_new_points=var, max_displacement_m=float(np.max(np.linalg.norm(out - base, axis=1))), optimizer_converged=bool(fit.success), rail_arclength_budget_m=0.0005, warning='This helper checks source-to-candidate probes only; full bidirectional/final topology/native fields/poses/export guards are mandatory.')
    spec['rail_shape_adaptation'] = report
    return (spec, out, report)

def adapt_inner_nasal_rails(source_vertices, source_faces, source_regions, current_vertices, current_faces, current_regions, spec, proposed, final_faces):
    """Joint-stage callback. Geometry and connectivity retain native axis convention.

    Caller invokes fitting for profiles whose old patch needed adaptation. Already
    accepted profiles keep their existing actual-export contract, not a newly
    imposed hypothetical unused-diagonal requirement. This function itself never
    selects an export diagonal or relaxes any final guard.
    """
    updated, points, report = adapt_inner_real_rails(source_vertices, source_faces, source_regions, current_vertices, current_faces, current_regions, spec, proposed, final_faces)
    if updated is spec:
        updated = copy.deepcopy(spec)
    updated['rail_shape_adaptation'] = report
    return (updated, points, final_faces)

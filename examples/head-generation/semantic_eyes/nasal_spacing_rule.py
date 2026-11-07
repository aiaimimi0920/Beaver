"""CPU proposal only: current-source five-rail redistribution and upper bridge repair.
No saved mesh/artist coordinates/global IDs. Caller must perform native field/export/pose gates.
Native XYZ: lateral, negative-front depth, height. Input paths are outer-to-axis.
"""
from collections import defaultdict, Counter
import numpy as np
from scipy.spatial import cKDTree
from scipy.optimize import minimize
from nasal_source_geometry import AuthorQuadSurface

def propose_nasal_bridge_spacing(vertices, faces, regions, semantic_paths, ear_regions, outer_anchor_offset=4, strength=0.65):
    if not np.isfinite(strength) or not 0 <= strength <= 1:
        raise ValueError('Spacing strength must be finite in [0,1]')
    v = np.asarray(vertices, float).copy()
    if v.ndim != 2 or v.shape[1] != 3 or (not np.isfinite(v).all()):
        raise ValueError('Nonfinite or malformed current source')
    fs = [list(map(int, q)) for q in faces]
    r = np.asarray(regions)
    paths = [list(map(int, p)) for p in semantic_paths]
    if len(paths) != 5 or len(set(map(len, paths))) != 1 or len(paths[0]) < 10:
        raise ValueError('Unsupported current five-rail profile')
    if len(r) != len(fs) or 1 in set(ear_regions):
        raise ValueError('Invalid source region ownership')
    if int(outer_anchor_offset) != outer_anchor_offset or not 1 <= outer_anchor_offset < len(paths[0]) - 4:
        raise ValueError('Outer anchor offset invalid')
    C = np.array([p[::-1] for p in paths])
    adj = defaultdict(set)
    inc = defaultdict(list)
    ef = defaultdict(list)
    members = defaultdict(set)
    for i, (q, rr) in enumerate(zip(fs, r)):
        for x in q:
            members[x].add(int(rr))
        if rr != 1:
            continue
        if len(q) != 4:
            raise ValueError('Expected authoring skin quads')
        for a, b in zip(q, q[1:] + q[:1]):
            adj[a].add(b)
            adj[b].add(a)
            ef[tuple(sorted((a, b)))].append(i)
        for x in q:
            inc[x].append(i)
    if any((tuple(sorted(e)) not in ef for p in paths for e in zip(p, p[1:]))):
        raise ValueError('Disconnected semantic flow')
    if any((tuple(sorted((int(C[a, j]), int(C[a + 1, j])))) not in ef for a in range(4) for j in range(C.shape[1]))):
        raise ValueError('No coherent current transverse station edges')
    mixed = {i for i in range(len(v)) if members[i] != {1}}
    functional_mixed = {i for i in mixed if members[i] - {1} - set(ear_regions)}
    protected = mixed | {j for i in functional_mixed for j in adj[i]} | set(map(int, C[:, -1]))
    axis, near, previous = map(int, C[0, :3])
    below = int(C[1, 0])
    side = np.sign(v[near, 0])
    span = float(v[axis, 2] - v[C[4, 0], 2])
    if side != 1 or not 0.01 < span < 0.06 or abs(v[axis, 0]) > 1e-08 or (len(adj[axis]) != 6) or (len(adj[near]) != 3):
        raise ValueError('Unsupported upper bridge junction')

    def one(x, msg):
        x = list(x)
        if len(x) != 1:
            raise ValueError(msg)
        return int(x[0])
    above = one((i for i in adj[axis] if abs(v[i, 0]) < 1e-08 and v[i, 2] > v[axis, 2]), 'Upper axis ambiguous')
    side_bridge = one((i for i in adj[axis] if v[i, 0] > 1e-08 and i != near), 'Bridge rail ambiguous')
    upper_bridge = one((adj[above] & adj[side_bridge]) - {axis}, 'Upper bridge corner ambiguous')
    if {axis, near, side_bridge, upper_bridge} & protected:
        raise ValueError('Junction touches protected functional region')
    dist, mirror = cKDTree(v).query(v * [-1, 1, 1])
    domain = set(C.ravel()) | {axis, near, previous, above, side_bridge, upper_bridge}
    if max(dist[list(domain)]) > 1e-07:
        raise ValueError('Nasal source symmetry unsupported')
    lookup = {frozenset(q): i for i, q in enumerate(fs)}
    expected_old = [[axis, side_bridge, upper_bridge, above], [side_bridge, axis, near, previous]]
    oldids = [lookup.get(frozenset(q)) for q in expected_old]
    if None in oldids or set(oldids) != set(ef[tuple(sorted((axis, side_bridge)))]):
        raise ValueError('Unexpected bridge fan')
    newright = [[axis, near, upper_bridge, above], [near, previous, side_bridge, upper_bridge]]

    def orient(q, old):
        a = v[q]
        b = v[old]
        n = np.cross(a[1] - a[0], a[2] - a[0])
        m = np.cross(b[1] - b[0], b[2] - b[0])
        return q if n @ m > 0 else q[::-1]
    edits = {}
    for i, q in zip(oldids, newright):
        q = orient(q, fs[i])
        edits[i] = q
        mi = lookup[frozenset(mirror[fs[i]])]
        edits[mi] = [int(mirror[x]) for x in q[::-1]]
    outfaces = [q[:] for q in fs]
    for i, q in edits.items():
        outfaces[i] = q
    surf = AuthorQuadSurface(v, fs, r)
    if abs(v[near, 0] - v[previous, 0]) <= 1e-10 or v[axis, 2] - v[below, 2] <= 1e-10:
        raise ValueError('Collapsed tangent or nasal-axis interval')
    tangent_height = v[near, 2] - v[near, 0] * (v[near, 2] - v[previous, 2]) / (v[near, 0] - v[previous, 0])
    delta = float(np.clip(strength * (tangent_height - v[axis, 2]), -0.18 * (v[axis, 2] - v[below, 2]), 0))
    fraction = -delta / (v[axis, 2] - v[below, 2])
    axis_point = (1 - fraction) * v[axis] + fraction * v[below]
    mirror_bridge = int(mirror[side_bridge])
    fitfaces = sorted(set(inc[side_bridge] + inc[mirror_bridge] + list(edits)))

    def quality(w, qs):
        p = w[np.array(qs)]
        a = np.roll(p, 1, 1) - p
        b = np.roll(p, -1, 1) - p
        le = np.linalg.norm(a, axis=2)
        be = np.linalg.norm(b, axis=2)
        angles = np.degrees(np.arccos(np.clip(np.sum(a * b, axis=2) / (le * be), -1, 1)))
        cr = np.cross(a, b)[:, :, 1]
        sign = np.sign(np.sum(cr, axis=1))
        return (angles, cr * sign[:, None], le.max(1) / le.min(1))
    unit = span / 25

    def trial(d):
        p = v[side_bridge, [0, 2]] + unit * np.asarray(d)
        w = v.copy()
        w[axis] = axis_point
        w[side_bridge] = surf.sample(*p)
        w[mirror_bridge] = w[side_bridge] * [-1, 1, 1]
        return w

    def residual(d):
        if np.linalg.norm(d) > 4:
            return 100000000.0 + np.linalg.norm(d)
        try:
            w = trial(d)
        except ValueError:
            return 100000000.0
        angles, cv, ratios = quality(w, [outfaces[i] for i in fitfaces])
        return float(np.sum(np.maximum(26 - angles, 0) ** 2) + np.sum(np.maximum(angles - 155, 0) ** 2) + 1000000000000.0 * np.sum(np.maximum(1e-08 - cv, 0) ** 2) + 25 * max(ratios.max() - 3, 0) ** 2 + 4 * np.sum(np.asarray(d) ** 2))
    opt = minimize(residual, [1.7, 0], method='Nelder-Mead', options={'maxiter': 300, 'xatol': 1e-07, 'fatol': 1e-07})
    if not opt.success:
        raise ValueError('Bridge resampling did not converge')
    w = trial(opt.x)
    lineage = {axis: dict(donors=[axis, below], weights=[float(1 - fraction), float(fraction)])}
    donors, weights, _ = surf.lineage(*w[side_bridge, [0, 2]])
    lineage[side_bridge] = dict(donors=list(donors), weights=list(weights))
    lineage[mirror_bridge] = dict(donors=[int(mirror[i]) for i in donors], weights=list(weights))
    count = C.shape[1]
    anchor = count - 1 - int(outer_anchor_offset)
    if anchor < 3:
        raise ValueError('Outer anchor profile unsupported')
    for col in range(count):
        ids = C[:, col]
        points = v[ids].copy()
        if col == 0:
            points[0] = axis_point
        lengths = np.linalg.norm(np.diff(points, axis=0), axis=1)
        if np.any(lengths <= 1e-10):
            raise ValueError('Collapsed transverse source edge')
        s = np.r_[0, np.cumsum(lengths)]
        t = np.clip((col - anchor) / (count - 1 - anchor), 0, 1)
        blend = strength * (1 - 3 * t * t + 2 * t * t * t)
        for row in (1, 2, 3):
            idx = int(ids[row])
            if idx in protected:
                continue
            target = (1 - blend) * s[row] + blend * row / 4 * s[-1]
            k = min(np.searchsorted(s, target, side='right') - 1, 3)
            u = float((target - s[k]) / lengths[k])
            weights = [1 - u, u]
            donors = [int(ids[k]), int(ids[k + 1])]
            if col == 0 and k == 0:
                weights = [(1 - u) * (1 - fraction), (1 - u) * fraction + u]
            L = dict(donors=donors, weights=weights)
            w[idx] = np.array(weights) @ v[donors]
            lineage[idx] = L
            mi = int(mirror[idx])
            w[mi] = w[idx] * [-1, 1, 1]
            lineage[mi] = dict(donors=[int(mirror[j]) for j in donors], weights=weights)
    changed = set(map(int, np.flatnonzero(np.linalg.norm(w - v, axis=1) > 1e-10)))
    lineage = {i: L for i, L in lineage.items() if i in changed}
    affected = sorted((i for i, q in enumerate(fs) if set(q) & changed or i in edits))
    if changed & protected:
        raise ValueError('Protected region moved')
    if any((r[i] != 1 or len(outfaces[i]) != 4 for i in affected)):
        raise ValueError('Affected domain escapes skin quads')
    if any((np.linalg.norm(np.array(L['weights']) @ v[L['donors']] - w[i]) > 1e-08 or min(L['weights']) < 0 or abs(sum(L['weights']) - 1) > 1e-08 for i, L in lineage.items())):
        raise ValueError('Invalid source field lineage')

    def boundary(qs):
        ct = Counter((tuple(sorted(e)) for q in qs for e in zip(q, q[1:] + q[:1])))
        return {e for e, n in ct.items() if n == 1}
    if boundary([fs[i] for i in affected]) != boundary([outfaces[i] for i in affected]):
        raise ValueError('Changed domain boundary')
    nad = defaultdict(set)
    nin = defaultdict(list)
    for i, q in enumerate(outfaces):
        if r[i] != 1:
            continue
        for a, b in zip(q, q[1:] + q[:1]):
            nad[a].add(b)
            nad[b].add(a)
        for a in q:
            nin[a].append(i)
    for p in paths:
        for a, b, c in zip(p, p[1:], p[2:]):
            opp = nad[b] - {a} - set((x for fi in nin[b] if a in outfaces[fi] for x in outfaces[fi]))
            if c not in opp:
                raise ValueError('Five-rail opposite flow broken')
            oldopp = adj[b] - {a} - set((x for fi in inc[b] if a in fs[fi] for x in fs[fi]))
            if not opp - {c} <= oldopp - {c}:
                raise ValueError('New opposite-flow ambiguity introduced')
    angle, cv, ratio = quality(w, [outfaces[i] for i in affected])
    oldangle, oldcv, oldratio = quality(v, [fs[i] for i in affected])
    newbad = {affected[i] for i in np.flatnonzero(cv.min(1) <= 0)}
    oldbad = {affected[i] for i in np.flatnonzero(oldcv.min(1) <= 0)}
    if newbad - oldbad:
        raise ValueError('New front-chart fold')
    return dict(
        vertices=w,
        faces=outfaces,
        lineage=lineage,
        affected_faces=affected,
        edited_faces=edits,
        roles=dict(axis=axis, axis_below=below, axis_above=above, near=near, previous=previous, bridge=side_bridge, upper_bridge=upper_bridge),
        protected=sorted(protected),
        ear_owned_fixed=sorted((i for i in mixed if members[i] & set(ear_regions))),
        shape_gate='Pending CPU dense export + actual native fields/poses',
        metrics=dict(changed_vertices=len(changed), max_move_mm=float(1000 * np.linalg.norm(w - v, axis=1).max()), min_angle=float(angle.min()), max_angle=float(angle.max()), max_edge_ratio=float(ratio.max()), existing_projected_return_nonconvex=sorted(oldbad), new_projected_nonconvex=sorted(newbad)),
        axis_tangent=dict(target_height=float(tangent_height), height_move_mm=float(delta * 1000), fraction_of_current_axis_edge=float(fraction)),
        fade=dict(anchor_station=anchor, strength=strength, rule='1-3t²+2t³ from current outer anchor to fixed endpoint'),
    )

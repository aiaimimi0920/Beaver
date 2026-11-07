"""Bounded CPU-only extension: current rear skin rail + one generated posterior-rim station.
The caller MUST verify that the rim is the generated inward/backward open strip;
this helper does not classify an arbitrary boundary as safe from coordinates alone.
"""
import numpy as np
from scipy.optimize import minimize
from collections import Counter, defaultdict

def nasal_rear_turns(points):
    delta = np.diff(points, axis=0)
    lengths = np.linalg.norm(delta, axis=1)
    if np.any(lengths < 1e-10):
        raise ValueError('Zero-length flow edge')
    direction = delta / lengths[:, None]
    return np.degrees(np.arccos(np.clip(np.einsum('ij,ij->i', direction[:-1], direction[1:]), -1, 1)))

def propose_nasal_rear_spacing(vertices, faces, regions, paths_axis_to_rear, mirror, *, ear_regions, protected_vertices, max_move_m=0.002, reserve_deg=0.005):
    v = np.asarray(vertices, float)
    C = np.asarray(paths_axis_to_rear, int)
    if C.ndim != 2 or C.shape[0] != 5 or C.shape[1] < 6:
        raise ValueError('Need five current paths')
    if not np.isfinite(v).all() or not np.isfinite(max_move_m) or (not 0 < max_move_m <= 0.002):
        raise ValueError('Invalid movement bound')
    if not np.isfinite(reserve_deg) or not 0 < reserve_deg <= 0.02:
        raise ValueError('Invalid turn margin')
    inc = defaultdict(list)
    ad = defaultdict(set)
    mem = defaultdict(set)
    ed = Counter()
    for fi, (q, r) in enumerate(zip(faces, regions)):
        for i in q:
            inc[i].append(fi)
            mem[i].add(int(r))
        for a, b in zip(q, q[1:] + q[:1]):
            ad[a].add(b)
            ad[b].add(a)
            ed[tuple(sorted((a, b)))] += 1
    free = C[3, -3:]
    don = C[2, -3:]
    endpoint = int(free[-1])
    previous = int(free[-2])
    coface = {x for fi in inc[endpoint] if previous in faces[fi] for x in faces[fi]}
    opp = ad[endpoint] - {previous} - coface
    if len(opp) != 1:
        raise ValueError('Ambiguous return continuation')
    rim = next(iter(opp))
    support = [faces[fi] for fi in inc[rim] if endpoint in faces[fi] and int(don[-1]) in faces[fi]]
    if len(support) != 1 or len(support[0]) != 4:
        raise ValueError('No unique upper return support quad')
    rim_donor = next(iter(set(support[0]) - {rim, endpoint, int(don[-1])}))
    boundary = lambda i: {j for j in ad[i] if ed[tuple(sorted((i, j)))] == 1}
    rim_lower = boundary(rim) - {rim_donor}
    if len(rim_lower) != 1 or rim_donor not in boundary(rim):
        raise ValueError('Not a simple open rim')
    lower = next(iter(rim_lower))
    prev = boundary(lower) - {rim}
    nxt = boundary(rim_donor) - {rim}
    if len(prev) != 1 or len(nxt) != 1:
        raise ValueError('Open rim continuation ambiguous')
    boundary_path = [next(iter(prev)), lower, rim, rim_donor, next(iter(nxt))]
    ear_regions = set(map(int, ear_regions))
    protected_vertices = set(map(int, protected_vertices))
    if not ear_regions or not mem[rim_donor] - {1} <= ear_regions or (not mem[int(don[-1])] - {1} <= ear_regions):
        raise ValueError('Rim donors are not explicit ear-owned ports')
    if mem[rim] != {1} or 1 not in mem[rim_donor] or len(mem[rim_donor]) < 2:
        raise ValueError('Rim not pure skin / upper seam not explicit')
    ids = np.r_[free, rim]
    donors = np.r_[don, rim_donor]
    if (set(map(int, ids)) | {int(mirror[i]) for i in ids}) & protected_vertices:
        raise ValueError('Movable stations overlap functional protection')
    for i, j in zip(ids, donors):
        if mem[i] != {1} or tuple(sorted((int(i), int(j)))) not in ed:
            raise ValueError('Not a current skin donor edge')
    unit = v[donors] - v[ids]
    length = np.linalg.norm(unit, axis=1)
    if min(length) < 1e-09:
        raise ValueError('Zero donor length')
    unit /= length[:, None]
    fullpath = C[3].tolist() + [rim]
    oldturn = nasal_rear_turns(v[fullpath])
    oldrim = nasal_rear_turns(v[boundary_path])
    active = np.arange(len(oldturn) - 4, len(oldturn))

    def points(mm):
        w = v.copy()
        w[ids] += unit * mm[:, None] * 0.001
        return w

    def loss(mm):
        w = points(mm)
        g = np.linalg.norm(np.diff(w[C[:, -3:]], axis=0), axis=2)
        return float(np.sum(np.log(g[2] / g[3]) ** 2) + 0.0001 * mm[-1] ** 2)

    def gates(mm):
        w = points(mm)
        rt = nasal_rear_turns(w[boundary_path])
        return np.r_[oldturn[active] - reserve_deg - nasal_rear_turns(w[fullpath])[active], np.diff(mm[:3]), oldrim + 1.0 - rt, oldrim.sum() - rt.sum()]
    best = None
    for start in ([0.4, 0.8, 1.0, 1.0], [0.7, 1.4, 1.9, 2.0], [0.2, 0.4, 0.5, 0.5]):
        res = minimize(loss, np.minimum(start, max_move_m * 1000), method='SLSQP', bounds=[(0, max_move_m * 1000)] * 4, constraints={'type': 'ineq', 'fun': gates}, options={'maxiter': 800, 'ftol': 1e-12})
        if res.success and min(gates(res.x)) >= -1e-07 and (best is None or res.fun < best.fun):
            best = res
    if best is None:
        raise ValueError('No bounded turn-preserving candidate')
    w = v.copy()
    lineage = {}
    for i, j, mm, ll in zip(ids, donors, best.x, length):
        alpha = mm * 0.001 / ll
        if not 0 <= alpha < 1:
            raise ValueError('Invalid current-edge weight')
        for a, b in [(int(i), int(j)), (int(mirror[i]), int(mirror[j]))]:
            if mem[a] != {1} or tuple(sorted((a, b))) not in ed:
                raise ValueError('Mirror source-edge mismatch')
            lineage[a] = {'donors': [a, b], 'weights': [1 - alpha, alpha]}
            w[a] = np.asarray((1 - alpha) * v[a] + alpha * v[b], dtype=np.float32)
    affected = sorted({fi for i in lineage for fi in inc[i]})
    if not all((regions[fi] == 1 and len(faces[fi]) == 4 for fi in affected)):
        raise ValueError('Affected face ownership changed')
    newturn = nasal_rear_turns(w[fullpath])
    newrim = nasal_rear_turns(w[boundary_path])
    if max(newturn - oldturn) > 1e-05 or max(newrim - oldrim) > 1.0 or newrim.sum() > oldrim.sum():
        raise ValueError('Quantized turn checks failed')
    return {'vertices': w, 'lineage': lineage, 'affected_faces': affected, 'positive_move_mm': best.x.tolist(), 'roles': {'moved': ids.tolist(), 'donors': donors.tolist(), 'flow_with_return': fullpath, 'rim_path': boundary_path}, 'old_turns': oldturn.tolist(), 'new_turns': newturn.tolist(), 'old_rim_turns': oldrim.tolist(), 'new_rim_turns': newrim.tolist(), 'max_turn_increase_deg': float(max(newturn - oldturn)), 'accept_turn_gate': True, 'objective': float(best.fun), 'scope': 'Three rear rail stations plus one generated posterior rim station per side; no retile or new density'}

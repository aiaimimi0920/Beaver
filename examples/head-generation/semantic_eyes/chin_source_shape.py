"""Sampled bidirectional export checks and bounded current-source correction.

This numerical helper never mutates caller inputs; the native adapter owns edits.
Dense sampled checks are not a continuous-surface error certificate.
"""
import numpy as np
from scipy.spatial import cKDTree
from scipy.optimize import minimize
from collections import defaultdict

def chin_shape_closest_triangles(tr, p):
    a, b, c = (tr[:, 0], tr[:, 1], tr[:, 2])
    ab = b - a
    ac = c - a
    ap = p - a
    aa = np.sum(ab * ab, 1)
    bb = np.sum(ab * ac, 1)
    cc = np.sum(ac * ac, 1)
    da = np.sum(ap * ab, 1)
    dc = np.sum(ap * ac, 1)
    den = aa * cc - bb * bb
    u = (da * cc - dc * bb) / den
    v = (dc * aa - da * bb) / den
    inside = (u >= 0) & (v >= 0) & (u + v <= 1)
    plane = a + u[:, None] * ab + v[:, None] * ac
    candidates = []
    for x, y in [(a, b), (b, c), (c, a)]:
        d = y - x
        t = np.clip(np.sum((p - x) * d, 1) / np.sum(d * d, 1), 0, 1)
        candidates.append(x + t[:, None] * d)
    hit = np.stack(candidates, 1)
    dist = np.linalg.norm(hit - p[:, None], axis=2)
    best = hit[np.arange(len(tr)), dist.argmin(1)]
    best[inside] = plane[inside]
    return best

def chin_shape_tessellate(v, q, n):
    w = np.array([[(1 - u) * (1 - t), u * (1 - t), u * t, (1 - u) * t] for u in np.linspace(0, 1, n + 1) for t in np.linspace(0, 1, n + 1)])
    p = np.einsum('ij,kjl->kil', w, v[np.asarray(q)]).reshape(-1, 3)
    ts = []
    for f in range(len(q)):
        for i in range(n):
            for j in range(n):
                a = f * (n + 1) ** 2 + i * (n + 1) + j
                b = a + n + 1
                ts.extend([[a, b, b + 1], [a, b + 1, a + 1]])
    return (p, np.array(ts))

class ChinFrozenDistance:

    def __init__(self, v, q, n=16):
        p, t = chin_shape_tessellate(v, q, n)
        self.points = p
        self.ts = t
        self.tri = p[t]
        self.tree = cKDTree(p)
        inc = [[] for _ in p]
        for i, face in enumerate(t):
            for j in face:
                inc[j].append(i)
        self.inc = np.full((len(p), max(map(len, inc))), -1, dtype=int)
        for i, js in enumerate(inc):
            self.inc[i, :len(js)] = js

    def distance(self, pts, details=False):
        out = []
        bestids = []
        hits = []
        for a in range(0, len(pts), 200):
            p = pts[a:a + 200]
            ns = self.tree.query(p, k=12)[1]
            ids = self.inc[ns].reshape(len(p), -1)
            valid = ids >= 0
            tr = self.tri[np.maximum(ids, 0)]
            pp = np.repeat(p, ids.shape[1], axis=0)
            hit = chin_shape_closest_triangles(tr.reshape(-1, 3, 3), pp).reshape(len(p), -1, 3)
            d = np.linalg.norm(hit - p[:, None], axis=2)
            d[~valid] = np.inf
            j = d.argmin(1)
            out.extend(d[np.arange(len(p)), j])
            bestids.extend(ids[np.arange(len(p)), j])
            hits.extend(hit[np.arange(len(p)), j])
        return (np.array(out), np.array(bestids), np.array(hits)) if details else np.array(out)

def chin_shape_exact_distance(p, v, ts, details=False):
    tr = v[np.asarray(ts)]
    out = []
    ids = []
    hits = []
    for a in range(0, len(p), 200):
        pp = p[a:a + 200]
        repeat = np.repeat(pp, len(tr), axis=0)
        hit = chin_shape_closest_triangles(np.tile(tr, (len(pp), 1, 1)), repeat).reshape(len(pp), len(tr), 3)
        d = np.linalg.norm(hit - pp[:, None], axis=2)
        j = d.argmin(1)
        out.extend(d[np.arange(len(pp)), j])
        ids.extend(j)
        hits.extend(hit[np.arange(len(pp)), j])
    return (np.array(out), np.array(ids), np.array(hits)) if details else np.array(out)

class ChinShapeRefinementContext:

    def __init__(self, old, oldfaces, delta, proposal, surface, selector, roles, n=32):
        self.oldfaces = oldfaces
        self.old = np.asarray(old, float)
        self.delta = np.asarray(delta, dtype=np.float32)
        self.initial = np.asarray(proposal['new_vertices'], float)
        self.q = np.array(proposal['replacement_faces'])
        self.oldq = [oldfaces[i] for i in proposal['source_face_ids']]
        self.mov = list(map(int, proposal['mov']))
        self.lineage = {int(k): v for k, v in proposal['lineage'].items()}
        self.surface = surface
        self.selector = selector
        self.roles = roles
        self.n = n
        self.oldposes = [self.old, (self.old.astype(np.float32) + self.delta).astype(float)]
        self.fields = [ChinFrozenDistance(v, self.oldq, n) for v in self.oldposes]
        self.triweights = np.array([[i / n, j / n, 1 - (i + j) / n] for i in range(n + 1) for j in range(n + 1 - i)])
        self.fixed = np.array(proposal['fixed_triples'], dtype=int)
        self.regular = np.array(proposal['regular_triples'], dtype=int)
        self.axes = [(roles[f'chin_row_{r}_station_0'], roles[f'chin_row_{r}_station_1']) for r in [1, 2, 3]]

    def geometry(self, p, lineage, stored=True):
        nd = self.delta.astype(float).copy()
        for i, r in lineage.items():
            nd[int(i)] = np.array(r['weights']) @ self.delta[np.array(r['donors'])]
        ps = np.asarray(p, dtype=np.float32).astype(float)
        dd = nd.astype(np.float32)
        posed = (ps.astype(np.float32) + dd).astype(float)
        ts = []
        selection = []
        for f in self.q:
            opts = self.selector(ps, f, posed)
            if not opts:
                raise ValueError('Invalid paired quad')
            s = max(opts, key=lambda k: opts[k])
            selection.append(s)
            ts.extend([[f[i] for i in t] for t in ([(0, 1, 2), (0, 2, 3)] if s == 0 else [(0, 1, 3), (1, 2, 3)])])
        return ([ps, posed] if stored else [p, p + nd], np.array(ts), selection)

    def audit(self, p, lineage):
        poses, ts, sel = self.geometry(p, lineage, True)
        out = {'sampling': self.n, 'selection': sel, 'cases': []}
        witness = []
        for pose, v in enumerate(poses):
            op = self.fields[pose].points
            np_ = np.einsum('ij,kjl->kil', self.triweights, v[ts]).reshape(-1, 3)
            forward, oldti, _ = self.fields[pose].distance(np_, True)
            back, newti, _ = chin_shape_exact_distance(op, v, ts, True)
            for direction, d in [('forward', forward), ('reverse', back)]:
                out['cases'].append({
                    'pose': pose,
                    'direction': direction,
                    'max_mm': float(d.max() * 1000),
                    'p95_mm': float(np.quantile(d, 0.95) * 1000),
                    'rms_mm': float(np.sqrt(np.mean(d * d)) * 1000),
                })
                per = {}
                for k in np.flatnonzero(d > 0.0009):
                    face = k // (2 * len(self.triweights)) if direction == 'forward' else k // (self.n + 1) ** 2
                    if face not in per or d[k] > d[per[face]]:
                        per[face] = int(k)
                for face, k in per.items():
                    if direction == 'forward':
                        ti = k // len(self.triweights)
                        witness.append({
                            'pose': pose,
                            'direction': direction,
                            'quad': ti // 2,
                            'half': ti % 2,
                            'bary': self.triweights[k % len(self.triweights)].tolist(),
                            'distance_mm': float(d[k] * 1000),
                            'vertices': self.q[ti // 2].tolist(),
                        })
                    else:
                        witness.append({
                            'pose': pose,
                            'direction': direction,
                            'point': op[k].tolist(),
                            'source_quad': int(face),
                            'closest_triangle': int(newti[k]),
                            'distance_mm': float(d[k] * 1000),
                            'vertices': ts[newti[k]].tolist(),
                        })
        out['pass'] = all((r['max_mm'] <= 1 for r in out['cases']))
        out['witnesses'] = witness
        return out

    def distances(self, p, lineage, witness):
        poses, ts, sel = self.geometry(p, lineage, False)
        ans = []
        for w in witness:
            pose = w['pose']
            v = poses[pose]
            if w['direction'] == 'forward':
                tri = ts[w['quad'] * 2 + w['half']]
                point = np.array(w['bary']) @ v[tri]
                ans.append(self.fields[pose].distance(point[None])[0])
            else:
                ans.append(chin_shape_exact_distance(np.array(w['point'])[None], v, ts)[0])
        return np.array(ans)

def chin_shape_angles(v, tr):
    q = v[tr]
    a = q[:, 1] - q[:, 0]
    b = q[:, 2] - q[:, 1]
    return np.degrees(np.arccos(np.clip(np.sum(a * b, 1) / (np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1)), -1, 1)))

def chin_shape_axis_angles(v, axes):
    d = np.array([v[b] - v[a] for a, b in axes])
    return np.degrees(np.arccos(np.clip((d[:, 0] ** 2 - d[:, 1] ** 2 - d[:, 2] ** 2) / np.sum(d * d, 1), -1, 1)))

def chin_refine_selected_export(context, proposal, progress=None):
    V = context if context.n >= 32 else ChinShapeRefinementContext(context.old, context.oldfaces, context.delta, proposal, context.surface, context.selector, context.roles, n=32)
    initial = V.audit(V.initial, V.lineage)
    if initial['pass']:
        return (proposal, {
            'changed': False,
            'initial': initial,
            'final': initial,
            'verification_sampling': V.n,
            'rule': 'No-op: original current proposal already meets1mm dense paired-bidirectional gate.',
        })
    C = context if context.n == 16 else ChinShapeRefinementContext(context.old, context.oldfaces, context.delta, proposal, context.surface, context.selector, context.roles, n=16)
    work_initial = C.audit(C.initial, C.lineage)
    current = C.initial.copy()
    lineage = dict(C.lineage)
    witness = list(initial['witnesses'] if work_initial['pass'] else work_initial['witnesses'])
    history = []
    for iteration in range(4):
        active = sorted(set(C.mov) & {i for w in witness for i in w['vertices']})
        scale = 0.001
        oldfixed = chin_shape_angles(C.old, C.fixed)
        oldaxis = chin_shape_axis_angles(C.old, C.axes)
        origregular = chin_shape_angles(C.initial, C.regular)
        qs = current[C.q]
        ee = np.roll(qs[:, :, [0, 2]], -1, 1) - qs[:, :, [0, 2]]
        area = np.maximum(abs(ee[:, :, 0] * np.roll(ee[:, :, 1], -1, 1) - ee[:, :, 1] * np.roll(ee[:, :, 0], -1, 1)).mean(1), 1e-08)
        cache = {}

        def values(x):
            key = tuple(x)
            if key in cache:
                return cache[key]
            p = current.copy()
            lin = dict(lineage)
            try:
                for i, xy in zip(active, current[active][:, [0, 2]] + x.reshape(-1, 2) * scale):
                    ds, w, pt = C.surface.lineage(*xy)
                    p[i] = pt
                    lin[i] = {'donors': list(ds), 'weights': list(w)}
            except ValueError:
                return None
            cache[key] = (p, lin)
            return (p, lin)

        def objective(x):
            val = values(x)
            if val is None:
                return 1000000000.0
            p, _ = val
            return float(np.sum(((p[C.mov] - C.initial[C.mov]) / 0.0001) ** 2))

        def constraints_at(p, lin):
            q = p[C.q]
            ep = np.roll(q[:, :, [0, 2]], -1, 1) - q[:, :, [0, 2]]
            cross = (ep[:, :, 0] * np.roll(ep[:, :, 1], -1, 1) - ep[:, :, 1] * np.roll(ep[:, :, 0], -1, 1)) / area[:, None]
            ns = np.array([np.cross(q[:, b] - q[:, a], q[:, c] - q[:, a]) for a, b, c in [(0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0)]])
            nl = np.linalg.norm(ns, axis=2)
            co = np.array([np.sum(ns[a] * ns[b], 1) / (nl[a] * nl[b]) for a, b in [(0, 1), (2, 3)]])
            try:
                dist = C.distances(p, lin, witness)
            except ValueError:
                dist = np.full(len(witness), 1.0)
            return np.concatenate([
                (cross - 0.0001).ravel(),
                (co - 0.1001).ravel(),
                (0.35 - (chin_shape_angles(p, C.fixed) - oldfixed)) / 5,
                (0.35 - (chin_shape_axis_angles(p, C.axes) - oldaxis)) / 5,
                (0.75 - (chin_shape_angles(p, C.regular) - origregular)) / 5,
                (0.00095 - dist) / 0.001,
            ])

        def constraints(x):
            val = values(x)
            if val is None:
                return np.full(4 * len(C.q) + 2 * len(C.q) + len(C.fixed) + len(C.axes) + len(C.regular) + len(witness), -10000.0)
            return constraints_at(*val)
        result = minimize(
            objective,
            np.zeros(len(active) * 2),
            method='SLSQP',
            bounds=[(-0.5, 0.5)] * (len(active) * 2),
            constraints=[{'type': 'ineq', 'fun': constraints}],
            options={'maxiter': 90, 'ftol': 1e-09, 'disp': False},
        )
        val = values(result.x)
        if val is None:
            raise ValueError('Refinement left frozen source domain')
        p, lin = val
        aud = C.audit(p, lin)
        row = {
            'iteration': iteration,
            'active': active,
            'success': bool(result.success),
            'message': str(result.message),
            'nfev': result.nfev,
            'objective': float(result.fun),
            'min_constraint': float(min(constraints(result.x))),
            'audit': aud,
        }
        history.append(row)
        if progress:
            progress(row)
        if aud['pass'] and min(constraints(result.x)) >= -1e-05:
            for i in C.mov:
                if np.linalg.norm(p[i] - C.initial[i]) < 1e-09:
                    p[i] = C.initial[i]
                    lin[i] = C.lineage[i]
            postprune_min = float(min(constraints_at(p, lin)))
            if postprune_min < -1e-05:
                raise ValueError('Final constraints failed after numerical-noop pruning')
            aud = V.audit(p, lin)
            row['dense_verification'] = aud
            if not aud['pass']:
                current = p
                lineage = lin
                witness += aud['witnesses']
                continue
            q = p[C.q]
            ee = np.roll(q, -1, 1) - q
            ll = np.linalg.norm(ee, axis=2)
            co = np.sum(-np.roll(ee, 1, 1) * ee, 2) / (np.roll(ll, 1, 1) * ll)
            qa = np.degrees(np.arccos(np.clip(co, -1, 1)))
            xy = q[:, :, [0, 2]]
            de = np.roll(xy, -1, 1) - xy
            cross = de[:, :, 0] * np.roll(de[:, :, 1], -1, 1) - de[:, :, 1] * np.roll(de[:, :, 0], -1, 1)
            out = dict(proposal)
            out['new_vertices'] = p.tolist()
            out['lineage'] = {int(i): r for i, r in lin.items()}
            out['regular_turn_deg'] = chin_shape_angles(p, C.regular).tolist()
            out['quality'] = {
                'min_angle': float(qa.min()),
                'max_angle': float(qa.max()),
                'max_ratio': float((ll.max(1) / ll.min(1)).max()),
                'nonconvex': int(sum(~np.all(cross > 1e-12, axis=1))),
                'min_cross': float(abs(cross).min()),
            }
            out['selection_final'] = aud['selection']
            if 'selection_initial' in out:
                out['selection_changes'] = sum((a != b for a, b in zip(out['selection_initial'], out['selection_final'])))
            return (out, {
                'changed': True,
                'initial': initial,
                'optimization_initial': work_initial,
                'final': aud,
                'optimization_sampling': 16,
                'verification_sampling': V.n,
                'postprune_min_constraint': postprune_min,
                'iterations': history,
                'changed_roles': [i for i in C.mov if not np.array_equal(p[i], C.initial[i])],
                'maximum_adjustment_mm': float(np.linalg.norm(p - C.initial, axis=1).max() * 1000),
            })
        current = p
        lineage = lin
        witness += aud['witnesses']
    raise ValueError('Shape refinement failed existing1mm/flow/interface gates')

"""Distribute guide stations and relax adjoining supports in the current source chart."""
from collections import defaultdict, Counter
import numpy as np
from scipy.sparse import lil_matrix
from scipy.sparse.linalg import spsolve
from scipy.ndimage import gaussian_filter1d
from nasal_source_geometry import AuthorQuadSurface
from forehead_flow_rule import forehead_chart
from orbital_support_rule import orbital_quad_quality
from forehead_ear_route import extend_to_fitted_ear

def forehead_resample_flow(points, fractions):
    length = np.r_[0, np.cumsum(np.linalg.norm(np.diff(points, axis=0), axis=1))]
    return np.column_stack([np.interp(fractions * length[-1], length, points[:, k]) for k in range(points.shape[1])])

def redistribute_forehead_guides(old, of, reg, report, features, base):
    old = np.asarray(old, float)
    radius = np.median(np.linalg.norm(old[:, :2], axis=1))
    T = base['topology']
    faces = T['faces']
    vv = np.asarray(base['vertices'], float)
    chart = forehead_chart(vv)
    chart[:, 0] *= radius / np.median(np.linalg.norm(vv[:, :2], axis=1))
    paths = {name: list(path) for name, path in T['paths'].items()}
    paths['green'] = extend_to_fitted_ear(vv, faces, [reg[i] for i in T['face_owners']], report, paths['green'])
    sigma, shift = (20, 0.012)
    adj = defaultdict(set)
    inc = defaultdict(list)
    edges = Counter()
    for j, q in enumerate(faces):
        for i in q:
            inc[i].append(j)
        for a, b in zip(q, q[1:] + q[:1]):
            adj[a].add(b)
            adj[b].add(a)
            edges[tuple(sorted((a, b)))] += 1
    pins = set(features['protected']) | {i for e, c in edges.items() if c == 1 for i in e} | {i for p in paths.values() for i in (p[0], p[-1])}
    pins |= {i for i, js in inc.items() if any((len(faces[j]) != 4 or reg[T['face_owners'][j]] != 1 for j in js))}
    guide = {i for p in paths.values() for i in p}
    moving = {i for i in (guide | set(range(len(old), len(vv)))) - pins if vv[i, 2] > 1.595}
    for _ in range(2):
        moving |= {j for i in moving for j in adj[i] if j not in pins and vv[j, 0] > 1e-08 and (vv[j, 2] > 1.595)}
    support = sorted(moving - guide)
    ids = sorted(moving)
    si = {i: k for k, i in enumerate(support)}
    L = lil_matrix((len(support), len(support)))
    weights = {}
    for i in support:
        ns = list(adj[i])
        ws = 1 / np.maximum(np.linalg.norm(chart[ns][:, [0, 2]] - chart[i, [0, 2]], axis=1), 0.002)
        ws /= ws.sum()
        weights[i] = list(zip(ns, ws))
        L[si[i], si[i]] = 1
        for j, w in weights[i]:
            if j in si:
                L[si[i], si[j]] = -w
    L = L.tocsr()
    sf = [q for q, o in zip(of, reg) if o == 1 and len(q) == 4 and (min(old[q, 0]) >= -1e-08)]
    surface = AuthorQuadSurface(forehead_chart(old), sf, [1] * len(sf))
    surface.vertices = old
    oldids = sorted(set(T['retiled_old_faces']) | {j for j, q in enumerate(of) if moving.intersection(q)})
    newids = [j for j, q in enumerate(faces) if moving.intersection(q) or T['face_owners'][j] in T['retiled_old_faces']]
    mouth = float(old[list(features['oral']), 2].mean())
    print('domain', len(moving), len(support), len(oldids), len(newids), flush=True)
    target = chart.copy()
    for name, path in paths.items():
        if name in ['red', 'orange']:
            target[path] = np.asarray(base['target_chart'])[path] if 'target_chart' in base else forehead_chart(np.asarray(base['vertices']))[path]
            continue
        q = chart[path][:, [0, 2]]
        length = np.r_[0, np.cumsum(np.linalg.norm(np.diff(q, axis=0), axis=1))]
        N = int(length[-1] * 1000) + 1
        dense = forehead_resample_flow(q, np.linspace(0, 1, N))
        smooth = gaussian_filter1d(dense, sigma, axis=0, mode='nearest')
        t = np.linspace(0, 1, N)[:, None]
        smooth += (q[0] - smooth[0]) * (1 - t) + (q[-1] - smooth[-1]) * t
        fractions = length / length[-1]
        fractions = 0.65 * fractions + 0.35 * np.linspace(0, 1, len(path))
        new = forehead_resample_flow(smooth, fractions)
        new[:, 0] += shift * np.sin(np.pi * fractions)
        fade = np.clip((q[:, 1] - (mouth + 0.035)) / 0.035, 0, 1)
        fade = fade * fade * (3 - 2 * fade)
        new = q + (new - q) * fade[:, None]
        new[0] = q[0]
        new[-1] = q[-1]
        target[path, 0] = new[:, 0]
        target[path, 2] = new[:, 1]
    target[list(pins)] = chart[list(pins)]
    delta = target - chart
    rhs = np.zeros((len(support), 3))
    for i in support:
        for j, w in weights[i]:
            if j not in si:
                rhs[si[i]] += w * delta[j]
    delta[support] = spsolve(L, rhs)
    target = chart + delta
    nv = vv.copy()
    lineage = {int(i): row for i, row in base['lineage'].items()}
    ok = True
    for i in ids:
        try:
            donors, ws, pt = surface.lineage(target[i, 0], target[i, 2])
        except ValueError as err:
            ok = False
            print(str(err), i, vv[i], target[i], flush=True)
            break
        nv[i] = np.float32(pt)
        lineage[i] = {'donors': list(donors), 'weights': list(ws)}
    if not ok:
        raise ValueError('Redistributed guide left frozen author surface')
    qs = np.asarray([faces[j] for j in newids])
    ang, rat = orbital_quad_quality(nv, qs)
    S = dict(base, vertices=nv, faces=faces, lineage=lineage, paths=paths, moving=ids, support=support, protected=sorted(pins), affected_old=oldids, affected_new=newids, source_faces=sf, source_chart=forehead_chart(old), target_chart=target, topology=T)
    S['affected_old'] = sorted(set(S['affected_old']) | set(base['affected_old']))
    S['affected_new'] = sorted(set(S['affected_new']) | set(base['affected_new']))
    return S

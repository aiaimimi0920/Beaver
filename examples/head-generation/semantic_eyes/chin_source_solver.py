"""Deterministic current-source geometry proposal, without native mesh mutation.

Recompute harmonic initialization, quality/port fitting and selected-export fitting
from this build. Source points and field lineage stay frozen throughout the solve.
Native topology/fields/shape/export validation is mandatory before acceptance.
"""
import time
from collections import defaultdict, Counter
import numpy as np
from scipy.optimize import least_squares
from scipy.spatial import cKDTree
from chin_source_template import chin_retile_recipe


def chin_propose_retile(vertices, full_faces, role_ids, mouth_open_delta, *,
                        author_surface, author_faces, polygon_diagonals,
                        storage_height_offset_m=0.0, progress=None):
    """Return one deterministic frozen-source proposal without mutating inputs.

    vertices: current coordinates, x width / y negative-depth / z height.
    full_faces: actual full adjacency (including chin underside interfaces).
    role_ids: semantic strings above to CURRENT vertex IDs, never historic IDs.
    author_surface.lineage(x,z): immutable current-source bilinear sampler.
    author_faces: original positive-X skin quads for surface-distance evaluation.
    mouth_open_delta: frozen current per-point vector field, same coordinate axes.
    polygon_diagonals: existing shared neutral/posed selector callback.
    storage_height_offset_m: only for preassembly CPU replay; native input uses0.

    The .006m optimizer search radius and warmup .008m migration preference are
    numerical initialization/search controls, NOT allowable surface deformation.
    Gates remain exact pins, valid quads/ports, donor ownership, actual <=1mm
    bidirectional source/export checks, and native functional-field/pose checks.
    No generic field application is performed here.
    """
    template = chin_retile_recipe()
    CHIN_OLD_ROLE_FACES = template['old_faces']
    CHIN_NEW_ROLE_FACES = template['replacement_faces']
    CHIN_MOVABLE_ROLES = template['movable_roles']
    V = np.asarray(vertices, float).copy()
    oldfull = [list(map(int, f)) for f in full_faces]
    author_faces = [list(map(int, f)) for f in author_faces]
    delta = np.asarray(mouth_open_delta, dtype=np.float32).copy()
    if delta.shape != V.shape:
        raise ValueError('MouthOpenDelta shape mismatch')
    role_ids = {k: int(v) for k, v in role_ids.items()}
    oldpatch = [[role_ids[r] for r in f] for f in CHIN_OLD_ROLE_FACES]
    allnew = [[role_ids[r] for r in f] for f in CHIN_NEW_ROLE_FACES]
    allmov = [role_ids[r] for r in CHIN_MOVABLE_ROLES]
    if len(set(allmov)) != 25:
        raise ValueError('Expected 25 distinct movable semantic roles')
    bykey = {frozenset(f): i for i, f in enumerate(oldfull)}
    if len(bykey) != len(oldfull):
        raise ValueError('Duplicate source face')
    ids = [bykey[frozenset(f)] for f in oldpatch]
    if len(set(ids)) != 37:
        raise ValueError('Expected 37 original semantic quads')
    for f, i in zip(oldpatch, ids):
        old = oldfull[i]
        if not any((f == old[k:] + old[:k] for k in range(4))):
            raise ValueError('Semantic face winding mismatch')
    surface = author_surface

    def graph(fs):
        ad = defaultdict(set)
        inc = defaultdict(list)
        for j, f in enumerate(fs):
            if any((V[i, 0] < -1e-10 for i in f)):
                continue
            for k, a in enumerate(f):
                ad[a].update([f[k - 1], f[(k + 1) % len(f)]])
                inc[a].append(j)
        return (ad, inc)

    def opposite(fs, ad, inc, b, a):
        return ad[b] - {a} - {x for fi in inc[b] if a in fs[fi] for x in fs[fi]}

    def cos_triples(v, t):
        p = v[t]
        a = p[:, 1] - p[:, 0]
        b = p[:, 2] - p[:, 1]
        return np.sum(a * b, 1) / np.maximum(np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1), 1e-15)

    def quality(v, fs):
        q = v[np.array(fs)]
        e = np.roll(q, -1, 1) - q
        l = np.linalg.norm(e, axis=2)
        co = np.sum(-np.roll(e, 1, 1) * e, 2) / (np.roll(l, 1, 1) * l)
        angles = np.degrees(np.arccos(np.clip(co, -1, 1)))
        p = q[:, :, [0, 2]]
        d = np.roll(p, -1, 1) - p
        cross = d[:, :, 0] * np.roll(d[:, :, 1], -1, 1) - d[:, :, 1] * np.roll(d[:, :, 0], -1, 1)
        return dict(
            min_angle=float(angles.min()),
            max_angle=float(angles.max()),
            max_ratio=float((l.max(1) / l.min(1)).max()),
            nonconvex=int(sum(~(np.all(cross > 1e-12, 1) | np.all(cross < -1e-12, 1)))),
            min_cross=float(abs(cross).min()),
        )

    def tessellate(v, faces, n=12):
        pts = []
        tris = []
        for f in faces:
            q = v[f]
            off = len(pts)
            pts.extend((np.array([(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w]) @ q for u in np.linspace(0, 1, n + 1) for w in np.linspace(0, 1, n + 1)))
            for i in range(n):
                for j in range(n):
                    a = off + i * (n + 1) + j
                    b = a + n + 1
                    tris.extend([[a, b, b + 1], [a, b + 1, a + 1]])
        return (np.array(pts), np.array(tris))

    def closest_triangles(tr, p):
        a = tr[:, 0]
        b = tr[:, 1]
        c = tr[:, 2]
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
        plane = a + u[:, None] * ab + v[:, None] * ac
        inside = (u >= 0) & (v >= 0) & (u + v <= 1)
        candidates = []
        for x, y in [(a, b), (b, c), (c, a)]:
            d = y - x
            t = np.clip(np.sum((p - x) * d, 1) / np.sum(d * d, 1), 0, 1)
            candidates.append(x + t[:, None] * d)
        candidates = np.stack(candidates, 1)
        dist = np.linalg.norm(candidates - p[:, None], axis=2)
        best = candidates[np.arange(len(tr)), dist.argmin(1)]
        best[inside] = plane[inside]
        return best
    FS = allnew[:35]
    Q = np.asarray(FS)
    mov = allmov[:-1]
    full = [list(f) for f in oldfull]
    for i, f in zip(ids, allnew):
        full[i] = f
    counts = Counter((tuple(sorted((a, b))) for f in oldpatch[:35] for a, b in zip(f, f[1:] + f[:1])))
    bd = sorted({i for e, n in counts.items() if n == 1 for i in e})
    bedges = np.array([e for e, n in counts.items() if n == 1])
    bp = V[bedges][:, :, [0, 2]]

    def project_xy(p):
        try:
            d, w, q = surface.lineage(*p)
            return (np.asarray(q), (d, w), 0.0)
        except ValueError:
            a = bp[:, 0]
            b = bp[:, 1]
            z = b - a
            t = np.clip(np.sum((p - a) * z, 1) / np.sum(z * z, 1), 0, 1)
            xy = a + t[:, None] * z
            i = np.argmin(np.linalg.norm(xy - p, axis=1))
            d, w, q = surface.lineage(*xy[i])
            return (np.asarray(q), (d, w), float(np.linalg.norm(xy[i] - p)))
    ad, _ = graph(FS)
    mi = {i: k for k, i in enumerate(mov)}
    mat = np.zeros((24, 24))
    rhs = np.zeros((24, 2))
    for i, k in mi.items():
        mat[k, k] = len(ad[i])
        for j in ad[i]:
            if j in mi:
                mat[k, mi[j]] -= 1
            else:
                rhs[k] += V[j, [0, 2]]
    xy = np.linalg.solve(mat, rhs)
    initial = V.copy()
    for i, p in zip(mov, xy):
        initial[i] = surface.lineage(*p)[2]
    if progress:
        progress('harmonic', initial, {})
    ad0, inc0 = graph(oldfull)
    ports = []
    for b in bd:
        if len(ad0[b]) != 4:
            continue
        ins = [i for i in ad0[b] if i in mov]
        if len(ins) != 1:
            continue
        a = ins[0]
        op = opposite(oldfull, ad0, inc0, b, a)
        if len(op) == 1:
            ports.append([a, b, next(iter(op))])
    ports = np.array(ports)
    axes35 = [(role_ids['chin_row_1_station_0'], role_ids['chin_row_1_station_1']), (role_ids['chin_row_2_station_0'], role_ids['chin_row_2_station_1'])]

    def axiscos_for(v, axes):
        d = np.array([v[b] - v[a] for a, b in axes])
        return (d[:, 0] ** 2 - d[:, 1] ** 2 - d[:, 2] ** 2) / np.sum(d * d, 1)

    def axiscos(v):
        return axiscos_for(v, axes35)
    oldportcos = cos_triples(V, ports)
    oldportturn = np.arccos(np.clip(oldportcos, -1, 1))
    portcoslimit = np.cos(oldportturn + np.radians(0.5))
    oldaxis = axiscos(V)
    axislimit = np.cos(np.arccos(np.clip(oldaxis, -1, 1)) + np.radians(0.5))
    sp = []
    sn = []
    for f in author_faces:
        q = V[f]
        for u in np.linspace(0, 1, 9):
            for w in np.linspace(0, 1, 9):
                sp.append(np.array([(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w]) @ q)
                du = (1 - w) * (q[1] - q[0]) + w * (q[2] - q[3])
                dv = (1 - u) * (q[3] - q[0]) + u * (q[2] - q[1])
                n = np.cross(du, dv)
                sn.append(n / max(np.linalg.norm(n), 1e-20))
    sp = np.array(sp)
    sn = np.array(sn)
    tree = cKDTree(sp)
    weights = np.array([[(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w] for u in [0.2, 0.4, 0.6, 0.8] for w in [0.2, 0.4, 0.6, 0.8]])

    def fit35(initial, strict=False):
        ad, inc = graph(full)
        trip = []
        for b in mov:
            if len(ad[b]) != 4:
                continue
            for a in ad[b]:
                op = opposite(full, ad, inc, b, a)
                if len(op) == 1:
                    c = next(iter(op))
                    if a < c:
                        trip.append([a, b, c])
        trip = np.array(trip)
        q0 = V[Q]
        p0 = q0[:, :, [0, 2]]
        d0 = np.roll(p0, -1, 1) - p0
        area = np.maximum(abs(d0[:, :, 0] * np.roll(d0[:, :, 1], -1, 1) - d0[:, :, 1] * np.roll(d0[:, :, 0], -1, 1)).mean(1), 1e-06)
        baseline_sign = np.sign(np.mean([np.cross((V[f[1]] - V[f[0]])[[0, 2]], (V[f[2]] - V[f[1]])[[0, 2]]) for f in oldpatch[:35]]))
        x0 = ((initial[mov][:, [0, 2]] - V[mov][:, [0, 2]]) / 0.006).ravel()
        x0 = np.clip(x0, -0.999, 0.999)
        cache = {}
        calls = 0

        def values(x):
            nonlocal calls
            key = tuple(x)
            if key in cache:
                return cache[key]
            p = V.copy()
            lineage = {}
            outs = []
            xy = V[mov][:, [0, 2]] + x.reshape(-1, 2) * 0.006
            for i, coord in zip(mov, xy):
                pp, (d, w), out = project_xy(coord)
                p[i] = pp
                lineage[i] = dict(donors=d, weights=w)
                outs.append(out)
            cache[key] = (p, lineage, np.array(outs))
            calls += 1
            if len(cache) > 3000:
                cache.clear()
            return (p, lineage, np.array(outs))

        def residual(x):
            p, _, outside = values(x)
            q = p[Q]
            d = np.roll(q, -1, 1) - q
            l = np.maximum(np.linalg.norm(d, axis=2), 1e-12)
            co = np.sum(-np.roll(d, 1, 1) * d, 2) / (np.roll(l, 1, 1) * l)
            ratio = l.max(1) / l.min(1)
            pp = q[:, :, [0, 2]]
            ee = np.roll(pp, -1, 1) - pp
            cr = ee[:, :, 0] * np.roll(ee[:, :, 1], -1, 1) - ee[:, :, 1] * np.roll(ee[:, :, 0], -1, 1)
            migration = np.linalg.norm(p[mov] - V[mov], axis=1)
            rr = [
                np.maximum(abs(co) - np.cos(np.radians(25)), 0).ravel() * 20,
                np.maximum(np.log(ratio / 5), 0) * 3,
                np.minimum(cr * baseline_sign / area[:, None] - 0.02, 0).ravel() * 50,
                outside / 0.001 * 30,
                (p[mov] - V[mov]).ravel() / 0.008 * 0.3,
            ]
            if not strict:
                rr.insert(3, np.maximum(migration / 0.008 - 1, 0) * 30)
            pc = cos_triples(p, ports)
            if strict:
                rr.append(np.maximum(np.arccos(np.clip(pc, -1, 1)) - oldportturn, 0) / np.radians(0.5) * 10)
                rr.append(np.maximum(np.arccos(np.clip(axiscos(p), -1, 1)) - np.arccos(np.clip(oldaxis, -1, 1)), 0) / np.radians(0.5) * 10)
            else:
                rr.append(np.maximum(portcoslimit - pc, 0) * 100)
                rr.append(np.maximum(axislimit - axiscos(p), 0) * 100)
            if len(trip):
                tc = cos_triples(p, trip)
                rr.append(np.maximum(np.cos(np.radians(30)) - tc, 0) * 3)
                qp = p[trip]
                la = np.linalg.norm(qp[:, 1] - qp[:, 0], axis=1)
                lb = np.linalg.norm(qp[:, 2] - qp[:, 1], axis=1)
                rr.append(np.maximum(np.abs(np.log(la / lb)) - np.log(2), 0) * 1.5)
            tests = np.einsum('ij,kjl->kil', weights, q).reshape(-1, 3)
            near = tree.query(tests, k=3)[1]
            plane = np.abs(np.sum((tests[:, None] - sp[near]) * sn[near], 2)).min(1)
            rr.append(np.maximum(plane / 0.0005 - 1, 0) * 4)
            return np.concatenate(rr)
        start = time.time()
        r = least_squares(residual, x0, bounds=(-np.ones(len(x0)), np.ones(len(x0))), max_nfev=120, ftol=1e-07, xtol=1e-08, verbose=0)
        p, lineage, outside = values(r.x)
        actualport = np.degrees(np.arccos(np.clip(cos_triples(p, ports), -1, 1)))
        oldpt = np.degrees(oldportturn)
        return (p, dict(success=bool(r.success), nfev=r.nfev, cost=float(r.cost)))
    initial, warmup = fit35(initial, False)
    if progress:
        progress('warmup', initial, warmup)
    initial, strict = fit35(initial, True)
    if progress:
        progress('strict', initial, strict)
    mov = allmov
    FS = allnew
    Q = np.array(FS)
    ad, inc = graph(full)
    tr = []
    fixed = []
    for b in ad:
        if len(ad[b]) != 4:
            continue
        for a in ad[b]:
            op = opposite(full, ad, inc, b, a)
            if len(op) != 1:
                continue
            c = next(iter(op))
            if a >= c:
                continue
            if b in mov:
                tr.append([a, b, c])
            elif a in mov or c in mov:
                fixed.append([a, b, c])
    tr = np.array(tr)
    fixed = np.array(fixed)
    oldfixed = np.arccos(np.clip(cos_triples(V, fixed), -1, 1))
    axes = axes35 + [(role_ids['chin_row_3_station_0'], role_ids['chin_row_3_station_1'])]

    def axiscos(v):
        d = np.array([v[b] - v[a] for a, b in axes])
        return (d[:, 0] ** 2 - d[:, 1] ** 2 - d[:, 2] ** 2) / np.sum(d * d, 1)
    oldaxis = np.arccos(np.clip(axiscos(V), -1, 1))
    ef = Counter((tuple(sorted((a, b))) for f in oldpatch for a, b in zip(f, f[1:] + f[:1])))
    bedges = np.array([e for e, n in ef.items() if n == 1])
    bd = sorted({i for e in bedges for i in e})
    bp = V[bedges][:, :, [0, 2]]

    def project(p):
        try:
            d, w, q = surface.lineage(*p)
            return (q, (d, w), 0.0)
        except ValueError:
            a = bp[:, 0]
            b = bp[:, 1]
            z = b - a
            t = np.clip(np.sum((p - a) * z, 1) / np.sum(z * z, 1), 0, 1)
            xy = a + t[:, None] * z
            i = np.argmin(np.linalg.norm(xy - p, axis=1))
            d, w, q = surface.lineage(*xy[i])
            return (q, (d, w), np.linalg.norm(xy[i] - p))
    nearfaces = author_faces
    sp, st = tessellate(V, nearfaces, 12)
    tri = sp[st]
    tree = cKDTree(tri.mean(1))

    def distfield(p):
        k = 20
        j = tree.query(p, k=k)[1]
        tt = tri[j].reshape(-1, 3, 3)
        pp = np.repeat(p, k, axis=0)
        hit = closest_triangles(tt, pp)
        return np.linalg.norm(hit - pp, axis=1).reshape(-1, k).min(1)
    x0 = ((initial[mov][:, [0, 2]] - V[mov][:, [0, 2]]) / 0.006).ravel()
    x0 = np.clip(x0, -0.999, 0.999)
    cache = {}

    def values(x):
        key = tuple(x)
        if key in cache:
            return cache[key]
        p = V.copy()
        lineage = {}
        outside = []
        for i, xy in zip(mov, V[mov][:, [0, 2]] + x.reshape(-1, 2) * 0.006):
            pp, (d, w), o = project(xy)
            p[i] = pp
            lineage[i] = {'donors': d, 'weights': w}
            outside.append(o)
        out = (p, lineage, np.array(outside))
        cache[key] = out
        if len(cache) > 4000:
            cache.clear()
        return out

    def getweights(p, lineage):
        nd = delta.copy()
        for i, r in lineage.items():
            nd[i] = np.array(r['weights']) @ delta[list(r['donors'])]
        ps = p.astype(np.float32)
        ps[:, 2] -= np.float32(storage_height_offset_m)
        posed = (ps + nd).astype(float)
        ps = ps.astype(float)
        ww = []
        sel = []
        for f in FS:
            e = polygon_diagonals(ps, f, posed)
            s = max(e, key=lambda k: e[k]) if e else 0
            sel.append(s)
            w = []
            for ti in [(0, 1, 2), (0, 2, 3)] if s == 0 else [(0, 1, 3), (1, 2, 3)]:
                for i in range(7):
                    for j in range(7 - i):
                        a = np.zeros(4)
                        a[list(ti)] = [i / 6, j / 6, 1 - (i + j) / 6]
                        w.append(a)
            ww.append(w)
        return (np.array(ww), sel)
    p, lin, _ = values(x0)
    weights, sel = getweights(p, lin)
    baseline_sign = 1
    pp = V[Q][:, :, [0, 2]]
    ee = np.roll(pp, -1, 1) - pp
    cr = ee[:, :, 0] * np.roll(ee[:, :, 1], -1, 1) - ee[:, :, 1] * np.roll(ee[:, :, 0], -1, 1)
    area = np.maximum(abs(cr).mean(1), 1e-06)

    def residual(x):
        p, lin, outside = values(x)
        q = p[Q]
        d = np.roll(q, -1, 1) - q
        l = np.maximum(np.linalg.norm(d, axis=2), 1e-12)
        co = np.sum(-np.roll(d, 1, 1) * d, 2) / (np.roll(l, 1, 1) * l)
        ratio = l.max(1) / l.min(1)
        pp = q[:, :, [0, 2]]
        ee = np.roll(pp, -1, 1) - pp
        cr = ee[:, :, 0] * np.roll(ee[:, :, 1], -1, 1) - ee[:, :, 1] * np.roll(ee[:, :, 0], -1, 1)
        rr = [
            np.maximum(abs(co) - np.cos(np.radians(25)), 0).ravel() * 20,
            np.maximum(np.log(ratio / 5), 0) * 3,
            np.minimum(cr / area[:, None] - 0.02, 0).ravel() * 50,
            outside / 0.001 * 30,
            (p[mov] - V[mov]).ravel() / 0.008 * 0.15,
        ]
        rr.append(np.maximum(np.arccos(np.clip(cos_triples(p, fixed), -1, 1)) - oldfixed, 0) / np.radians(0.5) * 10)
        rr.append(np.maximum(np.arccos(np.clip(axiscos(p), -1, 1)) - oldaxis, 0) / np.radians(0.5) * 10)
        turns = np.degrees(np.arccos(np.clip(cos_triples(p, tr), -1, 1)))
        rr.append(np.maximum(turns - 25, 0) / 10 * 4)
        qp = p[tr]
        la = np.linalg.norm(qp[:, 1] - qp[:, 0], axis=1)
        lb = np.linalg.norm(qp[:, 2] - qp[:, 1], axis=1)
        rr.append(np.maximum(abs(np.log(la / lb)) - np.log(1.7), 0) * 2)
        tests = np.einsum('kij,kjl->kil', weights, q).reshape(-1, 3)
        err = distfield(tests)
        rr.append(np.maximum(err / 0.0008 - 1, 0) * 12)
        return np.concatenate(rr)
    start = time.time()
    r = least_squares(residual, x0, bounds=(-np.ones(len(x0)), np.ones(len(x0))), max_nfev=130, ftol=2e-07, xtol=1e-08, verbose=0)
    p, lineage, outside = values(r.x)
    newweights, newsel = getweights(p, lineage)
    if progress:
        progress('final', p, {'success': bool(r.success), 'nfev': r.nfev, 'cost': float(r.cost)})

    def chin_oriented_cycle(face):
        face = tuple(face)
        return min((face[k:] + face[:k] for k in range(len(face))))
    original_donors = {chin_oriented_cycle(f) for f in oldpatch}
    for i, row in lineage.items():
        donors = tuple(row['donors'])
        w = np.asarray(row['weights'], float)
        if len(donors) != 4 or len(set(donors)) != 4 or chin_oriented_cycle(donors) not in original_donors:
            raise ValueError('Final donor is not an oriented original 37-patch quad')
        if w.shape != (4,) or not np.all(np.isfinite(w)) or min(w) < -1e-10 or (abs(sum(w) - 1) > 1e-09) or (abs(w[0] * w[2] - w[1] * w[3]) > 1e-08):
            raise ValueError('Invalid frozen bilinear donor weights')
        if np.linalg.norm(w @ V[list(donors)] - p[i]) > 1e-09:
            raise ValueError('Frozen donor reconstruction mismatch')
    outside_ids = sorted(set(range(len(V))) - set(mov))
    if not np.array_equal(p[outside_ids], V[outside_ids]):
        raise ValueError('Pinned point changed')
    return {
        'new_vertices': p.tolist(),
        'replacement_faces': FS,
        'source_face_ids': ids,
        'full_faces': full,
        'lineage': {int(i): {'donors': list(row['donors']), 'weights': list(row['weights'])} for i, row in lineage.items()},
        'mov': mov,
        'boundary': [int(i) for i in bd],
        'quality': quality(p, FS),
        'fixed_triples': fixed.tolist(),
        'regular_triples': tr.tolist(),
        'regular_turn_deg': np.degrees(np.arccos(np.clip(cos_triples(p, tr), -1, 1))).tolist(),
        'selection_initial': sel,
        'selection_final': newsel,
        'selection_changes': sum((a != b for a, b in zip(sel, newsel))),
        'optimizer': {'warmup': warmup, 'strict': strict, 'final': {'success': bool(r.success), 'nfev': r.nfev, 'cost': float(r.cost), 'seconds': time.time() - start}},
        'scope': 'Pure current-source geometry proposal; native field and bidirectional export/pose validation still mandatory',
    }

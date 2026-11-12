"""Experimental shared nasal rule. CPU-only; never returns deployable without external unchanged gates.
Input coordinate system is native x,depth,z-height. Uses current semantic landmarks and paths.
"""
import numpy as np
from collections import Counter, defaultdict
from scipy.spatial import cKDTree
from scipy.optimize import least_squares
from nasal_regularization_chart import ConnectedQuadChart
from nasal_source_domain import current_source_features
from nasal_source_geometry import AuthorQuadSurface

def propose_nasal_rule(vertices, quads, regions, ordinals, semantic_flow_paths):
    features = current_source_features(vertices, quads, regions, ordinals)
    span = float(features['eye_height'] - features['nose_peak'][2])
    if not 0.01 < span < 0.06 or len(semantic_flow_paths) != 5:
        raise ValueError('Unsupported anatomical/source flow profile')
    ff = [list(map(int, q)) for q in quads]
    r = np.asarray(regions)
    v = np.asarray(vertices, float)[:, [0, 2, 1]].copy()
    v[:, 2] *= -1
    original = {i: q[:] for i, q in enumerate(ff) if r[i] == 1}
    distances, mirror = cKDTree(v).query(v * [-1, 1, 1])
    if max(distances[list({x for q in original.values() for x in q})]) > 1e-07:
        raise ValueError('Current source skin is not exactly symmetric')

    def graph(f):
        edges = defaultdict(list)
        adj = defaultdict(set)
        for i, q in f.items():
            for x, y in zip(q, q[1:] + q[:1]):
                edges[tuple(sorted((x, y)))].append(i)
                adj[x].add(y)
                adj[y].add(x)
        return (edges, adj)

    def quality(q):
        p = v[q]
        xy = p[:, :2]
        e = np.roll(xy, -1, 0) - xy
        cr = np.cross(e, np.roll(e, -1, 0))
        if not (np.all(cr > 1e-12) or np.all(cr < -1e-12)):
            return None
        u = np.roll(p, 1, 0) - p
        w = np.roll(p, -1, 0) - p
        angles = np.degrees(np.arccos(np.clip(np.sum(u * w, 1) / (np.linalg.norm(u, axis=1) * np.linalg.norm(w, axis=1)), -1, 1)))
        return (float(min(angles)), float(max(angles)), float(max(np.linalg.norm(u, axis=1)) / min(np.linalg.norm(u, axis=1))))

    def options(f, target):
        edges, adj = graph(f)
        out = []
        for n in adj[target]:
            if v[n, 0] <= 1e-09:
                continue
            ids = edges[tuple(sorted((n, target)))]
            if len(ids) != 2:
                continue
            qs = [f[i] for i in ids]
            ct = Counter((tuple(sorted((x, y))) for q in qs for x, y in zip(q, q[1:] + q[:1])))
            b = [(x, y) for q in qs for x, y in zip(q, q[1:] + q[:1]) if ct[tuple(sorted((x, y)))] == 1]
            succ = dict(b)
            cy = [b[0][0]]
            while succ[cy[-1]] != cy[0]:
                cy.append(succ[cy[-1]])
            if len(cy) != 6:
                continue
            for k in range(3):
                c = cy[k:] + cy[:k]
                diag = tuple(sorted((c[0], c[3])))
                if target in diag or diag in edges:
                    continue
                new = [c[:4], [c[3], c[4], c[5], c[0]]]
                qu = [quality(q) for q in new]
                if any((z is None for z in qu)):
                    continue
                lookup = {tuple(sorted(q)): i for i, q in f.items()}
                mids = [lookup.get(tuple(sorted((int(mirror[x]) for x in q)))) for q in qs]
                if None in mids:
                    continue
                nf = {i: q[:] for i, q in f.items()}
                for i, q in zip(ids, new):
                    nf[i] = q
                for i, q in zip(mids, new):
                    nf[i] = [int(mirror[x]) for x in q[::-1]]
                _, ad = graph(nf)
                if len(ad[target]) != 4:
                    continue
                if any((abs(v[i, 0]) < 1e-09 and len(ad[i]) > len(adj[i]) for i in ad)):
                    continue
                score = min((z[0] for z in qu)) - max((z[2] for z in qu))
                out.append((score, nf, dict(target=target, removed_edge=[target, n], face_ids=ids + mids, new_faces=[nf[i] for i in ids + mids], quality=qu)))
        return sorted(out, key=lambda x: x[0], reverse=True)
    _, adj = graph(original)
    nose_height = float(features['nose_peak'][2])
    targets = sorted([i for i in features['axis'] if len(adj[i]) == 6 and 0 < (v[i, 1] - nose_height) / span < 0.7], key=lambda i: -v[i, 1])
    if len(targets) != 3:
        raise ValueError('Unsupported axis junction profile; expected three nasal 6-valence nodes')
    original_edges, _ = graph(original)
    if any((tuple(sorted((a, b))) not in original_edges for q in semantic_flow_paths for a, b in zip(q, q[1:]))):
        raise ValueError('Semantic flow is not a source-edge path')
    beam = [(0, original, [])]
    for target in targets:
        next_beam = []
        for score, mesh, hist in beam:
            for sc, new, edit in options(mesh, target):
                next_beam.append((score + sc, new, hist + [edit]))
        beam = sorted(next_beam, key=lambda x: x[0], reverse=True)[:30]
        if not beam:
            raise ValueError('No symmetric legal axis reconnection')
    _, chosen, history = beam[0]
    topology = dict(new_faces={str(i): q for i, q in chosen.items() if q != original[i]}, edits=history)
    base = v.copy()
    F = {i: q[:] for i, q in original.items()}
    C = topology
    for i, q in C['new_faces'].items():
        F[int(i)] = q
    _, adj = graph(F)
    _, oldadj = graph(original)
    paths = semantic_flow_paths
    members = [set() for _ in v]
    for q, r0 in zip(ff, r):
        for x in q:
            members[x].add(int(r0))
    protected = {i for i, x in enumerate(members) if x != {1}}
    protected |= {n for i in list(protected) for n in oldadj[i]}
    axis_patch = {x for q in C['new_faces'].values() for x in q}
    mov = {i for i in axis_patch if v[i, 0] > 1e-08}
    for q in paths:
        p = v[q]
        d = np.diff(p, axis=0)
        t = d / np.linalg.norm(d, axis=1)[:, None]
        j = 1 + int(np.argmax(np.arccos(np.clip(np.sum(t[:-1] * t[1:], 1), -1, 1))))
        chain_ids = q[max(0, j - 2):min(len(q), j + 3)]
        for node in chain_ids:
            node = int(mirror[node]) if v[node, 0] < 0 else node
            mov.add(node)
            mov.update(oldadj[node])
    mov = sorted((i for i in mov if i not in protected and v[i, 0] > 1e-08))
    mir = [int(mirror[i]) for i in mov]
    allmove = set(mov + mir)
    affected = sorted((i for i, q in F.items() if set(q) & allmove or str(i) in C['new_faces']))
    Q = np.array([F[i] for i in affected])
    q0 = base[Q]
    grad = []
    for i in mov:
        ns = list(oldadj[i])
        d = base[ns, :2] - base[i, :2]
        grad.append(np.linalg.lstsq(d, base[ns, 2] - base[i, 2], rcond=None)[0])
    grad = np.array(grad)
    caps = np.array([min(0.0008, 0.025 * span) if i in axis_patch else min(0.0015, 0.045 * span) for i in mov]) / np.sqrt(1 + np.sum(grad ** 2, axis=1))

    def points(x):
        out = base.copy()
        d = x.reshape(-1, 2) * caps[:, None]
        out[mov, :2] += d
        out[mov, 2] += np.sum(d * grad, 1)
        out[mir] = out[mov] * [-1, 1, 1]
        return out

    def residual(x):
        out = points(x)
        p = out[Q]
        u = np.roll(p, 1, 1) - p
        w = np.roll(p, -1, 1) - p
        co = np.sum(u * w, 2) / (np.linalg.norm(u, axis=2) * np.linalg.norm(w, axis=2))
        lengths = np.linalg.norm(u, axis=2)
        ratios = lengths.max(1) / lengths.min(1)
        res = [np.maximum(abs(co) - np.cos(np.radians(25)), 0).ravel() * 25, x * 0.2, np.maximum(ratios - 4, 0) * 2]
        e = np.roll(p[:, :, :2], -1, 1) - p[:, :, :2]
        cross = e[:, :, 0] * np.roll(e[:, :, 1], -1, 1) - e[:, :, 1] * np.roll(e[:, :, 0], -1, 1)
        sg = np.sign(np.sum((np.roll(q0[:, :, :2], -1, 1) - q0[:, :, :2])[:, :, 0] * np.roll((np.roll(q0[:, :, :2], -1, 1) - q0[:, :, :2])[:, :, 1], -1, 1) - (np.roll(q0[:, :, :2], -1, 1) - q0[:, :, :2])[:, :, 1] * np.roll((np.roll(q0[:, :, :2], -1, 1) - q0[:, :, :2])[:, :, 0], -1, 1), 1))
        res.append(np.minimum(cross * sg[:, None] - 1e-08, 0).ravel() * 200000.0)
        for q in paths:
            pp = out[q]
            d = np.diff(pp, axis=0)
            ll = np.linalg.norm(d, axis=1)
            tt = d / ll[:, None]
            turn = np.diff(tt, axis=0)
            res.append((np.diff(turn, axis=0) * 3).ravel())
            res.append((np.maximum(np.linalg.norm(turn, axis=1) - 0.48, 0) * 5).ravel())
        return np.concatenate(res)
    fit = least_squares(residual, np.zeros(2 * len(mov)), bounds=(-np.ones(2 * len(mov)), np.ones(2 * len(mov))), max_nfev=100, ftol=1e-06, xtol=1e-07, verbose=0)
    out = points(fit.x)
    native = base[:, [0, 2, 1]].copy()
    native[:, 1] *= -1
    near = set(allmove)
    for _ in range(2):
        near |= {j for i in list(near) for j in oldadj[i]}
    source_region = [1 if int(rr) == 1 and set(q) & near else 0 for q, rr in zip(ff, r)]
    surf = AuthorQuadSurface(native, ff, source_region)
    lineage = {}
    projection_error = []
    projection_notes = []
    chart = ConnectedQuadChart(native, ff, r)
    for i, j in zip(mov, mir):
        proposed = out[i].copy()
        item = None
        reasons = []
        for alpha in (1.0, 0.75, 0.5, 0.25, 0.125, 0.0625, 0.0):
            xy = base[i, :2] + alpha * (proposed[:2] - base[i, :2])
            try:
                item = chart.sample_from_vertex(i, xy)
                pp = item['point'][[0, 2, 1]].copy()
                pp[2] *= -1
                if np.linalg.norm(pp - base[i]) > min(0.0025, 0.075 * span):
                    item = None
                    continue
                break
            except ValueError as error:
                reasons.append(str(error))
                item = None
        if item is None:
            raise ValueError('No unique local source chart, including identity fallback')
        donors, weights = (item['donors'], item['weights'])
        projection_notes.append(dict(vertex=i, alpha=alpha, walk=item['walk'], rejected_trials=reasons))
        projection_error.append(float(np.linalg.norm(pp - proposed)))
        out[i] = pp
        out[j] = pp * [-1, 1, 1]
        lineage[i] = dict(donors=list(donors), weights=list(weights))
        lineage[j] = dict(donors=[int(mirror[z]) for z in donors], weights=list(weights))

    def qs(v, Q):
        p = v[Q]
        u = np.roll(p, 1, 1) - p
        w = np.roll(p, -1, 1) - p
        angles = np.degrees(np.arccos(np.clip(np.sum(u * w, 2) / (np.linalg.norm(u, axis=2) * np.linalg.norm(w, axis=2)), -1, 1)))
        lens = np.linalg.norm(u, axis=2)
        e = np.roll(p[:, :, :2], -1, 1) - p[:, :, :2]
        c = e[:, :, 0] * np.roll(e[:, :, 1], -1, 1) - e[:, :, 1] * np.roll(e[:, :, 0], -1, 1)
        return dict(min_angle=float(angles.min()), max_angle=float(angles.max()), max_edge_ratio=float((lens.max(1) / lens.min(1)).max()), nonconvex_front_faces=int(sum(~(np.all(c > 0, 1) | np.all(c < 0, 1)))))

    def flow(v, q):
        d = np.diff(v[q], axis=0)
        t = d / np.linalg.norm(d, axis=1)[:, None]
        ang = np.degrees(np.arccos(np.clip(np.sum(t[:-1] * t[1:], 1), -1, 1)))
        return dict(max_turn=float(max(ang)), top3_share=float(sum(sorted(ang)[-3:]) / sum(ang)), turns=ang.tolist())
    ed, ad = graph(F)
    eold, _ = graph(original)
    boundary0 = Counter((tuple(sorted((x, y))) for i in affected for q in [original[i]] for x, y in zip(q, q[1:] + q[:1])))
    boundary = sorted((e for e, n in boundary0.items() if n == 1))
    boundsverts = set((x for e in boundary for x in e))
    result = dict(scope='CPU candidate only; not validated for application, native fields or expressions', source_sha256=None, projection_notes=projection_notes, optimizer_success=bool(fit.success), evaluations=fit.nfev, moved_vertex_ids=sorted(allmove), new_vertex_positions={i: out[i].tolist() for i in sorted(allmove)}, lineage=lineage, new_faces=C['new_faces'], affected_faces=affected, outer_boundary_edges=boundary, outer_boundary_fixed=not bool(boundsverts & allmove), all_other_vertices_exact=bool(np.array_equal(out[[i for i in range(len(v)) if i not in allmove]], base[[i for i in range(len(v)) if i not in allmove]])), protected_vertices_unchanged=not bool(protected & allmove), axis_valence={i: len(ad[i]) for i in targets}, maximum_move_m=float(np.linalg.norm(out - base, axis=1).max()), max_tangent_approximation_correction_m=max(projection_error), old_quality=qs(base, np.array([original[i] for i in affected])), candidate_quality=qs(out, Q), flows=[dict(old=flow(base, q), candidate=flow(out, q)) for q in paths], warning='Source-surface vertex projection preserves sampled shape, not entire changed bilinear patch. Shape-distance, intersections, UV/fields/morphs and native visual review still required.')
    result['deployable'] = False
    result['pending_gates'] = ['original nasal patch gate remains unchanged', 'actual automatic runtime bidirectional shape gate', 'native fields and UV corner ownership', 'morph/runtime/collision replay', 'native visual review', 'cross-character generated parameter trials']
    result['semantic_selection'] = {'nose_eye_span': span, 'axis_targets': targets, 'target_normalized_heights': [(v[i, 1] - nose_height) / span for i in targets]}
    return (result, out, np.array(list(F.values())), np.array(list(F)), np.array(list(original.values())))

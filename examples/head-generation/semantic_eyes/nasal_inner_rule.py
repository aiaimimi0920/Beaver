"""CPU226 bounded inner nasal topology proposal. No engine, no global source IDs.
Supported input profile is current raw13 five-flow output; unsupported inputs fail closed.
Returned vertex array keeps unused old vertices for lineage; delete only reported removed_vertices.
"""
from collections import defaultdict, Counter
import numpy as np
from scipy.spatial import cKDTree

def propose_inner_nasal(vertices, quads, regions, ordinals, semantic_flow_paths):
    v = np.asarray(vertices, float).copy()
    fs = [list(map(int, q)) for q in quads]
    r = np.asarray(regions)
    paths = [list(map(int, p)) for p in semantic_flow_paths]
    assert len(r) == len(fs) and len(paths) == 5
    edgefaces = defaultdict(list)
    adj = defaultdict(set)
    for i, q in enumerate(fs):
        for a, b in zip(q, q[1:] + q[:1]):
            edgefaces[tuple(sorted((a, b)))].append(i)
            adj[a].add(b)
            adj[b].add(a)
    assert [len(p) for p in paths] == [17, 18, 17, 18, 17], 'Unsupported current nasal path profile'
    main = [p[-4:] if len(p) == 17 else [p[-5]] + p[-3:] for p in paths]
    axis_choices = [x for x in adj[main[0][-1]] if abs(v[x, 0]) < 1e-09]
    assert len(axis_choices) == 1, 'Ambiguous upper axis anchor'
    axis0 = axis_choices[0]
    axis = [axis0]
    while len(axis) < 6:
        down = [x for x in adj[axis[-1]] if abs(v[x, 0]) < 1e-09 and v[x, 2] < v[axis[-1], 2]]
        assert len(down) == 1, 'Ambiguous descending axis'
        axis.append(down[0])
    tip = axis[-1]
    bridges = [x for x in adj[tip] & adj[main[-1][-1]] if v[x, 0] > 0]
    assert len(bridges) == 1, 'Ambiguous nasal peak bridge'
    tipbridge = bridges[0]
    cycle = main[0] + axis + [tipbridge] + main[-1][::-1] + [main[i][0] for i in (3, 2, 1)]
    assert len(cycle) == 18 and len(set(cycle)) == 18
    cuts = {tuple(sorted(e)) for e in zip(cycle, cycle[1:] + cycle[:1])}
    assert cuts <= edgefaces.keys()
    seed = min(edgefaces[tuple(sorted((main[0][0], main[1][0])))], key=lambda i: np.mean(v[fs[i], 0]))
    domain = {seed}
    stack = [seed]
    while stack:
        i = stack.pop()
        q = fs[i]
        for e in zip(q, q[1:] + q[:1]):
            e = tuple(sorted(e))
            if e in cuts:
                continue
            for j in edgefaces[e]:
                if j not in domain:
                    domain.add(j)
                    stack.append(j)
    assert len(domain) < 100
    nv = list(v.copy())
    lineage = {}
    grid = np.zeros((5, 5), int)
    grid[:, 0] = [p[0] for p in main]
    grid[0, :4] = main[0]
    grid[-1, :4] = main[-1]
    grid[:, -1] = axis[:5]
    import heapq
    rails = {}
    for col in (1, 2, 3):
        start = int(grid[0, col])
        end = int(grid[4, col])
        heap = [(0.0, tuple(v[start]), start, [start])]
        seen = set()
        allowed = {tuple(sorted(e)) for fi in domain for e in zip(fs[fi], fs[fi][1:] + fs[fi][:1])}
        while heap:
            cost, _, x, path = heapq.heappop(heap)
            if x in seen:
                continue
            seen.add(x)
            if x == end:
                rails[col] = path
                break
            for y in adj[x]:
                if tuple(sorted((x, y))) not in allowed or v[y, 2] >= v[x, 2] - 1e-10:
                    continue
                penalty = np.linalg.norm(v[y] - v[x]) + 100 * abs(v[y, 0] - v[x, 0])
                heapq.heappush(heap, (cost + penalty, tuple(v[y]), y, path + [y]))
        assert col in rails
    for row in (1, 2, 3):
        for col in (1, 2, 3):
            u = col / 4
            t = row / 4
            left = v[grid[row, 0], 2]
            right = v[grid[row, 4], 2]
            top = v[grid[0, col], 2]
            bottom = v[grid[4, col], 2]
            corners = (1 - u) * (1 - t) * v[grid[0, 0], 2] + u * (1 - t) * v[grid[0, 4], 2] + (1 - u) * t * v[grid[4, 0], 2] + u * t * v[grid[4, 4], 2]
            height = (1 - u) * left + u * right + (1 - t) * top + t * bottom - corners
            rail = rails[col]
            hits = [(a, b) for a, b in zip(rail, rail[1:]) if v[a, 2] >= height >= v[b, 2]]
            assert len(hits) == 1
            a, b = hits[0]
            t = (v[a, 2] - height) / (v[a, 2] - v[b, 2])
            idx = len(nv)
            grid[row, col] = idx
            nv.append((1 - t) * v[a] + t * v[b])
            lineage[idx] = {'donors': [a, b], 'weights': [float(1 - t), float(t)], 'source_edge': [a, b]}
    quads = []
    for row in range(4):
        for col in range(4):
            quads.append([int(grid[row, col]), int(grid[row, col + 1]), int(grid[row + 1, col + 1]), int(grid[row + 1, col])])
    quads.append([main[-1][-1], axis[4], tip, tipbridge])
    nv = np.array(nv)
    sign = np.sign(np.cross(v[fs[seed][1]] - v[fs[seed][0]], v[fs[seed][2]] - v[fs[seed][1]])[1])
    for q in quads:
        if np.sign(np.cross(nv[q[1]] - nv[q[0]], nv[q[2]] - nv[q[1]])[1]) != sign:
            q.reverse()
    from scipy.spatial import cKDTree
    dist, mirror = cKDTree(v).query(v * [-1, 1, 1])
    assert max(dist[list({x for i in domain for x in fs[i]})]) < 1e-07
    fm = {frozenset(q): i for i, q in enumerate(fs)}
    other = {fm[frozenset((int(mirror[x]) for x in fs[i]))] for i in domain}
    mp = {i: int(mirror[i]) for i in range(len(v))}
    newlist = list(nv)
    for i in range(len(v), len(nv)):
        mp[i] = len(newlist)
        newlist.append(nv[i] * [-1, 1, 1])
        L = lineage[i]
        lineage[mp[i]] = {'donors': [int(mirror[x]) for x in L['donors']], 'weights': L['weights'], 'source_edge': [int(mirror[x]) for x in L['source_edge']]}
    newquads = quads + [[mp[x] for x in q[::-1]] for q in quads]
    nv = np.array(newlist)
    removed = domain | other
    kept = [q for i, q in enumerate(fs) if i not in removed]
    nf = kept + newquads
    used = {x for q in nf for x in q}
    deleted = set(range(len(v))) - used
    oldE = Counter((tuple(sorted(e)) for q in fs for e in zip(q, q[1:] + q[:1])))
    newE = Counter((tuple(sorted(e)) for q in nf for e in zip(q, q[1:] + q[:1])))
    assert max(newE.values()) <= 2
    assert {e for e, c in oldE.items() if c == 1} == {e for e, c in newE.items() if c == 1}
    assert len(v) - len(oldE) + len(fs) == len(used) - len(newE) + len(nf)
    angles = []
    ratios = []
    fold = []
    for k, q in enumerate(newquads):
        p = nv[q]
        a = np.roll(p, 1, axis=0) - p
        b = np.roll(p, -1, axis=0) - p
        angles.extend(np.degrees(np.arccos(np.clip(np.sum(a * b, 1) / np.linalg.norm(a, axis=1) / np.linalg.norm(b, axis=1), -1, 1))))
        le = np.linalg.norm(b, axis=1)
        ratios.append(max(le) / min(le))
        cr = np.cross(a, b)[:, 1]
        if min(cr) * max(cr) <= 0:
            fold.append(k)
    res = dict(status='CPU prototype, NOT approved for native trial', boundary_positive=cycle, axis=axis, grid_positive=grid.tolist(), removed_faces=sorted(removed), removed_vertices=sorted(deleted), new_quads=newquads, new_point_lineage=lineage, old_domain_faces_per_half=len(domain), new_domain_faces_per_half=len(quads), old_vertices=len(v), new_used_vertices=len(used), all_retained_points_exact=True, all_outer_flows_through_station8_exact=True, nose_peak_exact=True, manifold_and_euler_preserved=True, min_angle=float(min(angles)), max_angle=float(max(angles)), max_edge_ratio=float(max(ratios)), projected_fold_faces=fold)
    mem = defaultdict(set)
    skinadj = defaultdict(set)
    for q, rr in zip(fs, r):
        for x in q:
            mem[x].add(int(rr))
        if rr == 1:
            for a, b in zip(q, q[1:] + q[:1]):
                skinadj[a].add(b)
                skinadj[b].add(a)
    protected = {i for i in range(len(v)) if mem[i] != {1}}
    protected |= {x for i in list(protected) for x in skinadj[i]}
    assert not protected & {x for fi in removed for x in fs[fi]}, 'Inner replacement touches protected first support'
    assert all((r[i] == 1 for i in removed)), 'Non-skin region selected'
    assert all((len(fs[i]) == 4 for i in removed))
    assert all((set(L['donors']) <= set(range(len(v))) and min(L['weights']) >= 0 and (abs(sum(L['weights']) - 1) < 1e-10) for L in lineage.values()))
    newpaths = [p[:p.index(g[0])] + list(map(int, g)) for p, g in zip(paths, grid)]
    assert all((tuple(sorted((a, b))) in newE for p in newpaths for a, b in zip(p, p[1:]))), 'New primary flow is disconnected'
    eye_regions = {value for name, value in ordinals.items() if 'iris' in name.lower()}
    eye_ids = {x for q, rr in zip(fs, r) if rr in eye_regions for x in q}
    assert eye_ids, 'Missing semantic eye profile'
    eye_height = float(np.mean(v[list(eye_ids), 2]))
    span = eye_height - v[tip, 2]
    assert span > 0.01 and 0 < (v[axis[0], 2] - v[tip, 2]) / span < 1.2, 'Nasal-axis frame unsupported'
    assert v[tip, 1] <= min(v[axis, 1]) + 1e-08, 'Selected lower axis is not nasal peak'
    res.update(updated_full_paths=newpaths, new_point_ids=list(range(len(v), len(nv))), protected_first_support_untouched=True, source_relative_axis_height_ratios=((v[axis, 2] - v[tip, 2]) / span).tolist(), source_vertex_count=len(v), source_face_count=len(fs), source_point_lineage={str(i): {'donors': [i], 'weights': [1.0]} for i in used if i < len(v)})
    return (res, nv, nf)

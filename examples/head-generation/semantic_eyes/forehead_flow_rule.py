"""Five semantic forehead paths fair on an immutable current quad surface.

No reference-mesh coordinates are imported. The caller binds authored roles,
protects functional boundaries, and owns native field transport and export.
"""
from collections import defaultdict, Counter
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_turns, orbital_quad_quality


def forehead_chart(points):
    points = np.asarray(points, float)
    radius = np.median(np.linalg.norm(points[:, :2], axis=1))
    if not np.isfinite(radius) or radius <= 1e-6:
        raise ValueError('Invalid forehead angular chart radius')
    return np.column_stack((radius*np.arctan2(points[:, 0], -points[:, 1]),
                            np.zeros(len(points)), points[:, 2]))


def forehead_convex_chart(chart, quads):
    q = chart[np.asarray(quads), :][:, :, [0, 2]]
    edge = np.roll(q, -1, axis=1)-q
    cross = edge[:, :, 0]*np.roll(edge[:, :, 1], -1, axis=1)
    cross -= edge[:, :, 1]*np.roll(edge[:, :, 0], -1, axis=1)
    if np.any(np.min(cross, axis=1)*np.max(cross, axis=1) <= 1e-18):
        raise ValueError('Forehead source chart has folded or degenerate quads')
    return cross


def forehead_no_chart_crossings(chart, faces):
    edges = sorted({tuple(sorted((a, b))) for q in faces
                    for a, b in zip(q, q[1:]+q[:1])})
    for k, (a, b) in enumerate(edges):
        candidates = [(c, d) for c, d in edges[k+1:] if len({a,b,c,d}) == 4]
        if not candidates:
            continue
        xy = chart[:, [0, 2]]
        start = xy[np.asarray(candidates)[:, 0]]
        end = xy[np.asarray(candidates)[:, 1]]
        cross = lambda u, v: u[..., 0]*v[..., 1]-u[..., 1]*v[..., 0]
        first = cross(xy[b]-xy[a], start-xy[a])*cross(xy[b]-xy[a], end-xy[a])
        second = cross(end-start, xy[a]-start)*cross(end-start, xy[b]-start)
        if np.any((first < -1e-20) & (second < -1e-20)):
            raise ValueError('Forehead source edges cross in angular chart')


def forehead_open_boundary_guard(old, new, faces, affected):
    counts = Counter(tuple(sorted((a, b))) for q in faces
                     for a, b in zip(q, q[1:]+q[:1]))
    touched = {tuple(sorted((a, b))) for i in affected for q in [faces[i]]
               for a, b in zip(q, q[1:]+q[:1])}
    if any(counts[e] not in (1, 2) for e in touched):
        raise ValueError('Nonmanifold pre-existing forehead source')
    boundary = [e for e in sorted(touched) if counts[e] == 1]
    pins = sorted({i for e in boundary for i in e})
    if not np.array_equal(np.asarray(old)[pins], np.asarray(new)[pins]):
        raise ValueError('Forehead changed an existing open boundary endpoint')
    return {'existing_open_edges': [list(e) for e in boundary],
            'open_boundary_positions_exact': True,
            'policy': 'Only unchanged pre-existing incidence-one boundaries; no new holes'}


def propose_forehead_flow(vertices, faces, paths, protected, minimum_height):
    """Fair semantic paths using a donor-bound, symmetric caller transaction.

    Only the positive-side upper corridor is solved. Ear/silhouette/functional
    pins cannot move. All source face cycles and all other points stay exact.
    Candidate steps are rejected for invalid cells rather than clamped in 3D.
    """
    old = np.asarray(vertices, float)
    paths = [list(map(int, p)) for p in paths]
    faces = [list(map(int, f)) for f in faces]
    if len(paths) != 5 or any(len(p) < 5 or len(set(p)) != len(p) for p in paths):
        raise ValueError('Expected five simple forehead source paths')
    if len(set(i for p in paths for i in p)) != sum(map(len, paths)):
        raise ValueError('Distinct forehead paths intersect')
    if not np.isfinite(old).all() or not np.isfinite(minimum_height):
        raise ValueError('Nonfinite forehead source geometry')
    adjacency = defaultdict(set)
    for q in faces:
        for a, b in zip(q, q[1:]+q[:1]):
            adjacency[a].add(b); adjacency[b].add(a)
    if any(b not in adjacency[a] for p in paths for a, b in zip(p, p[1:])):
        raise ValueError('Forehead role is not a current source edge path')
    pins = set(protected) | {i for p in paths for i in (p[0], p[-1])}
    moving = {i for p in paths for i in p[1:-1]
              if old[i, 0] > 1e-8 and old[i, 2] > minimum_height and i not in pins}
    affected = [j for j, q in enumerate(faces) if set(q) & moving]
    if not moving or any(len(faces[i]) != 4 for i in affected):
        raise ValueError('Forehead moving star must contain only quads')
    quads = np.asarray([faces[i] for i in affected], int)
    chart = forehead_chart(old)
    original_cross = forehead_convex_chart(chart, quads)
    surface = AuthorQuadSurface(chart, quads, [1]*len(quads))
    surface.vertices = old.copy()
    target = chart.copy()
    for iteration in range(8):
        previous = target.copy()
        for row, path in enumerate(paths):
            if row < 2 and iteration >= 2:
                continue
            strength = .18 if row < 2 else (.08 if row > 2 else .35)
            for a, b, c in zip(path, path[1:], path[2:]):
                if b not in moving:
                    continue
                la = np.linalg.norm(previous[b, [0, 2]]-previous[a, [0, 2]])
                lb = np.linalg.norm(previous[c, [0, 2]]-previous[b, [0, 2]])
                if min(la, lb) <= 1e-9:
                    raise ValueError('Collapsed forehead path station')
                midpoint = (previous[a]*lb+previous[c]*la)/(la+lb)
                target[b] = previous[b]+strength*(midpoint-previous[b])
    # The green guide approaches the fixed upper-ear rim with its tangent,
    # instead of forcing an interior U-shaped route followed by a horizontal spur.
    green = paths[2]
    if len(green) != 19 or not set(green[14:]).issubset(pins):
        raise ValueError('Green upper-ear rim contract changed')
    segment = green[9:15]
    a, b = chart[segment[0], [0, 2]], chart[segment[-1], [0, 2]]
    ta = a-chart[green[8], [0, 2]]
    tb = chart[green[15], [0, 2]]-b
    lengths = np.linalg.norm(np.diff(chart[segment][:, [0, 2]], axis=0), axis=1)
    total = float(lengths.sum())
    if min(np.linalg.norm(ta), np.linalg.norm(tb), total) <= 1e-9:
        raise ValueError('Degenerate green approach tangent')
    ca = a+ta/np.linalg.norm(ta)*total*.42
    cb = b-tb/np.linalg.norm(tb)*total*.42
    for index, t in zip(segment, np.r_[0, np.cumsum(lengths)]/total):
        if index in moving:
            target[index, [0, 2]] = (1-t)**3*a+3*(1-t)**2*t*ca+3*(1-t)*t*t*cb+t**3*b
    target[green] = chart[green]+.2*(target[green]-chart[green])
    triples = [np.asarray(list(zip(p, p[1:], p[2:])), int) for p in paths]
    angles, ratios = orbital_quad_quality(old, quads)
    before = [orbital_turns(old, t) for t in triples]
    failures = []
    for amount in (.8, .7, .6, .5, .4):
        points, lineage = old.copy(), {}
        try:
            for i in sorted(moving):
                sample = chart[i]+(1. if i in green else amount)*(target[i]-chart[i])
                ids, weights, point = surface.lineage(sample[0], sample[2])
                points[i] = np.asarray(point, np.float32)
                lineage[i] = {'donors': list(ids), 'weights': list(weights)}
            after_chart = forehead_chart(points)
            # Keep conditioning fixed to the immutable source radius.
            after_chart[:, 0] *= np.median(np.linalg.norm(old[:, :2], axis=1))/np.median(np.linalg.norm(points[:, :2], axis=1))
            cross = forehead_convex_chart(after_chart, quads)
            if np.any(cross*original_cross <= 0):
                raise ValueError('Forehead quad orientation reversed')
            forehead_no_chart_crossings(after_chart, [faces[i] for i in affected])
            new_angles, new_ratios = orbital_quad_quality(points, quads)
            if (new_angles.min() < angles.min()-1e-5 or new_angles.max() > angles.max()+1e-5
                    or new_ratios.max() > ratios.max()+1e-5):
                raise ValueError('Forehead source cell quality exceeds the declared envelope')
            old_q, new_q = old[quads], points[quads]
            for a, b, c, d in ((0, 1, 2, 3), (1, 2, 3, 0)):
                n1 = np.cross(new_q[:, b]-new_q[:, a], new_q[:, c]-new_q[:, a])
                n2 = np.cross(new_q[:, c]-new_q[:, a], new_q[:, d]-new_q[:, a])
                length = np.linalg.norm(n1, axis=1)*np.linalg.norm(n2, axis=1)
                if np.any(length <= 1e-18) or np.any(np.sum(n1*n2, axis=1)/length <= .1):
                    raise ValueError('Forehead automatic diagonal folds')
            after = [orbital_turns(points, t) for t in triples]
            if max(after[2]) >= max(before[2])-3:
                raise ValueError('Green temple flow did not meaningfully soften')
            if any(np.mean(a*a) > np.mean(b*b)*1.01 for a, b in zip(after, before)):
                raise ValueError('A forehead flow curvature energy regressed')
            return {'vertices': points, 'faces': faces, 'edited_faces': {},
                    'lineage': lineage, 'affected_faces': affected, 'paths': paths,
                    'metrics': {'amount': amount, 'peak_turn_before': [float(max(t)) for t in before],
                        'peak_turn_after': [float(max(t)) for t in after],
                        'quality_before': [float(angles.min()), float(angles.max()), float(ratios.max())],
                        'quality_after': [float(new_angles.min()), float(new_angles.max()), float(new_ratios.max())],
                        'maximum_move_mm': float(np.linalg.norm(points-old, axis=1).max()*1000),
                        'aspect_ratio_worsened_cells': int(np.sum(new_ratios > ratios+1e-5)),
                        'minimum_angle_worsened_over5deg': int(np.sum(new_angles.min(axis=1) < angles.min(axis=1)-5)),
                        'cells_below20deg': int(np.sum(new_angles.min(axis=1) < 20)),
                        'quality_policy': 'Global minimum angle, maximum angle and maximum ratio no worse; individual-cell tradeoffs recorded; independent shape and native expression guards required'},
                    'scope': 'Source-path fairing on frozen donor quads, not reference vertex copying'}
        except ValueError as error:
            failures.append(str(error))
    raise ValueError('No valid five-flow fairing step: '+str(failures))

"""Resolve the real fitted pinna root in the current assembly frame."""
from collections import defaultdict, deque
import numpy as np
from forehead_star_topology import forehead_guide_corners


def extend_to_fitted_ear(vertices, faces, regions, report, interior_path):
    points = np.asarray(vertices, float)
    rows = [r for r in report['ear_root_fit'] if r['name'] == 'Ear hollow pinna 1']
    if len(rows) != 1:
        raise ValueError('Missing fitted positive ear root')
    position = np.array(max(rows[0]['root_positions'], key=lambda p: p[2]), float)
    position[2] += float(report['head_assembly_y_offset_m'])
    candidates = np.flatnonzero(np.linalg.norm(points-position, axis=1) < 1e-7)
    if len(candidates) != 1:
        raise ValueError('Fitted root does not uniquely match current mesh')
    root = int(candidates[0])
    adjacent = defaultdict(set)
    for q, owner in zip(faces, regions):
        if owner == 1:
            for a, b in zip(q, q[1:]+q[:1]):
                adjacent[a].add(b)
                adjacent[b].add(a)
    prefix = list(interior_path[:-1])
    queue, solutions = deque([[prefix[-1]]]), []
    while queue:
        path = queue.popleft()
        if solutions and len(path) > len(solutions[0]):
            break
        if path[-1] == root:
            solutions.append(path)
            continue
        for nxt in sorted(adjacent[path[-1]]):
            if (nxt not in prefix and nxt not in path and points[nxt, 0] > 0
                    and points[nxt, 2] < points[path[-1], 2]-1e-7):
                queue.append(path+[nxt])
    if len(solutions) != 1:
        raise ValueError('Ambiguous descending skin route to fitted ear')
    result = prefix+solutions[0][1:]
    if forehead_guide_corners(faces, result):
        raise ValueError('Fitted ear route creates a shared-quad elbow')
    return result

"""Differential of an explicitly collocated source vertex to a whole target rail."""
import numpy as np

def forehead_vertex_gap_row(record, stage, points, index, world_jac, world_nonsmooth):
    g = np.zeros(2 * len(index))
    T = record['target_mm']
    flags = []
    if record['domain_boundary']:
        flags.append('vertex_domain_boundary')
    if not record['active']:
        return (g, T, bool(flags), flags)
    source = record['source_id']
    path = np.asarray(stage.paths[record['target']])
    curve = points[path]
    edges = np.diff(curve, axis=0)
    sq = np.sum(edges * edges, axis=1)
    hit = points[source]
    frac = np.clip(np.sum((hit - curve[:-1]) * edges, axis=1) / sq, 0, 1)
    res = hit - (curve[:-1] + frac[:, None] * edges)
    dsq = np.sum(res * res, axis=1)
    k = int(dsq.argmin())
    f = frac[k]
    distance = float(np.sqrt(dsq[k]))
    unit = res[k] / distance if distance > 1e-12 else np.zeros(3)
    if distance < 1e-09:
        flags.append('zero_vertex_distance')
    base = {int(path[k]): 1 - f, int(path[k + 1]): f}
    base = {i: w for i, w in base.items() if w > 1e-09}
    for q in np.flatnonzero(dsq <= dsq[k] + 1e-16):
        other = {int(path[q]): 1 - frac[q], int(path[q + 1]): frac[q]}
        other = {i: w for i, w in other.items() if w > 1e-09}
        if other.keys() != base.keys() or any((abs(other[i] - base[i]) > 1e-07 for i in base)):
            flags.append('distinct_vertex_target_tie')
            break
    if source in index:
        col = index[source]
        g[2 * col:2 * col + 2] += unit @ world_jac[col] * 1000
        if world_nonsmooth[col]:
            flags.append('vertex_author_boundary')
    for node, weight in base.items():
        if node not in index:
            continue
        col = index[node]
        g[2 * col:2 * col + 2] -= weight * (unit @ world_jac[col]) * 1000
        if world_nonsmooth[col]:
            flags.append('vertex_target_author_boundary')
    gap = distance * 1000
    error = gap / T - 1
    if abs(error) < 1e-08 and np.linalg.norm(g) > 1e-12:
        flags.append('vertex_absolute_value_kink')
    return (-np.sign(error) * g / T, gap, bool(flags), flags)

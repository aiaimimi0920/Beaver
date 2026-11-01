"""Read-only piecewise differential prototype. No solver integration.
Coordinates in metres; solver variables in millimetres. Fallback rows must be FD.
"""
from forehead_vertex_gap_jacobian import forehead_vertex_gap_row
import numpy as np
from scipy.sparse import csr_matrix as forehead_gap_csr_matrix

def forehead_gap_source_differential(batch, coordinates):
    """Return exact selected primal, m/mm world Jacobian, author-branch flags.
    Requires the proposed public ForeheadSurfaceBatch.sample_differential API.
    No access to mutable/evictable sampler candidate-cache internals.
    """
    result = batch.sample_differential(coordinates)
    if result is None:
        raise ValueError('Invalid author chart')
    points, derivative_per_chart_metre, boundary = result
    return (points, derivative_per_chart_metre / 1000.0, boundary)

def forehead_matched_gap_jacobian(stage, points, params, ids, world_jac, world_nonsmooth):
    index = {v: i for i, v in enumerate(ids)}
    data = stage.fast.evaluate(points, params, details=True)
    if not data['valid']:
        raise ValueError(data['reason'])
    xy = params[:, [0, 2]] if params.shape[1] == 3 else params
    rows = []
    values = []
    fallback = []
    reasons = {}
    nvars = 2 * len(ids)
    for spec, detail in zip(stage.fast.specs, data['details']):
        a, b, kind, origins, directions = spec
        T = stage.targets[a + '-' + b]
        for side, (source, target) in enumerate(((a, b), (b, a))):
            path = stage.paths[source]
            targetpath = np.asarray(stage.paths[target])
            curve = points[targetpath]
            edges = np.diff(curve, axis=0)
            sq = np.sum(edges * edges, axis=1)
            for j, (hit, s) in enumerate(zip(detail['world_hits'][side], detail['curve_parameters'][side])):
                k = min(int(np.floor(s)), len(path) - 2)
                t = float(s - k)
                i0, i1 = path[k:k + 2]
                normal = np.array([-directions[j, 1], directions[j, 0]])
                den = float(normal @ (xy[i1] - xy[i0]))
                flags = []
                if abs(den) < 1e-10:
                    flags.append('near_parallel_source')
                nearest_integer = round(s)
                if abs(s - nearest_integer) < 1e-07 and path[nearest_integer] in index:
                    flags.append('source_vertex_switch')
                frac = np.clip(np.sum((hit - curve[:-1]) * edges, axis=1) / sq, 0, 1)
                res = hit - (curve[:-1] + frac[:, None] * edges)
                dsq = np.sum(res * res, axis=1)
                l = int(dsq.argmin())
                f = frac[l]
                distance = float(np.sqrt(dsq[l]))
                unit = res[l] / distance if distance > 1e-12 else np.zeros(3)
                if distance < 1e-09:
                    flags.append('zero_distance')
                ties = np.flatnonzero(dsq <= dsq[l] + 1e-16)
                base = {int(targetpath[l]): 1 - f, int(targetpath[l + 1]): f}
                base = {i: w for i, w in base.items() if w > 1e-09}
                for ll in ties:
                    alt = {int(targetpath[ll]): 1 - frac[ll], int(targetpath[ll + 1]): frac[ll]}
                    alt = {i: w for i, w in alt.items() if w > 1e-09}
                    if alt.keys() != base.keys() or any((abs(alt[i] - base[i]) > 1e-07 for i in base)):
                        flags.append('distinct_target_tie')
                        break
                g = np.zeros(nvars)
                worldedge = points[i1] - points[i0]
                for node, weight in [(i0, 1 - t), (i1, t)]:
                    if node not in index:
                        continue
                    col = index[node]
                    hitjac = weight * world_jac[col] - np.outer(worldedge, weight * normal / (1000 * den))
                    g[2 * col:2 * col + 2] += unit @ hitjac * 1000
                    if world_nonsmooth[col] and weight > 1e-09:
                        flags.append('author_boundary')
                for node, weight in base.items():
                    if node not in index:
                        continue
                    col = index[node]
                    g[2 * col:2 * col + 2] -= weight * (unit @ world_jac[col]) * 1000
                    if world_nonsmooth[col]:
                        flags.append('author_boundary')
                gap = distance * 1000
                error = gap / T - 1
                if abs(error) < 1e-08 and np.linalg.norm(g) > 1e-12:
                    flags.append('absolute_value_kink')
                rows.append(-np.sign(error) * g / T)
                values.append(gap)
                fallback.append(bool(flags))
                for flag in set(flags):
                    reasons[flag] = reasons.get(flag, 0) + 1
    for record in stage.vertex_records(points, params, data['details']):
        g, gap, flag, why = forehead_vertex_gap_row(record, stage, points, index, world_jac, world_nonsmooth)
        rows.append(g)
        values.append(gap)
        fallback.append(flag)
        for reason in set(why):
            reasons[reason] = reasons.get(reason, 0) + 1
    return (forehead_gap_csr_matrix(np.asarray(rows)), np.asarray(values), np.asarray(fallback), reasons)

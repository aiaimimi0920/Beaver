"""Whole-guide station evidence and cumulative frozen field references."""
import numpy as np


def forehead_station_residuals(points, paths):
    points = np.asarray(points, float)
    residuals = []
    for path in paths.values():
        lengths = np.linalg.norm(np.diff(points[np.asarray(path, int)], axis=0), axis=1)
        if len(lengths) < 2 or not np.isfinite(lengths).all() or lengths.min() <= 1e-9:
            raise ValueError('Invalid forehead guide station lengths')
        residuals.extend(lengths / lengths.mean() - 1.)
    return np.asarray(residuals)


def measure_forehead_station_spacing(points, paths):
    points = np.asarray(points, float)
    forehead_station_residuals(points, paths)
    result = {}
    for name, path in paths.items():
        lengths = np.linalg.norm(np.diff(points[np.asarray(path, int)], axis=0), axis=1)
        result[name] = {
            'segments': len(lengths),
            'minimum_mm': float(lengths.min() * 1000),
            'maximum_mm': float(lengths.max() * 1000),
            'mean_mm': float(lengths.mean() * 1000),
            'max_min_ratio': float(lengths.max() / lengths.min()),
            'coefficient_of_variation': float(lengths.std() / lengths.mean()),
            'maximum_relative_mean_deviation': float(abs(lengths / lengths.mean() - 1).max()),
            'scope': 'Every source edge of the whole guide, including fixed endpoints',
        }
    return result


def forehead_frozen_field(old_delta, point_count, initial_lineage):
    """Never reset original-vertex transport budgets after intermediate relocation."""
    old_delta = np.asarray(old_delta, float)
    if old_delta.ndim != 2 or old_delta.shape[1] != 3 or point_count < len(old_delta):
        raise ValueError('Invalid frozen forehead field dimensions')
    frozen = np.zeros((point_count, 3))
    frozen[:len(old_delta)] = old_delta
    for index, row in initial_lineage.items():
        if index >= len(old_delta):
            frozen[index] = np.asarray(row['weights']) @ old_delta[row['donors']]
    return frozen


def forehead_initial_parameters(old, points, source_chart, lineage, indices):
    """Recover actual author parameters, not atan2 of a bilinear world sample.

    Polar projection does not invert bilinear source interpolation. Reapplying
    it at each solver stage can move a point before optimization even begins.
    """
    old, points, chart = np.asarray(old), np.asarray(points), np.asarray(source_chart)
    coordinates = []
    for index in indices:
        if index in lineage:
            row = lineage[index]
            parameter = np.asarray(row['weights']) @ chart[row['donors']][:, [0, 2]]
        elif index < len(old) and np.array_equal(points[index], old[index]):
            parameter = chart[index, [0, 2]]
        else:
            raise ValueError('Relocated forehead point lacks frozen author lineage')
        if not np.isfinite(parameter).all():
            raise ValueError('Nonfinite forehead author parameters')
        coordinates.append(parameter)
    return np.asarray(coordinates)


def forehead_physical_turn_residuals(points, paths, limits, step=.01, samples=129):
    """Fixed-distance chord turns, independent of how many stations a rail has.

    The independent dense acceptance sampler uses far more centers. This is an
    appearance objective, never a replacement for that final measurement.
    """
    residuals = []
    for name, path in paths.items():
        curve = np.asarray(points, float)[path]
        lengths = np.r_[0., np.cumsum(np.linalg.norm(np.diff(curve, axis=0), axis=1))]
        if lengths[-1] <= 2*step or np.any(np.diff(lengths) <= 1e-9):
            raise ValueError('Collapsed or too-short physical forehead guide')
        centers = np.linspace(step, lengths[-1]-step, samples)
        a, b, c = [np.column_stack([np.interp(centers+offset, lengths, curve[:, k])
                                    for k in range(3)]) for offset in (-step, 0., step)]
        u, v = b-a, c-b
        cosine = np.einsum('ij,ij->i',u,v)/(np.linalg.norm(u,axis=1)*np.linalg.norm(v,axis=1))
        angles = np.degrees(np.arccos(np.clip(cosine,-1.,1.)))
        residuals.extend(np.maximum(angles-limits[name],0.)/np.sqrt(samples))
    return np.asarray(residuals)


def forehead_uniform_stage_target(old, points, source_chart, lineage, paths, ids,
                                  faces, protected, max_step_mm=6., active_names=None):
    """Move toward genuinely uniform whole-rail stations through bounded stages.

    A partial stage is not an equal-spacing acceptance result. Original source
    coordinates and cumulative field references remain unchanged across stages.
    """
    from scipy.sparse import lil_matrix
    from scipy.sparse.linalg import spsolve
    points = np.asarray(points, float)
    parameters = forehead_initial_parameters(old, points, source_chart, lineage,
                                             list(range(len(points))))
    moving, pins = set(ids), set(protected)
    guides = {i for path in paths.values() for i in path}
    active = set(paths) if active_names is None else set(active_names)
    if not active or not active <= set(paths):raise ValueError('Invalid active forehead guide names')
    delta = np.zeros_like(parameters)
    for name, path in paths.items():
        if name not in active:continue
        arc = np.r_[0., np.cumsum(np.linalg.norm(np.diff(points[path],axis=0),axis=1))]
        if arc[-1] <= 0 or np.any(np.diff(arc) <= 1e-9):
            raise ValueError('Cannot uniformly resample a collapsed forehead guide')
        stations = np.linspace(0.,arc[-1],len(path))
        targets = np.column_stack([np.interp(stations,arc,parameters[path,k]) for k in (0,1)])
        for node, target in zip(path,targets):
            if node in moving and node not in pins:
                delta[node] = target-parameters[node]
    adjacency = {i:set() for i in ids}
    for face in faces:
        for a,b in zip(face,face[1:]+face[:1]):
            if a in adjacency:adjacency[a].add(b)
            if b in adjacency:adjacency[b].add(a)
    support = sorted(moving-guides-pins)
    index = {node:i for i,node in enumerate(support)}
    if support:
        matrix = lil_matrix((len(support),len(support)))
        rhs = np.zeros((len(support),2))
        for node in support:
            neighbors = sorted(adjacency[node])
            weights = 1./np.maximum(np.linalg.norm(parameters[neighbors]-parameters[node],axis=1),1e-9)
            weights /= weights.sum()
            matrix[index[node],index[node]] = 1.
            for other,weight in zip(neighbors,weights):
                if other in index:matrix[index[node],index[other]] -= weight
                else:rhs[index[node]] += weight*delta[other]
        delta[support] = spsolve(matrix.tocsr(),rhs)
    if not np.isfinite(delta).all() or not np.isfinite(max_step_mm) or max_step_mm <= 0:
        raise ValueError('Invalid bounded forehead redistribution target')
    requested_mm = float(np.linalg.norm(delta[ids],axis=1).max()*1000)
    fraction = min(1.,max_step_mm/max(requested_mm,1e-12))
    target = (parameters[ids]+fraction*delta[ids]).ravel()*1000
    return target, {'full_uniform_target_maximum_mm':requested_mm,
                    'stage_fraction':fraction,'maximum_target_step_mm':max_step_mm,
                    'active_guides':[name for name in paths if name in active],
                    'partial_step_is_not_equal_spacing_acceptance':True}

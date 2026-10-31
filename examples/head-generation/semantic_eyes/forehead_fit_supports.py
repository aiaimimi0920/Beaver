"""Quality and posed-field fitting of quad supports and lower guide continuations."""
import numpy as np
from scipy.optimize import minimize, least_squares
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality, orbital_turns
from forehead_flow_rule import forehead_chart, forehead_convex_chart, forehead_no_chart_crossings
from forehead_surface_batch import make_forehead_surface_batch
from forehead_spacing import forehead_frozen_field

def fit_forehead_supports(vertices, old_open_delta, proposal, sparse_warm=False):
    old = np.asarray(vertices, float)
    old_delta = np.asarray(old_open_delta, float)
    initial = np.asarray(proposal['vertices'], float)
    tails = [proposal['paths'][name] for name in ('blue', 'purple')]
    lower = {i for path in tails for i in path[1:-1] if initial[i, 2] < proposal['mouth_height'] + 0.03}
    lower &= set(proposal['moving']) - set(proposal['protected'])
    ids = sorted(set(proposal['support']) | lower)
    lower_triples = np.asarray([t for path in tails for t in zip(path, path[1:], path[2:]) if initial[t[1], 2] < proposal['mouth_height'] + 0.03], int)
    if not len(lower_triples):
        raise ValueError('Missing lower forehead guide continuation')
    quads = np.asarray([proposal['faces'][j] for j in proposal['affected_new']], int)
    source_chart = proposal['source_chart']
    donors = np.asarray(proposal['source_faces'], int)
    radius = np.median(np.linalg.norm(old[:, :2], axis=1))
    start_chart = np.column_stack((radius * np.arctan2(initial[:, 0], -initial[:, 1]), initial[:, 2]))
    start = start_chart[ids].ravel() * 1000
    batch = make_forehead_surface_batch(source_chart, old, donors)
    delta_batch = make_forehead_surface_batch(source_chart, old_delta, donors)
    surface = AuthorQuadSurface(source_chart, donors, [1] * len(donors))
    surface.vertices = old.copy()
    signs = np.sign(forehead_convex_chart(forehead_chart(initial), quads))
    frozen_delta = forehead_frozen_field(old_delta, len(initial), proposal['topology']['lineage'])
    cache, best = ({}, {})
    iterations = [0]

    def evaluate(values):
        key = values.tobytes()
        if key in cache:
            return cache[key]
        coordinates = values.reshape(-1, 2) / 1000
        sampled, delta = (batch.sample(coordinates), delta_batch.sample(coordinates))
        if sampled is None or delta is None:
            return None
        points = initial.copy()
        points[ids] = sampled
        angles, ratios = orbital_quad_quality(points, quads)
        q = forehead_chart(points)[quads][:, :, [0, 2]]
        edge = np.roll(q, -1, axis=1) - q
        cross = edge[:, :, 0] * np.roll(edge[:, :, 1], -1, axis=1)
        cross -= edge[:, :, 1] * np.roll(edge[:, :, 0], -1, axis=1)
        result = (points, angles, ratios, cross, delta)
        cache[key] = result
        if len(cache) > 2 * len(start) + 8:
            del cache[next(iter(cache))]
        return result

    def constraints(values):
        result = evaluate(values)
        if result is None:
            return np.full(len(quads) * 7 + len(ids) + len(lower_triples), -1000.0)
        points, a, ratio, cross, delta = result
        return np.r_[a.min(axis=1) - 25.003, 149.997 - a.max(axis=1), 6.998 - ratio, (cross * signs * 1000000.0).ravel(), 0.499 - np.linalg.norm(delta - frozen_delta[ids], axis=1) * 1000, 19.99 - orbital_turns(points, lower_triples)]

    def objective(values):
        result = evaluate(values)
        if result is None:
            return 100000000.0 + np.sum((values - start) ** 2)
        return np.sum((result[1] - 90) ** 2) / 10000 + 0.02 * np.sum((values - start) ** 2) + 0.1 * np.sum(np.log(result[2]) ** 2) + 0.01 * np.sum(orbital_turns(result[0], lower_triples) ** 2)

    def remember_feasible(values):
        iterations[0] += 1
        if iterations[0] % 20 == 0:
            print('strict', iterations[0], float(constraints(values).min()), flush=True)
        result = evaluate(values)
        if result is None:
            return
        _, a, ratio, cross, delta = result
        feasible = a.min() >= 25.001 and a.max() <= 149.999 and (ratio.max() <= 6.999) and np.all(cross * signs > 0) and (np.linalg.norm(delta - frozen_delta[ids], axis=1).max() * 1000 <= 0.4995) and (max(orbital_turns(result[0], lower_triples)) <= 19.995)
        cost = objective(values)
        if feasible and (not best or cost < best['cost']):
            best.update(values=values.copy(), cost=float(cost), iteration=iterations[0])
    if sparse_warm:
        from scipy.sparse import lil_matrix
        nc = len(quads) * 7 + len(ids) + len(lower_triples)
        sparse = lil_matrix((nc + len(start), len(start)), dtype=int)
        index = {i: j for j, i in enumerate(ids)}

        def mark(rows, nodes):
            columns = [2 * index[i] + d for i in nodes if i in index for d in (0, 1)]
            for row in rows:
                sparse[row, columns] = 1
        Q = len(quads)
        for k, quad in enumerate(quads):
            mark([k, Q + k, 2 * Q + k, *range(3 * Q + 4 * k, 3 * Q + 4 * k + 4)], quad)
        for k, i in enumerate(ids):
            mark([7 * Q + k], [i])
        for k, triple in enumerate(lower_triples):
            mark([7 * Q + len(ids) + k], triple)
        for k in range(len(start)):
            sparse[nc + k, k] = 1
        warm = least_squares(lambda x: np.r_[np.minimum(constraints(x), 0), 0.0001 * (x - start)], start, bounds=(start - 10, start + 10), max_nfev=250, diff_step=1e-06, jac_sparsity=sparse.tocsr(), tr_solver='lsmr')
        print('warm', warm.success, warm.nfev, float(constraints(warm.x).min()), flush=True)
        remember_feasible(warm.x)
    else:
        warm = least_squares(lambda x: np.r_[np.minimum(constraints(x), 0), 0.01 * (x - start)], start, bounds=(start - 10, start + 10), max_nfev=150, diff_step=1e-06)
        print('warm', warm.success, warm.nfev, float(constraints(warm.x).min()), flush=True)
    solved = minimize(objective, warm.x, method='SLSQP', bounds=[(x - 10, x + 10) for x in start], constraints=[{'type': 'ineq', 'fun': constraints}], callback=remember_feasible, options={'maxiter': 200, 'ftol': 1e-06, 'eps': 0.0001})
    remember_feasible(solved.x)
    if not best:
        raise ValueError('No feasible forehead support fit: ' + str(solved.message))
    values = best['values']
    points = initial.copy()
    for i, xy in zip(ids, values.reshape(-1, 2) / 1000):
        source, weights, point = surface.lineage(*xy)
        points[i] = np.asarray(point, np.float32)
        proposal['lineage'][i] = {'donors': list(source), 'weights': list(weights)}
    angles, ratios = orbital_quad_quality(points, quads)
    cross = forehead_convex_chart(forehead_chart(points), quads)
    if angles.min() < 25 or angles.max() > 150 or ratios.max() > 7 or np.any(cross * signs <= 0) or (max(orbital_turns(points, lower_triples)) > 20):
        raise ValueError('Quantized forehead support quality failed')
    forehead_no_chart_crossings(forehead_chart(points), [list(q) for q in quads])
    new_delta = np.zeros_like(points, dtype=np.float32)
    new_delta[:len(old)] = old_delta
    for i, row in proposal['lineage'].items():
        new_delta[i] = np.asarray(row['weights']) @ old_delta[row['donors']]
    if np.linalg.norm(new_delta[ids] - frozen_delta[ids], axis=1).max() * 1000 > 0.5:
        raise ValueError('Quantized forehead support changed its posed field excessively')
    if not np.array_equal(points[proposal['protected']], old[proposal['protected']]):
        raise ValueError('Forehead support fitting changed a protected original point')
    paths = list(proposal['paths'].values())
    turns = [orbital_turns(points, np.asarray(list(zip(p, p[1:], p[2:])))) for p in paths]
    proposal.update(vertices=points, open_delta=new_delta, metrics={'quality_after': [float(angles.min()), float(angles.max()), float(ratios.max())], 'peak_turn_after': [float(max(t)) for t in turns], 'support_solver_converged': bool(solved.success), 'support_solver_status': str(solved.message), 'selected_feasible_iteration': best['iteration'], 'quantized_quality_pass': True, 'posed_field_change_limit_mm': 0.5, 'lower_turn_limit_degrees': 20.0, 'lower_peak_turn_after': float(max(orbital_turns(points, lower_triples))), 'dense_shape_and_native_fields_required_separately': True})
    return proposal

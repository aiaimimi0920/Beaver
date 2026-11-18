"""Quality and posed-field fitting of quad supports and lower guide continuations."""
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality, orbital_turns
from forehead_flow_rule import forehead_chart, forehead_convex_chart, forehead_no_chart_crossings
from forehead_surface_batch import make_forehead_surface_batch
from forehead_spacing import forehead_initial_parameters, forehead_frozen_field, forehead_station_residuals, forehead_physical_turn_residuals, forehead_uniform_stage_target
from forehead_gap_measure import forehead_gap_objective
from forehead_solver_restore import restore_forehead_feasibility, fit_spacing_sparse
from forehead_objective_jacobian import make_forehead_objective_jacobian

def forehead_constraint_scaling(constraints, evaluate, start, step=1e-4):
    """Fixed positive row scaling; the raw feasible set and final guards are unchanged."""
    baseline = constraints(start)
    squared = np.zeros_like(baseline)
    for column in range(len(start)):
        trial = start.copy()
        trial[column] += step
        delta = step
        if evaluate(trial) is None:
            trial[column] = start[column] - step
            delta = -step
        if evaluate(trial) is None:
            raise ValueError('No valid local author-chart derivative for a forehead variable')
        squared += ((constraints(trial)-baseline)/delta)**2
    if not np.isfinite(squared).all():
        raise ValueError('Nonfinite forehead constraint scaling')
    return np.maximum(1., np.sqrt(squared))


def forehead_constraint_layout(quad_count, point_count, turn_count, aspect_rows_per_quad=1):
    if type(aspect_rows_per_quad) is not int or aspect_rows_per_quad < 1:
        raise ValueError('Invalid forehead aspect row count')
    result, offset = {}, 0
    for name, count in (('angle_lower', 4*quad_count), ('angle_upper', 4*quad_count),
                        ('aspect', aspect_rows_per_quad*quad_count), ('orientation', 4*quad_count),
                        ('field', point_count), ('lower_turn', turn_count)):
        result[name] = (offset, offset+count)
        offset += count
    result['count'] = offset
    return result


def forehead_quantized_quality_pass(points, quads):
    saved = np.asarray(points, np.float32).astype(float)
    angles, ratios = orbital_quad_quality(saved, quads)
    return bool(np.isfinite(angles).all() and np.isfinite(ratios).all()
                and angles.min() >= 25 and angles.max() <= 150 and ratios.max() <= 7)


def fit_forehead_density(vertices, old_open_delta, proposal, focus=False, restore=False, max_iterations=200, diagnostic=None, active_guides=None):
    budget_limit = 1000 if restore else 200
    if not isinstance(max_iterations, int) or not 1 <= max_iterations <= budget_limit:
        raise ValueError("Invalid bounded forehead iteration budget")
    active = set(proposal['paths']) if active_guides is None else set(active_guides)
    if not active or not active <= set(proposal['paths']):raise ValueError('Invalid active guide selection')
    active_paths = {name:path for name,path in proposal['paths'].items() if name in active}
    old = np.asarray(vertices, float)
    old_delta = np.asarray(old_open_delta, float)
    initial = np.asarray(proposal['vertices'], float)
    tails = [proposal['paths'][name] for name in ('blue', 'purple')]
    lower = {i for path in tails for i in path[1:-1] if initial[i, 2] < proposal['mouth_height'] + 0.03}
    lower &= set(proposal['moving']) - set(proposal['protected'])
    selected_paths = [proposal['paths']['purple']] if focus else proposal['paths'].values()
    guide_nodes = {i for path in selected_paths for i in path}
    guide_nodes -= set(proposal['protected'])
    near = {i for i in proposal['support'] if min((np.linalg.norm(initial[i] - initial[j]) for j in guide_nodes)) < 0.02}
    ids = sorted(near | guide_nodes) if focus else sorted(set(proposal['support']) | lower | guide_nodes)
    smooth_triples = np.asarray([triple for name in proposal['paths'] for triple in zip(proposal['paths'][name], proposal['paths'][name][1:], proposal['paths'][name][2:])], int)
    lower_triples = np.asarray([t for path in tails for t in zip(path, path[1:], path[2:]) if initial[t[1], 2] < proposal['mouth_height'] + 0.03], int)
    if not len(lower_triples):
        raise ValueError('Missing lower forehead guide continuation')
    all_quads = np.asarray([proposal['faces'][j] for j in proposal['affected_new']], int)
    variable = np.any(np.isin(all_quads, ids), axis=1)
    fixed = all_quads[~variable]
    if len(fixed):
        fa, fr = orbital_quad_quality(initial, fixed)
        if fa.min() < 25 or fa.max() > 150 or fr.max() > 7:
            raise ValueError('Frozen faces already fail hard quality bound')
    quads = all_quads[variable]
    layout = forehead_constraint_layout(len(quads), len(ids), len(lower_triples))
    source_chart = proposal['source_chart']
    donors = np.asarray(proposal['source_faces'], int)
    start = forehead_initial_parameters(old, initial, source_chart, proposal['lineage'], ids).ravel() * 1000
    batch = make_forehead_surface_batch(source_chart, old, donors)
    delta_batch = make_forehead_surface_batch(source_chart, old_delta, donors)
    surface = AuthorQuadSurface(source_chart, donors, [1] * len(donors))
    surface.vertices = old.copy()
    signs = np.sign(forehead_convex_chart(forehead_chart(initial), quads))
    frozen_delta = forehead_frozen_field(old_delta, len(initial), proposal['topology']['lineage'])
    chart_radius = float(np.median(np.linalg.norm(old[:, :2], axis=1)))
    gap_objective = forehead_gap_objective(initial, proposal['paths'])
    radii = np.asarray([8. if i in guide_nodes else 10. for i in ids for _ in range(2)])
    stage_target, stage_metadata = (start.copy(), {}) if restore else forehead_uniform_stage_target(
        old, initial, source_chart, proposal['lineage'], proposal['paths'], ids,
        proposal['faces'], proposal['protected'], active_names=active)
    target_weights = np.repeat([10. if i in guide_nodes else .2 for i in ids], 2)
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
        q = np.column_stack((chart_radius*np.arctan2(points[:, 0], -points[:, 1]), points[:, 2]))[quads]
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
            return np.full(layout['count'], -1000.0)
        points, a, ratio, cross, delta = result
        # Separate every corner: min/max compression has inconsistent finite
        # differences at tied corners, despite defining the same feasible set.
        return np.r_[(a - 25.003).ravel(), (149.997 - a).ravel(), 6.998 - ratio,
                     (cross * signs * 1000000.0).ravel(),
                     0.499 - np.linalg.norm(delta - frozen_delta[ids], axis=1) * 1000,
                     19.99 - orbital_turns(points, lower_triples)]

    def gauge(points):
        p = points[proposal['paths']['purple']]
        length = np.r_[0, np.cumsum(np.linalg.norm(np.diff(p, axis=0), axis=1))]
        d = np.linspace(0.01, length[-1] - 0.01, 160)
        a, b, c = [np.column_stack([np.interp(d + offset, length, p[:, k]) for k in range(3)]) for offset in (-0.01, 0, 0.01)]
        u, w = (b - a, c - b)
        return np.degrees(np.arccos(np.clip(np.sum(u * w, axis=1) / np.linalg.norm(u, axis=1) / np.linalg.norm(w, axis=1), -1, 1))).max()

    def objective(values):
        result = evaluate(values)
        if result is None:
            return 100000000.0 + np.sum((values - start) ** 2)
        if restore:
            return np.mean((values-start)**2)
        cost = (
            gap_objective(result[0])
            + 20 * np.sum(forehead_physical_turn_residuals(result[0], proposal['paths'],
                {'red': 18., 'orange': 20., 'green': 25., 'blue': 30., 'purple': 33.}) ** 2)
            + np.sum(target_weights * (values-stage_target)**2)
            + 2000 * np.sum(forehead_station_residuals(result[0], active_paths) ** 2)
            + np.sum((result[1] - 90) ** 2) / 10000
            + 0.02 * np.sum((values - start) ** 2)
            + 0.1 * np.sum(np.log(result[2]) ** 2)
            + 0.01 * np.sum(orbital_turns(result[0], lower_triples) ** 2)
            + (20 * max(0, gauge(result[0]) - 27.5) ** 2 if focus else 0)
            + 0.08 * np.sum(orbital_turns(result[0], smooth_triples) ** 2)
            + 2 * np.sum(np.log(np.linalg.norm(result[0][smooth_triples[:, 1]] - result[0][smooth_triples[:, 0]], axis=1)
                               / np.linalg.norm(result[0][smooth_triples[:, 2]] - result[0][smooth_triples[:, 1]], axis=1)) ** 2)
        )
        return cost / 10000


    def remember_feasible(values):
        iterations[0] += 1
        if iterations[0] % 20 == 0:
            print('strict', iterations[0], float(constraints(values).min()), flush=True)
        result = evaluate(values)
        if result is None:
            return
        _, a, ratio, cross, delta = result
        feasible = a.min() >= 25.001 and a.max() <= 149.999 and (ratio.max() <= 6.999) and np.all(cross * signs > 0) and (np.linalg.norm(delta - frozen_delta[ids], axis=1).max() * 1000 <= 0.4995) and (max(orbital_turns(result[0], lower_triples)) <= 19.995)
        # A restoration witness must satisfy its own tighter solve constraints.
        # Otherwise a zero-cost, slightly infeasible start can permanently win
        # over every genuinely restored point and poison the next stage.
        if restore:
            feasible = feasible and bool(np.all(constraints(values) >= 0))
        cost = objective(values)
        if feasible and (not best or cost < best['cost']):
            # Model coordinates are saved as float32. Short edges amplify tiny
            # rounding errors; a floating witness must never replace a valid
            # saved-coordinate checkpoint without checking its actual corners.
            quantized = result[0].astype(np.float32).astype(float)
            feasible = (forehead_quantized_quality_pass(quantized, all_quads)
                        and max(orbital_turns(quantized, lower_triples)) <= 20
                        and np.linalg.norm(delta.astype(np.float32)-frozen_delta[ids], axis=1).max()*1000 <= .5)
            if feasible:
                try:
                    qc = forehead_convex_chart(forehead_chart(quantized), quads)
                    feasible = bool(np.all(qc*signs > 0))
                except ValueError:
                    feasible = False
            if feasible:
                best.update(values=values.copy(), cost=float(cost), iteration=iterations[0])
        # Restoration has no appearance goal. Stop once a quantized witness
        # exists instead of spending the remaining budget minimizing its tether.
        return bool(restore and best and np.all(constraints(best['values']) >= 0))
    from scipy.sparse import lil_matrix
    nc = layout['count']
    sparse = lil_matrix((nc + len(start), len(start)), dtype=int)
    index = {i: j for j, i in enumerate(ids)}

    def mark(rows, nodes):
        columns = [2 * index[i] + d for i in nodes if i in index for d in (0, 1)]
        for row in rows:
            sparse[row, columns] = 1
    Q = len(quads)
    for k, quad in enumerate(quads):
        rows = [layout['aspect'][0]+k]
        for name in ('angle_lower', 'angle_upper', 'orientation'):
            rows.extend(range(layout[name][0]+4*k, layout[name][0]+4*k+4))
        mark(rows, quad)
    for k, i in enumerate(ids):
        mark([layout['field'][0]+k], [i])
    for k, triple in enumerate(lower_triples):
        mark([layout['lower_turn'][0]+k], triple)
    for k in range(len(start)):
        sparse[nc + k, k] = 1
    scales = forehead_constraint_scaling(constraints, evaluate, start)
    scaled_constraints = lambda values: constraints(values) / scales
    remember_feasible(start)
    gradient_metadata = {}
    if restore:
        solved = restore_forehead_feasibility(
            start, radii, scaled_constraints, sparse[:nc, :].tocsr(),
            layout['orientation'], layout['field'], remember_feasible, max_iterations)
    else:
        derivative, gradient_metadata = make_forehead_objective_jacobian(
            objective, evaluate, start, stage_target, target_weights, ids,
            {i for path in proposal['paths'].values() for i in path}, quads)
        solved = fit_spacing_sparse(start, radii, objective, scaled_constraints,
            sparse[:nc, :].tocsr(), remember_feasible, max_iterations, jacobian=derivative)
    remember_feasible(solved.x)
    if diagnostic is not None:
        diagnostic({
            'stage': 'solver_end', 'constraint_layout': layout, 'variable_ids': ids,
            'start_parameters_mm': start.tolist(), 'parameters_mm': solved.x.tolist(),
            'constraint_scales': scales.tolist(), 'raw_constraints': constraints(solved.x).tolist(),
            'solver_success': bool(solved.success), 'solver_message': str(solved.message),
            'best_parameters_mm': best['values'].tolist() if best else None,
            'scope': 'Diagnostic parameters only; final quantized mesh checks still required',
        })
    if not best:
        raise ValueError('No feasible forehead support fit: ' + str(solved.message))
    values = best['values']
    points = initial.copy()
    for i, xy in zip(ids, values.reshape(-1, 2) / 1000):
        source, weights, point = surface.lineage(*xy)
        points[i] = np.asarray(point, np.float32)
        proposal['lineage'][i] = {'donors': list(source), 'weights': list(weights)}
    all_angles, all_ratios = orbital_quad_quality(points, all_quads)
    if all_angles.min() < 25 or all_angles.max() > 150 or all_ratios.max() > 7:
        raise ValueError('Final full patch quality failed')
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
    proposal.update(vertices=points, open_delta=new_delta, metrics={'solver_phase': 'feasibility' if restore else 'spacing', 'solver_iteration_budget': max_iterations, 'uniform_station_stage': stage_metadata, 'objective_gradient': gradient_metadata, 'objective_before': float(objective(start)), 'objective_after': float(best['cost']), 'quality_after': [float(angles.min()), float(angles.max()), float(ratios.max())], 'peak_turn_after': [float(max(t)) for t in turns], 'support_solver_converged': bool(solved.success), 'support_solver_status': str(solved.message), 'selected_feasible_iteration': best['iteration'], 'quantized_quality_pass': True, 'posed_field_change_limit_mm': 0.5, 'lower_turn_limit_degrees': 20.0, 'lower_peak_turn_after': float(max(orbital_turns(points, lower_triples))), 'dense_shape_and_native_fields_required_separately': True})
    return proposal

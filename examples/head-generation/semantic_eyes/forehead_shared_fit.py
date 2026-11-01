"""Quality and posed-field fitting of quad supports and lower guide continuations."""
from forehead_fit_density import forehead_constraint_layout, forehead_quantized_quality_pass
from forehead_fairing_solver import make_forehead_fairing_objective, solve_forehead_fairing
from forehead_working_targets import ForeheadWorkingFlowStage
from forehead_matched_gap_stage import ForeheadMatchedGapStage
from forehead_hybrid_gap_jacobian import forehead_make_hybrid_gap_jacobian
from forehead_hybrid_scaling import forehead_prepare_hybrid_scaling
import copy as forehead_shared_copy
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality, orbital_turns
from forehead_flow_rule import forehead_chart, forehead_convex_chart, forehead_no_chart_crossings
from forehead_surface_batch import make_forehead_surface_batch
from forehead_spacing import forehead_initial_parameters, forehead_frozen_field
from forehead_solver_restore import restore_forehead_feasibility
from forehead_fairing_restore import forehead_restore_fairing_iterate, forehead_minimum_improvement_passes
from forehead_guide_fairing import forehead_guides_fairing
from forehead_lower_guard import forehead_build_lower_guard, forehead_update_lower_guard
from forehead_seam_growth import forehead_restore_seam_growth
from forehead_arc_quantization import forehead_arc_quantization_targets
from forehead_seam_support import forehead_select_seam_supports
from forehead_failed_seam_support import forehead_select_failed_seam_supports
from forehead_seam_hint import forehead_revalidate_seam_hint, forehead_seam_source_identity, forehead_next_stage_seam_hint
from forehead_field_jacobian import forehead_field_parameter_jacobian
from forehead_orientation_margin import forehead_signed_chart_cross_mm2, forehead_orientation_margin_targets
from forehead_lp_restore import forehead_restore_linearized, forehead_quantization_margin_targets
from forehead_anchor_domain import ForeheadAnchorDomain
from forehead_aspect_constraints import forehead_aspect_reference, forehead_pairwise_aspect

def fit_forehead_shared_constraints(vertices, old_open_delta, proposal, focus=False, restore=False, max_iterations=200, diagnostic=None, active_guides=None, physical_stage=False, progress_names=None, progress_scale=1.0, author_context=None, active_gap_pair=None, gap_reduction=0.025, gap_sample_count=129, objective_mode='restore', minimum_improvement=0., solver_mode='trust', seam_active_set_growth=False):
    if type(seam_active_set_growth) is not bool:
        raise ValueError('Invalid optional seam active-set mode')
    if solver_mode not in ('trust', 'linearized'):
        raise ValueError('Invalid shared solver mode')
    if solver_mode == 'linearized' and minimum_improvement:
        raise ValueError('Linearized restoration does not optimize bending improvement')
    if objective_mode not in ('restore', 'fairing') or not 0 <= minimum_improvement < 1:
        raise ValueError('Invalid shared fitting objective or improvement target')
    if objective_mode == 'restore' and minimum_improvement:
        raise ValueError('Restoration has no bending improvement target')
    if author_context is None or not restore or not physical_stage:
        raise ValueError('Shared fitting requires author context and retained flow restoration')
    if not isinstance(max_iterations, int) or not 1 <= max_iterations <= 1000:
        raise ValueError('Invalid bounded forehead iteration budget')
    # Rejected scalar reconstruction must not mutate the caller's accepted state.
    proposal = forehead_shared_copy.deepcopy(proposal)
    for owner in (proposal, proposal['topology']):
        owner['lineage'] = {int(index): row for index, row in owner['lineage'].items()}
    active = set(proposal['paths']) if active_guides is None else set(active_guides)
    if not active or not active <= set(proposal['paths']):
        raise ValueError('Invalid active guide selection')
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
    prior_lower_guard = (proposal['lower_turn_guard'] if 'lower_turn_guard' in proposal else
                         forehead_build_lower_guard(proposal, initial))
    lower_guard = forehead_update_lower_guard(proposal, initial, prior_lower_guard)
    proposal['lower_turn_guard'] = lower_guard
    lower_triples = np.asarray(lower_guard['triples'], int)
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
    layout = forehead_constraint_layout(len(quads), len(ids), len(lower_triples), aspect_rows_per_quad=12)
    aspect_reference = forehead_aspect_reference(initial, quads)
    source_chart = np.asarray(proposal['source_chart'], float)
    donors = np.asarray(proposal['source_faces'], int)
    start = forehead_initial_parameters(old, initial, source_chart, proposal['lineage'], ids).ravel() * 1000
    batch = make_forehead_surface_batch(source_chart, old, donors)
    delta_batch = make_forehead_surface_batch(source_chart, old_delta, donors)
    surface = AuthorQuadSurface(source_chart, donors, [1] * len(donors))
    surface.vertices = old.copy()
    signs = np.sign(forehead_convex_chart(forehead_chart(initial), quads))
    frozen_delta = forehead_frozen_field(old_delta, len(initial), proposal['topology']['lineage'])
    flow_stage = None
    if physical_stage:
        reference = proposal.setdefault('uniform_station_reference_length_mm', {name: float(np.linalg.norm(np.diff(initial[path], axis=0), axis=1).sum() * 1000) for name, path in proposal['paths'].items()})
        flow_stage = ForeheadWorkingFlowStage(old, initial, proposal['paths'], reference_length_mm=reference, cap_state=proposal.get('joint_stage_caps'), advance_names=progress_names or (), step_scale=progress_scale)
        offset = layout['count']
        layout['physical_flow'] = (offset, offset + flow_stage.count)
        layout['count'] += flow_stage.count
    original_params = np.vstack((source_chart, np.zeros((len(initial) - len(old), 3))))
    for i, row in proposal['lineage'].items():
        original_params[int(i)] = np.asarray(row['weights']) @ source_chart[row['donors']]

    def author_parameters(values):
        params = original_params.copy()
        params[np.asarray(ids), 0] = values.reshape(-1, 2)[:, 0] / 1000
        params[np.asarray(ids), 2] = values.reshape(-1, 2)[:, 1] / 1000
        return params
    gap_stage = ForeheadMatchedGapStage(author_context, proposal, original_params, active_pair=active_gap_pair, reduction=gap_reduction, count=gap_sample_count)
    anchor_domain = ForeheadAnchorDomain(gap_stage.fast, author_parameters(start), ids)
    offset = layout['count']
    layout['anchor_domain'] = (offset, offset + len(anchor_domain.specs))
    layout['count'] += len(anchor_domain.specs)
    offset = layout['count']
    layout['matched_gap'] = (offset, offset + gap_stage.size)
    layout['count'] += gap_stage.size
    chart_radius = float(np.median(np.linalg.norm(old[:, :2], axis=1)))
    radii = np.asarray([8.0 if i in guide_nodes else 10.0 for i in ids for _ in range(2)])
    stage_metadata = {}
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
        edges = np.roll(points[quads], -1, axis=1)-points[quads]
        if not np.isfinite(edges).all() or np.any(np.sum(edges*edges, axis=2) <= 0):
            return None
        angles, ratios = orbital_quad_quality(points, quads)
        q = np.column_stack((chart_radius * np.arctan2(points[:, 0], -points[:, 1]), points[:, 2]))[quads]
        edge = np.roll(q, -1, axis=1) - q
        cross = edge[:, :, 0] * np.roll(edge[:, :, 1], -1, axis=1)
        cross -= edge[:, :, 1] * np.roll(edge[:, :, 0], -1, axis=1)
        result = (points, angles, ratios, cross, delta)
        cache[key] = result
        if len(cache) > 2 * len(start) + 8:
            del cache[next(iter(cache))]
        return result

    def base_constraints(values):
        result = evaluate(values)
        if result is None:
            return np.full(layout['matched_gap'][0], -1000.0)
        points, a, ratio, cross, delta = result
        aspect = forehead_pairwise_aspect(points, quads, 6.998, aspect_reference)
        result = np.r_[(a - 25.003).ravel(), (149.997 - a).ravel(), aspect, (cross * signs * 1000000.0).ravel(), 0.499 - np.linalg.norm(delta - frozen_delta[ids], axis=1) * 1000, 19.99 - orbital_turns(points, lower_triples)]
        return np.r_[result, flow_stage.constraints(points), anchor_domain.values(values)]

    def constraints(values):
        base = base_constraints(values)
        result = evaluate(values)
        gaps = np.full(gap_stage.size, -1000.0) if result is None else gap_stage.constraints(result[0], author_parameters(values))
        return np.r_[base, gaps]

    def objective(values):
        if evaluate(values) is None:
            return 1e8 + np.sum((values-start)**2)
        return np.mean((values-start)**2)

    objective_derivative, fairing_metadata = None, {}
    if objective_mode == 'fairing':
        objective, objective_derivative, fairing_metadata = make_forehead_fairing_objective(
            evaluate, batch, ids, proposal['paths'],
            proposal['uniform_station_reference_length_mm'], start)
    initial_objective = objective(start)

    def guarded_lower_peak(points):
        guarded = forehead_update_lower_guard(proposal, points, lower_guard)
        return float(max(orbital_turns(points, np.asarray(guarded['triples'], int))))

    def remember_feasible(values):
        iterations[0] += 1
        if iterations[0] % 20 == 0:
            print('strict', iterations[0], float(constraints(values).min()), flush=True)
        result = evaluate(values)
        if result is None:
            return
        _, a, ratio, cross, delta = result
        feasible = a.min() >= 25.001 and a.max() <= 149.999 and (ratio.max() <= 6.999) and np.all(cross * signs > 0) and (np.linalg.norm(delta - frozen_delta[ids], axis=1).max() * 1000 <= 0.4995) and (max(orbital_turns(result[0], lower_triples)) <= 19.995)
        if restore:
            feasible = feasible and bool(np.all(constraints(values) >= -1e-09))
        cost = objective(values)
        if feasible and (not best or cost < best['cost']):
            quantized = result[0].astype(np.float32).astype(float)
            feasible = forehead_quantized_quality_pass(quantized, all_quads) and guarded_lower_peak(quantized) <= 20 and (np.linalg.norm(delta.astype(np.float32) - frozen_delta[ids], axis=1).max() * 1000 <= 0.5)
            if feasible:
                try:
                    qc = forehead_convex_chart(forehead_chart(quantized), quads)
                    feasible = bool(np.all(qc * signs > 0))
                except ValueError:
                    feasible = False
            if feasible and flow_stage:
                feasible = flow_stage.accepts(quantized)
            if feasible:
                feasible = gap_stage.accepts(quantized, author_parameters(values))
            if feasible:
                best.update(values=values.copy(), cost=float(cost), iteration=iterations[0])
        return bool(best and (not minimum_improvement or best['cost'] <=
                    (1-minimum_improvement)*initial_objective)
                    and np.all(constraints(best['values']) >= -1e-09))
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
        rows = list(range(layout['aspect'][0] + 12*k, layout['aspect'][0] + 12*k + 12))
        for name in ('angle_lower', 'angle_upper', 'orientation'):
            rows.extend(range(layout[name][0] + 4 * k, layout[name][0] + 4 * k + 4))
        mark(rows, quad)
    for k, i in enumerate(ids):
        mark([layout['field'][0] + k], [i])
    for k, triple in enumerate(lower_triples):
        mark([layout['lower_turn'][0] + k], triple)
    if flow_stage:
        for a, b, nodes in flow_stage.groups:
            mark(range(layout['physical_flow'][0] + a, layout['physical_flow'][0] + b), nodes)
    for row, (_, _, first, last) in enumerate(anchor_domain.specs):
        mark([layout['anchor_domain'][0] + row], [first, last])
    for a, b, nodes in gap_stage.groups:
        mark(range(layout['matched_gap'][0] + a, layout['matched_gap'][0] + b), nodes)
    for k in range(len(start)):
        sparse[nc + k, k] = 1
    scales = np.ones(nc, dtype=float)
    scaled_constraints = lambda values: constraints(values) / scales
    constraint_jacobian, hybrid_metadata = forehead_make_hybrid_gap_jacobian(base_constraints, evaluate, author_parameters, batch, ids, gap_stage, layout['matched_gap'][0], scales, sparse[:nc, :].tocsr())
    hybrid_derivative = constraint_jacobian
    field_jacobian_stats = {'exact_primal_checks': 0, 'boundary_fallback_rows': 0}

    def constraint_jacobian(values):
        result = hybrid_derivative(values).tolil()
        first, last = layout['anchor_domain']
        from scipy.sparse import diags
        result[first:last] = diags(1 / scales[first:last]) @ anchor_domain.jacobian(values)
        state = evaluate(values)
        if state is not None:
            differential = delta_batch.sample_differential(values.reshape(-1, 2) / 1000)
            if differential is None or not np.array_equal(differential[0], state[4]):
                raise ValueError('Field derivative and primal source samples differ')
            sampled, field_differential, boundary = differential
            block, flags = forehead_field_parameter_jacobian(
                sampled, field_differential, frozen_delta[ids], boundary)
            first, last = layout['field']
            block = (diags(1 / scales[first:last]) @ block).tolil()
            if np.any(flags):
                block[flags] = result[first:last].tocsr()[flags]
            result[first:last] = block
            field_jacobian_stats['exact_primal_checks'] += 1
            field_jacobian_stats['boundary_fallback_rows'] += int(flags.sum())
        return result.tocsr()

    constraint_jacobian, scaling_metadata = forehead_prepare_hybrid_scaling(
        constraint_jacobian, start, scales)
    remember_feasible(start)
    gradient_metadata = {}
    linearized_metadata = {}
    if solver_mode == 'linearized':
        accepted_cross = forehead_signed_chart_cross_mm2(
            evaluate(start)[0], quads, signs, chart_radius).ravel()

        def complete_domain(values):
            state = evaluate(values)
            if state is None or np.min(anchor_domain.values(values)) < 0:
                return {'valid': False, 'reason': 'Source sample or endpoint anchor invalid'}
            measured = gap_stage.fast.evaluate(state[0], author_parameters(values), details=True)
            return {'valid': measured['valid'], 'reason': measured['reason']}

        def quantization_margins(values, residuals):
            state = evaluate(values)
            if state is None:
                return None
            smooth = state[1]
            rounded, _ = orbital_quad_quality(state[0].astype(np.float32).astype(float), quads)
            lower_rows = np.arange(*layout['angle_lower'])
            upper_rows = np.arange(*layout['angle_upper'])
            jacobian = constraint_jacobian(values)
            targets, evidence = forehead_quantization_margin_targets(
                residuals, jacobian, scales, lower_rows, upper_rows, smooth, rounded)
            orientation_rows = np.arange(*layout['orientation'])
            rows = jacobian[orientation_rows]
            variable_rows = np.asarray(rows.multiply(rows).sum(axis=1)).ravel() > 1e-24
            smooth_cross = forehead_signed_chart_cross_mm2(
                state[0], quads, signs, chart_radius).ravel()
            rounded_cross = forehead_signed_chart_cross_mm2(
                state[0].astype(np.float32).astype(float), quads, signs, chart_radius).ravel()
            targets, orientation_evidence = forehead_orientation_margin_targets(
                targets, orientation_rows, scales, smooth_cross, rounded_cross,
                variable_rows, accepted_mm2=accepted_cross, interior_cap_mm2=.001)
            orientation_summary = {
                'working_target_row_count': len(orientation_evidence),
                'accepted_start_floor_cap_mm2': .001,
                'rounding_repairs': [row for row in orientation_evidence if row['rounding_failed']],
                'fixed_source_radius_m': chart_radius,
                'original_accepted_reference_retained': True}
            arc_evidence = []
            if flow_stage:
                arc_targets, arc_evidence = forehead_arc_quantization_targets(
                    state[0], flow_stage.spacing,
                    layout['physical_flow'][0] + flow_stage.flow.count, scales)
                targets = np.maximum(targets, arc_targets)
            return targets, {'angles': evidence, 'orientation': orientation_summary,
                             'arc_quantization': arc_evidence}

        orientation_first, orientation_last = layout['orientation']
        orientation_floor = np.minimum(.5 * accepted_cross, .001) / scales[orientation_first:orientation_last]
        source_identity = forehead_seam_source_identity(
            (old, old_delta, source_chart, donors, proposal['topology']['vertices'], frozen_delta),
            proposal['topology']['faces'])
        prior = (proposal.get('metrics', {}).get('next_stage_seam_hint') or
                 proposal.get('metrics', {}).get('linearized_restoration', {}).get('successful_selection') or {})
        seam_selection = {'held_columns': [], 'held_vertices': []}
        try:
            if prior.get('evidence'):
                seam_selection = forehead_revalidate_seam_hint(
                    prior, start.reshape(-1, 2), ids, frozen_delta[ids], proposal['support'],
                    proposal['paths'], proposal['protected'], delta_batch.sample_differential,
                    source_identity=source_identity, previous_source_identity=prior.get('source_identity', ''),
                    max_holds=4 if seam_active_set_growth else 1)
            if not seam_selection.get('held_columns'):
                seam_selection = forehead_select_seam_supports(
                    start.reshape(-1, 2), ids, frozen_delta[ids], proposal['support'],
                    proposal['paths'], proposal['protected'], delta_batch.sample_differential,
                    lambda xy: surface.lineage(*xy)[0])
        except ValueError as error:
            seam_selection = {'held_columns': [], 'held_vertices': [], 'selection_unavailable': str(error)}
        seam_selection['source_identity'] = source_identity

        def failed_selection(failed):
            selected = forehead_select_failed_seam_supports(
                start.reshape(-1, 2), failed.reshape(-1, 2), ids, frozen_delta[ids],
                proposal['support'], proposal['paths'], proposal['protected'],
                delta_batch.sample_differential, lambda xy: surface.lineage(*xy)[0],
                original_radii_mm=radii.reshape(-1, 2),
                max_holds=4 if seam_active_set_growth else 1)
            selected['source_identity'] = source_identity
            return selected

        solved, linearized_metadata = forehead_restore_seam_growth(
            start, radii, scaled_constraints, constraint_jacobian,
            remember_feasible, layout['orientation'], complete_domain,
            seam_selection, quantization_margins, orientation_floor,
            max_iterations=min(24, max_iterations), wall_seconds=120.,
            failed_selection=failed_selection, active_set_growth=seam_active_set_growth,
            growth_context={'variable_ids': ids, 'frozen_field_reference_m': frozen_delta[ids],
                            'support_ids': proposal['support'], 'protected_ids': proposal['protected'],
                            'guide_paths': proposal['paths'], 'source_identity': source_identity})
    elif objective_mode == 'fairing':
        solved = solve_forehead_fairing(start, radii, scaled_constraints, constraint_jacobian,
            layout['orientation'], remember_feasible, objective, objective_derivative, max_iterations)
    else:
        solved = restore_forehead_feasibility(
            start, radii, scaled_constraints, sparse[:nc, :].tocsr(),
            layout['orientation'], layout['field'], remember_feasible,
            max_iterations, jacobian=constraint_jacobian)
    remember_feasible(solved.x)
    restoration_continuation = {}
    if solver_mode == 'trust' and objective_mode == 'fairing' and not best:
        # One feasibility leg of this exact problem. Keep ORIGINAL bounds,
        # frozen lower triples, source sheets, row scaling, working caps and
        # references; do not rebuild a proposal/stage from the failed iterate.
        restoration_continuation = {
            'fairing_solver_success': bool(solved.success),
            'fairing_solver_message': str(solved.message),
            'fairing_parameters_mm': solved.x.tolist()}
        try:
            solved, continued = forehead_restore_fairing_iterate(
                solved.x, start, radii, scaled_constraints,
                sparse[:nc, :].tocsr(), layout['orientation'], remember_feasible,
                max_iterations=min(max_iterations, 80),
                jacobian=constraint_jacobian, wall_seconds=120.)
            restoration_continuation.update(continued)
            remember_feasible(solved.x)
        except ValueError as error:
            restoration_continuation['restoration_preflight_error'] = str(error)
    if diagnostic is not None:
        diagnostic({'stage': 'solver_end', 'constraint_layout': layout, 'variable_ids': ids, 'start_parameters_mm': start.tolist(), 'parameters_mm': solved.x.tolist(), 'constraint_scales': scales.tolist(), 'raw_constraints': constraints(solved.x).tolist(), 'solver_success': bool(solved.success), 'solver_message': str(solved.message), 'best_parameters_mm': best['values'].tolist() if best else None, 'restoration_continuation': restoration_continuation, 'linearized_restoration': linearized_metadata, 'lower_turn_guard': lower_guard, 'scope': 'Diagnostic parameters only; final quantized mesh checks still required'})
    if not best:
        raise ValueError('No feasible forehead support fit: ' + str(solved.message))
    values = best['values']
    points = initial.copy()
    for i, xy in zip(ids, values.reshape(-1, 2) / 1000):
        source, weights, point = surface.lineage(*xy)
        points[i] = np.asarray(point, np.float32)
        proposal['lineage'][i] = {'donors': list(source), 'weights': list(weights)}
    held_selection = linearized_metadata.get('successful_selection') or {}
    if held_selection.get('held_vertices'):
        held_vertices = held_selection['held_vertices']
        if not np.array_equal(points[held_vertices], initial[held_vertices]):
            raise ValueError('Temporary held support changed during scalar reconstruction')
    all_angles, all_ratios = orbital_quad_quality(points, all_quads)
    if all_angles.min() < 25 or all_angles.max() > 150 or all_ratios.max() > 7:
        raise ValueError('Final full patch quality failed')
    angles, ratios = orbital_quad_quality(points, quads)
    cross = forehead_convex_chart(forehead_chart(points), quads)
    if angles.min() < 25 or angles.max() > 150 or ratios.max() > 7 or np.any(cross * signs <= 0) or (guarded_lower_peak(points) > 20):
        raise ValueError('Quantized forehead support quality failed')
    forehead_no_chart_crossings(forehead_chart(points), [list(q) for q in quads])
    new_delta = np.zeros_like(points, dtype=np.float32)
    new_delta[:len(old)] = old_delta
    for i, row in proposal['lineage'].items():
        new_delta[i] = np.asarray(row['weights']) @ old_delta[row['donors']]
    if np.linalg.norm(new_delta - frozen_delta, axis=1).max() * 1000 > 0.5:
        raise ValueError('Quantized forehead support changed its posed field excessively')
    if not np.array_equal(points[proposal['protected']], old[proposal['protected']]):
        raise ValueError('Forehead support fitting changed a protected original point')
    paths = list(proposal['paths'].values())
    turns = [orbital_turns(points, np.asarray(list(zip(p, p[1:], p[2:])))) for p in paths]
    if flow_stage and (not flow_stage.accepts(points)):
        raise ValueError('Final scalar reconstruction fails joint flow/station stage')
    reconstructed_params = original_params.copy()
    for index, row in proposal['lineage'].items():
        reconstructed_params[int(index)] = np.asarray(row['weights']) @ source_chart[row['donors']]
    if not gap_stage.accepts(points, reconstructed_params):
        raise ValueError('Final scalar reconstruction fails fixed-domain gap stage')
    final_parameters = reconstructed_params[ids][:, [0, 2]].ravel() * 1000
    if np.min(anchor_domain.values(final_parameters)) < 0:
        raise ValueError('Final scalar reconstruction fails anchor domain')
    final_lower_guard = forehead_update_lower_guard(proposal, points, lower_guard)
    proposal['lower_turn_guard'] = final_lower_guard
    proposal.update(vertices=points, open_delta=new_delta, metrics={'solver_phase': 'feasibility' if restore else 'spacing', 'solver_iteration_budget': max_iterations, 'uniform_station_stage': stage_metadata, 'objective_gradient': gradient_metadata, 'objective_before': float(objective(start)), 'objective_after': float(best['cost']), 'quality_after': [float(angles.min()), float(angles.max()), float(ratios.max())], 'peak_turn_after': [float(max(t)) for t in turns], 'support_solver_converged': bool(solved.success), 'support_solver_status': str(solved.message), 'selected_feasible_iteration': best['iteration'], 'quantized_quality_pass': True, 'posed_field_change_limit_mm': 0.5, 'lower_turn_limit_degrees': 20.0, 'lower_peak_turn_after': guarded_lower_peak(points), 'dense_shape_and_native_fields_required_separately': True})
    if flow_stage:
        proposal['metrics']['physical_flow_stage'] = flow_stage.evidence()
        proposal['joint_stage_caps'] = flow_stage.state
    proposal['metrics']['matched_gap_stage'] = gap_stage.evidence()
    proposal['metrics']['hybrid_constraint_jacobian'] = hybrid_metadata
    proposal['metrics']['constraint_scaling'] = scaling_metadata
    proposal['metrics']['field_jacobian'] = field_jacobian_stats
    proposal['metrics']['anchor_domain'] = {
        'rows': len(anchor_domain.specs), 'specs': anchor_domain.specs,
        'margin': anchor_domain.margin.tolist(),
        'necessary_endpoint_rows_only': True, 'full_atlas_required': True}
    proposal['metrics']['restoration_continuation'] = restoration_continuation
    proposal['metrics']['linearized_restoration'] = linearized_metadata
    proposal['metrics']['solver_mode'] = solver_mode
    proposal['metrics']['solver_iterations'] = int(solved.nit)
    proposal['metrics']['solver_work_iterations'] = linearized_metadata.get('total_work_iterations', int(solved.nit))
    proposal['metrics']['lower_turn_guard_before'] = forehead_shared_copy.deepcopy(lower_guard)
    proposal['metrics']['lower_turn_guard_after'] = forehead_shared_copy.deepcopy(final_lower_guard)
    proposal['metrics']['objective_mode'] = objective_mode
    proposal['metrics']['guide_fairing'] = fairing_metadata
    proposal['metrics']['minimum_objective_improvement_requested'] = minimum_improvement
    proposal['matched_gap_caps_v2'] = gap_stage.caps
    if minimum_improvement:
        references = {name: value/1000 for name, value in proposal['uniform_station_reference_length_mm'].items()}
        initial_scalar_cost = forehead_guides_fairing(initial, proposal['paths'], references)[0]/len(paths)
        final_scalar_cost = (forehead_guides_fairing(points, proposal['paths'], references)[0]/len(paths)
                             + .0001*np.mean((values-start)**2))
        proposal['metrics']['minimum_improvement_scalar_costs'] = [float(initial_scalar_cost), float(final_scalar_cost)]
        if not forehead_minimum_improvement_passes(initial_scalar_cost, final_scalar_cost, minimum_improvement):
            if diagnostic is not None:
                diagnostic({'stage': 'minimum_improvement_unmet', 'fit_complete': False,
                    'geometry_admission_passed': True, 'constraint_layout': layout,
                    'variable_ids': ids, 'parameters_mm': values.tolist(),
                    'best_parameters_mm': values.tolist(), 'raw_constraints': constraints(values).tolist(),
                    'initial_scalar_cost': float(initial_scalar_cost), 'final_scalar_cost': float(final_scalar_cost),
                    'requested_minimum': minimum_improvement, 'lower_turn_guard': final_lower_guard,
                    'scope': 'Valid scalar geometry retained diagnostically; requested optimization success NOT admitted'})
            raise ValueError('Feasible geometry did not meet requested minimum fairing improvement')
    if held_selection.get('held_vertices'):
        try:
            next_parameters = forehead_initial_parameters(
                old, points, source_chart, proposal['lineage'], ids) * 1000
            proposal['metrics']['next_stage_seam_hint'] = forehead_next_stage_seam_hint(
                held_selection, next_parameters, ids, frozen_delta[ids],
                points, initial, source_identity)
        except ValueError as error:
            proposal['metrics']['next_stage_hint_unavailable'] = str(error)
    return proposal

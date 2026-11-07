"""Pure bounded mixed lower-rail recovery. No fixtures, files or native calls.
All existing vertices retain frozen ownership; return is LOCAL admission only.
"""
import copy
from forehead_affected_domain import forehead_close_affected_domain
import numpy as np
from forehead_corridor_setup import forehead_corridor_immutable_value, forehead_corridor_decode_source_fields
from forehead_lower_rail_capacity import forehead_lower_rail_capacity, forehead_lower_rail_candidate_demands
from forehead_pattern_catalogue import forehead_pattern_catalogue, forehead_pattern_select_cavity, forehead_pattern_counts, forehead_pattern_edge
from forehead_pattern_solver import forehead_pattern_solve
from forehead_pattern_materialize import forehead_pattern_materialize, forehead_pattern_validate_parent_chains, forehead_pattern_plain_records
from forehead_pattern_initialization import forehead_pattern_support_halo
from forehead_birth_geometry import forehead_birth_solve
from forehead_spacing import forehead_initial_parameters, forehead_frozen_field, measure_forehead_station_spacing
from forehead_flow_rule import forehead_chart, forehead_convex_chart, forehead_no_chart_crossings
from forehead_flow_acceptance import measure_forehead_flow
from forehead_surface_batch import make_forehead_surface_batch
from orbital_support_rule import orbital_quad_quality
from nasal_source_geometry import AuthorQuadSurface

def forehead_lower_rail_recover(proposal, source_vertices, source_fields, source_discrete, source_edge_fields,
    neutral_parent_edges, eligible_source_faces, role, *, max_graph_attempts=2, halo_layers=2, anchor_edges=4,
    graph_seconds=45.0, max_new_points=60, max_extra_route_edges=5, geometry_seconds=120.0,
    geometry_evaluations=(250, 450), width_fraction=0.15, old_halo_layers=2, target_feedback_policy='exclude',
    birth_target_weight=0.08, birth_quality_margin=0.02, provenance=None, source_faces=None):
    """Source samplers use original author data. eligible_source_faces is a
    caller-supplied semantic skin-owner domain, never inferred from numeric IDs.
    Failed attempts return diagnostics, never a committed replacement. Equal-arc
    targets guide allocation only; final whole-flow/gap/arc restoration remains
    external and MUST retain the input's exact owned caps and references.
    """
    proposal = forehead_pattern_plain_records(proposal)
    source_discrete = forehead_pattern_plain_records(source_discrete)
    if target_feedback_policy not in ('exclude', 'diagnose'):
        raise ValueError('Unknown target feedback policy')
    if not 1 <= max_graph_attempts <= 3 or not 0 <= halo_layers <= 4 or (not 0 < width_fraction < 0.5):
        raise ValueError('Invalid bounded local recovery options')
    source_discrete = {key: [forehead_corridor_immutable_value(value) for value in values] for key,
        values in source_discrete.items()}
    base = copy.deepcopy(proposal)
    source = np.asarray(source_vertices, float).copy()
    delta = np.asarray(source_fields, float).copy()
    N = len(base['vertices'])
    chart = np.asarray(base['source_chart'], float)
    v = np.asarray(base['vertices'], float)
    lineage = {int(i): r for i, r in base['lineage'].items()}
    frozen_rows = {int(i): r for i, r in base['topology']['lineage'].items()}
    neutral = {tuple(sorted(e)) for e in neutral_parent_edges}
    params = forehead_initial_parameters(source, v, chart, lineage, list(range(N)))
    frozen = forehead_frozen_field(delta, N, frozen_rows)
    current = np.asarray(base['open_delta'], float)
    field = make_forehead_surface_batch(chart, delta, np.asarray(base['source_faces']))
    surface = AuthorQuadSurface(chart, base['source_faces'], [1] * len(base['source_faces']))
    surface.vertices = source.copy()
    detection = forehead_lower_rail_capacity(v, base['paths'], role, params, current, frozen, field.sample,
        base['protected'], anchor_edges=anchor_edges)
    if not detection['blocked_vertices']:
        return {'admitted': False, 'status': 'no_field_blocker_selected', 'detection': detection}

    def sample(parameter):
        try:
            donors, weights, point = surface.lineage(parameter[0], parameter[2])
        except ValueError:
            return None
        return (np.asarray(point, np.float32).astype(float).tolist(), {'donors': list(donors),
            'weights': list(weights)})

    def quality(points, children):
        angles, ratios = orbital_quad_quality(np.asarray(points), np.asarray(children))
        return float(np.sum(np.maximum(25 - angles, 0) ** 2 + np.maximum(angles - 150,
            0) ** 2) / 100 + np.sum(np.log(ratios) ** 2) / 10)
    eligible = set(eligible_source_faces)
    eligible_current = [fi for fi, owner in enumerate(base['topology']['face_owners']) if owner in eligible]
    segments = {role: tuple(detection['segment'])}
    selected = forehead_pattern_select_cavity(base['faces'], base['paths'], segments, eligible_current,
        halo_layers=halo_layers)
    current_chart = np.column_stack([params[:, 0], np.zeros(N), params[:, 1]])
    blocked = set(detection['blocked_vertices'])
    history = []
    path = base['paths'][role]
    first, last = [path.index(i) for i in segments[role]]
    for attempt in range(max_graph_attempts):
        catalogue = forehead_pattern_catalogue(base, selected, current_chart, sample, source_discrete,
            neutral, segments, blocked_vertices=blocked, quality_cost=quality)
        witness = forehead_pattern_solve(base, catalogue, time_limit=graph_seconds,
            max_new_points=max_new_points, max_route_edges=last - first + max_extra_route_edges)
        record = {'attempt': attempt + 1, 'blocked_vertices': sorted(blocked),
            'graph_status': witness['status'], 'feasible': witness['feasible']}
        history.append(record)
        if not witness['feasible']:
            return {'admitted': False, 'status': 'local_graph_rejected', 'history': history,
                'detection': detection, 'witness': witness}
        candidate, graph_audit = forehead_pattern_materialize(base, witness, source, delta, source_discrete,
            source_edge_fields, neutral)
        feedback = forehead_lower_rail_candidate_demands(v, path, candidate['paths'][role], params, frozen,
            field.sample, N)
        record['recruited_transit_target_capacity'] = feedback
        record['graph_audit'] = graph_audit
        fresh = set(feedback['suggested_search_exclusions']) - blocked
        record['target_feedback_policy'] = target_feedback_policy
        if not fresh or target_feedback_policy == 'diagnose':
            break
        if attempt + 1 == max_graph_attempts:
            return {'admitted': False, 'status': 'recruited_transit_capacity_budget_exhausted',
                'history': history, 'witness': witness}
        blocked |= fresh
    admission, config = forehead_lower_rail_birth_geometry(base, candidate, witness, graph_audit, source,
        delta, source_discrete, neutral, role, segments[role], width_fraction=width_fraction,
        old_halo_layers=old_halo_layers, geometry_seconds=geometry_seconds,
        geometry_evaluations=geometry_evaluations, target_weight=birth_target_weight,
        quality_margin=birth_quality_margin)
    recipe = {'role': role, 'detection': detection, 'segments': segments, 'selected_faces': selected,
        'halo_layers': halo_layers, 'blocked_vertices': sorted(blocked), 'history': history,
        'witness': witness, 'geometry_config': config, 'provenance': copy.deepcopy(provenance)}
    if not admission.get('admitted'):
        return {'admitted': False, 'status': 'local_birth_geometry_rejected', 'recipe': recipe,
            'birth_geometry': admission}
    result, audit = forehead_lower_rail_commit(base, candidate, admission, config, source, delta,
        source_edge_fields, role, source_faces=source_faces)
    return {'admitted': True, 'local_init_only': True, 'fit_complete': False,
        'status': 'local_lower_rail_admitted', 'proposal': result, 'audit': audit, 'recipe': recipe,
        'birth_geometry': admission,
        'scope': 'Strict LOCAL geometry/fields/chains only; original whole-flow/station/arc/gap/independent1mm/pose/native gates remain mandatory'}

def forehead_lower_rail_birth_geometry(base, candidate, witness, graph_audit, source, delta, source_discrete,
    neutral, role, segment, *, width_fraction, old_halo_layers, geometry_seconds, geometry_evaluations,
    target_weight=0.08, quality_margin=0.02):
    base = forehead_pattern_plain_records(base)
    candidate = forehead_pattern_plain_records(candidate)
    witness = forehead_pattern_plain_records(witness)
    graph_audit = forehead_pattern_plain_records(graph_audit)
    source_discrete = forehead_pattern_plain_records(source_discrete)
    p = candidate
    N = len(base['vertices'])
    v = np.asarray(p['vertices'])
    chart = np.asarray(p['source_chart'])
    params = forehead_initial_parameters(source, v, chart, {int(i): r for i, r in p['lineage'].items()},
        list(range(len(v))))
    frozen = forehead_frozen_field(delta, N, {int(i): r for i, r in base['topology']['lineage'].items()})
    reference = copy.deepcopy(source_discrete)
    for key, values in reference.items():
        for i in range(len(source), N):
            row = base['topology']['lineage'].get(i, base['topology']['lineage'].get(str(i)))
            choices = [values[d] for d, w in zip(row['donors'], row['weights']) if w > 0]
            if not choices or any((x != choices[0] for x in choices)):
                raise ValueError('Existing birth has ambiguous semantic ownership')
            values.append(copy.deepcopy(choices[0]))
    remap = {int(i): j for i, j in graph_audit['allocation_to_compact'].items()}
    counts = forehead_pattern_counts([base['faces'][i] for i in witness['selected_base_faces']])
    interior = sorted((e for e, c in counts.items() if c == 2))
    mid_to_edge = {N + k: e for k, e in enumerate(interior)}
    parents = {forehead_pattern_edge(*r['edge']): r['parent'] for r in base['topology']['edge_lineage']}
    dependent = {}
    for raw in witness['new_midpoints']:
        edge = mid_to_edge[raw]
        parent = parents[edge]
        if parent is not None and tuple(sorted(parent)) not in neutral:
            dependent[remap[raw]] = (*edge, 0.5)
    canonical = {i: params[i].copy() for i in range(N, len(v))}
    centers = set(witness['new_centers'])
    for pattern in witness['patterns']:
        if 'transition_rotation' not in pattern:
            continue
        pair = pattern['transition_rotation']['removed']
        c = remap[next((i for i in pair if i in centers))]
        m = remap[next((i for i in pair if i not in centers))]
        direction = canonical[c] - canonical[m]
        canonical[m] -= 2 * width_fraction * direction
        canonical[c] += 2 * width_fraction * direction
    oldpath = base['paths'][role]
    newpath = p['paths'][role]
    start, end = segment
    a, b = (oldpath.index(start), oldpath.index(end))
    held = set(base['protected']) | {i for r,
        path in base['paths'].items() if r != role for i in path} | set(oldpath[:a + 1]) | set(oldpath[b:])
    free = set(range(N, len(v))) - set(dependent)
    movable = forehead_pattern_support_halo(p['faces'], range(N, len(v)), range(N), held,
        layers=old_halo_layers)
    guard = sorted(set(p['affected_new']) | {fi for fi,
        q in enumerate(p['faces']) if set(q) & (set(movable) | set(range(N, len(v))))})
    source_batch = make_forehead_surface_batch(chart, source, np.asarray(p['source_faces']))
    field_batch = make_forehead_surface_batch(chart, delta, np.asarray(p['source_faces']))
    surface = AuthorQuadSurface(chart, p['source_faces'], [1] * len(p['source_faces']))
    surface.vertices = source.copy()

    def row_sampler(xy):
        try:
            donors, weights, point = surface.lineage(*xy)
        except ValueError:
            return None
        return {'donors': list(donors), 'weights': list(weights)}
    oldparams = params[oldpath]
    arc = np.r_[0, np.cumsum(np.linalg.norm(np.diff(np.asarray(base['vertices'])[oldpath], axis=0), axis=1))]
    targets = {i: np.array([np.interp(arc[-1] * k / (len(newpath) - 1), arc, oldparams[:,
        j]) for j in range(2)]) for k, i in enumerate(newpath) if i in free or i in movable}
    config = {'existing_count': N, 'free_births': sorted(free), 'dependent_births': dependent,
        'movable_old': movable, 'held_nodes': sorted(held), 'guard_faces': guard,
        'width_fraction': width_fraction, 'target_weight': target_weight, 'quality_margin': quality_margin,
        'phase_evaluations': list(geometry_evaluations), 'wall_seconds': geometry_seconds,
        'canonical_parameters': {i: x.tolist() for i, x in canonical.items() if i in free},
        'rail_targets': {i: x.tolist() for i, x in targets.items()},
        'target_scope': 'Soft whole-guide targets; fixed local anchors can prevent reaching them. Global restoration must move upstream supports when needed.'}
    admission = forehead_birth_solve(vertices=v, faces=p['faces'], paths=p['paths'], parameters=params,
        existing_count=N, frozen_old_fields=frozen, current_fields=np.asarray(p['open_delta']),
        source_sampler=source_batch.sample, field_sampler=field_batch.sample, row_sampler=row_sampler,
        source_vertices=source, source_fields=delta, source_parameters=chart[:, [0, 2]],
        actual_chart=lambda pts: forehead_chart(pts)[:, [0, 2]], movable_old=movable, held_nodes=held,
        free_births=free, fixed_births=[], dependent_births=dependent, canonical_parameters={i: x for i,
        x in canonical.items() if i in free}, rail_targets=targets, target_weight=target_weight,
        quality_margin=quality_margin, active_roles={role}, guard_face_ids=guard,
        source_discrete=source_discrete, discrete_reference=reference, phase_evaluations=geometry_evaluations,
        wall_seconds=geometry_seconds)
    return (admission, config)

def forehead_lower_rail_commit(base, candidate, admission, config, source, delta, source_edge_fields, role, *, source_faces=None):
    base = forehead_pattern_plain_records(base)
    candidate = forehead_pattern_plain_records(candidate)
    admission = forehead_pattern_plain_records(admission)
    config = forehead_pattern_plain_records(config)
    if not admission.get('admitted'):
        raise ValueError('Cannot commit rejected local geometry')
    p = copy.deepcopy(candidate)
    N = len(base['vertices'])
    chart = np.asarray(p['source_chart'])
    old = np.asarray(base['vertices'])
    for obj in [p, p['topology']]:
        obj['lineage'] = {int(i): r for i, r in obj['lineage'].items()}
    for i, position in admission['positions'].items():
        p['vertices'][int(i)] = list(position)
    for key in ['old_current_row_updates', 'new_birth_rows']:
        for i, row in admission[key].items():
            p['lineage'][int(i)] = copy.deepcopy(row)
    for i, value in admission['current_field_updates'].items():
        p['open_delta'][int(i)] = list(value)
    for i, row in admission['new_birth_rows'].items():
        i = int(i)
        p['topology']['lineage'][i] = copy.deepcopy(row)
        p['topology']['vertices'][i] = list(p['vertices'][i])
        p['topology']['chart'][i] = (np.asarray(row['weights']) @ chart[row['donors']]).tolist()
    for key in ['vertices', 'chart']:
        if p['topology'][key][:N] != base['topology'][key]:
            raise ValueError('Existing frozen geometry changed')
    if any((p['topology']['lineage'][int(i)] != row for i, row in base['topology']['lineage'].items())):
        raise ValueError('Existing frozen row changed')
    for key in ['joint_stage_caps', 'matched_gap_caps_v2', 'uniform_station_reference_length_mm',
        'initial_old_field_baseline']:
        if p.get(key) != base.get(key):
            raise ValueError('Existing cap/reference/baseline changed')
    v = np.asarray(p['vertices'])
    held = config['held_nodes']
    if not np.array_equal(v[held], old[held]):
        raise ValueError('Held geometry changed')
    for other, path in base['paths'].items():
        if other != role and (p['paths'][other] != path or not np.array_equal(v[path], old[path])):
            raise ValueError('Other guide changed')
    for raw, row in {**admission['old_current_row_updates'], **admission['new_birth_rows']}.items():
        i = int(raw)
        weights = np.asarray(row['weights'])
        donors = row['donors']
        if not np.array_equal(np.asarray(weights @ source[donors], np.float32),
            v[i]) or np.linalg.norm(weights @ delta[donors] - p['open_delta'][i]) > 1e-10:
            raise ValueError('Changed row/source position or field mismatch')
    q = np.asarray([p['faces'][i] for i in config['guard_faces']])
    angles, ratio = orbital_quad_quality(v, q)
    cross = forehead_convex_chart(forehead_chart(v), q)
    forehead_no_chart_crossings(forehead_chart(v), q.tolist())
    frozen = forehead_frozen_field(delta, N, {int(i): r for i, r in base['topology']['lineage'].items()})
    drift = np.linalg.norm(np.asarray(p['open_delta'])[:N] - frozen, axis=1) * 1000
    forehead_lower_rail_assert_strict_quality(angles, ratio, drift)
    loci = {}

    def parameter_at(i):
        row = p['lineage'].get(i)
        return np.asarray(row['weights']) @ chart[row['donors']][:, [0, 2]] if row is not None else chart[i,
            [0, 2]]
    for raw_i, (a, b, fraction) in config['dependent_births'].items():
        i = int(raw_i)
        ends = [parameter_at(a), parameter_at(b)]
        birth = parameter_at(i)
        if np.linalg.norm(birth - ((1 - fraction) * ends[0] + fraction * ends[1])) > 1e-08:
            raise ValueError('Final birth-time locus mismatch')
        loci[i] = {'endpoints': [a, b], 'fraction': fraction,
            'endpoint_parameters_at_birth': [x.tolist() for x in ends], 'birth_parameter': birth.tolist(),
            'frozen_birth_row': copy.deepcopy(p['topology']['lineage'][i]),
            'position_at_birth': list(p['vertices'][i]), 'field_baseline_at_birth': list(p['open_delta'][i]),
            'persistent_geometry_constraint': False}
    p.setdefault('local_rail_birth_locus_history', []).append({'role': role, 'existing_count': N,
        'loci': loci,
        'scope': 'Construction-only CURRENT parameter chains; later fitting may change CURRENT lineage under retained frozen fields/parent labels'})
    p['affected_new'] = sorted(set(p['affected_new']) | set(config['guard_faces']))
    p, coverage = forehead_close_affected_domain(p, source, source_faces)
    expanded_quads = np.asarray([p['faces'][i] for i in p['affected_new']])
    expanded_angles, expanded_ratios = orbital_quad_quality(v, expanded_quads)
    forehead_lower_rail_assert_strict_quality(expanded_angles, expanded_ratios, drift)
    forehead_convex_chart(forehead_chart(v), expanded_quads)
    forehead_no_chart_crossings(forehead_chart(v), expanded_quads.tolist())
    angles, ratio = expanded_angles, expanded_ratios
    p['moving'] = sorted(set(p['moving']) | set(config['movable_old']))
    p['support'] = sorted(set(p['moving']) - {i for path in p['paths'].values() for i in path})
    audit = {'scope': 'LOCAL ONLY; existing working caps preserved but NOT asserted satisfied',
        'minimum_angle': float(angles.min()), 'maximum_angle': float(angles.max()),
        'maximum_ratio': float(ratio.max()), 'global_existing_field_mm': float(drift.max()),
        'actual_chart_crossing_check': True,
        'parent_chains': forehead_pattern_validate_parent_chains(p['topology']['edge_lineage'],
        source_edge_fields), 'existing_count': N, 'new_birth_count': len(v) - N,
        'changed_existing_ids': np.flatnonzero(np.any(v[:N] != old, axis=1)).tolist(),
        'existing_frozen_rows_geometry_exact': True, 'other_guides_exact': True,
        'owned_caps_references_exact': True, 'role_physical': measure_forehead_flow(v, p['paths'][role]),
        'role_station': measure_forehead_station_spacing(v, {role: p['paths'][role]})[role],
        'role_arc_mm': float(np.linalg.norm(np.diff(v[p['paths'][role]], axis=0), axis=1).sum() * 1000),
        'construction_only_dependencies': copy.deepcopy(config['dependent_births'])}
    audit['affected_domain_closure'] = coverage
    p['local_rail_birth_admission'] = copy.deepcopy(audit)
    return (p, audit)

def forehead_lower_rail_assert_strict_quality(angles, ratios, existing_field_mm):
    """Numerical optimization buffers never replace these exact hard guards."""
    a = np.asarray(angles, float)
    r = np.asarray(ratios, float)
    f = np.asarray(existing_field_mm, float)
    if not a.size or not r.size or (not f.size) or (not all((np.isfinite(x).all() for x in (a, r,
        f)))) or (a.min() < 25) or (a.max() > 150) or (r.max() > 7) or (f.max() > 0.5):
        raise ValueError('Independent local geometry/field rejection')

def forehead_recover_lower_rail(author, native_source_fields, proposal, role, eligible_source_faces,
    **bounded_options):
    """Minimal source-owned adapter. Semantic source-face eligibility is supplied
    by the caller; source/capture cryptographic provenance remains external.
    Does not load fixtures, reset fields or perform native/global restoration.
    """
    author = forehead_pattern_plain_records(author)
    native_source_fields = forehead_pattern_plain_records(native_source_fields)
    proposal = forehead_pattern_plain_records(proposal)
    source = np.asarray(author['vertices'], float)
    delta = np.asarray(author['open_delta'], float)
    if not np.array_equal(np.asarray(proposal['topology']['vertices'])[:len(source)], source):
        raise ValueError('Original author geometry ownership differs')
    if not np.array_equal(np.asarray(proposal['source_chart']), forehead_chart(source)):
        raise ValueError('Original author chart ownership differs')
    if 'initial_old_field_baseline' in proposal and (not np.array_equal(np.asarray(proposal['initial_old_field_baseline']),
        delta)):
        raise ValueError('Original frozen field ownership differs')
    original_faces = {tuple(q) for q in author['faces']}
    if any((tuple(q) not in original_faces for q in proposal['source_faces'])):
        raise ValueError('Source sampler faces differ from author')
    context = forehead_corridor_decode_source_fields(native_source_fields, len(source), len(author['faces']))
    expected_edges = {forehead_pattern_edge(a, b) for q in author['faces'] for a, b in zip(q, q[1:] + q[:1])}
    if set(context['edge_fields']) != expected_edges:
        raise ValueError('Native edge capture differs from author topology')
    return forehead_lower_rail_recover(proposal, source, delta, context['point_discrete'],
        context['edge_fields'], context['neutral_parent_edges'], eligible_source_faces, role,
        source_faces=author['faces'], **bounded_options)

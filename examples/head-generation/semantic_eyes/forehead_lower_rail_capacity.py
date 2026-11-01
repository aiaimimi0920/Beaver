"""Pure bounded local guide capacity detection; no data files or model IDs."""
import numpy as np

def forehead_lower_rail_capacity(vertices, paths, role, parameters, current_fields, frozen_fields,
    field_sampler, protected, *, budget_mm=0.5, headroom_mm=0.02, anchor_slack_mm=0.1, anchor_edges=4):
    """Flag only near-exhausted current rails whose equal-arc source target
    exceeds their own frozen budget. This is a heuristic recovery selector,
    not a proof that alternate field-isoline/coherent geometry is impossible.
    The field sampler must reject ambiguous/out-of-domain queries.
    """
    v = np.asarray(vertices, float)
    uv = np.asarray(parameters, float)
    current = np.asarray(current_fields, float)
    frozen = np.asarray(frozen_fields, float)
    if budget_mm != 0.5 or not 0 < headroom_mm < budget_mm or (not 0 <= anchor_slack_mm < budget_mm) or (not isinstance(anchor_edges,
        int)) or (anchor_edges < 1):
        raise ValueError('Invalid bounded capacity options')
    path = list(paths[role])
    held = set(protected)
    if len(path) < 4 or len(path) != len(set(path)) or uv.shape != (len(v), 2) or (current.shape != (len(v),
        3)) or (frozen.shape != current.shape):
        raise ValueError('Invalid capacity inputs')
    arc = np.r_[0, np.cumsum(np.linalg.norm(np.diff(v[path], axis=0), axis=1))]
    if np.any(np.diff(arc) <= 1e-09):
        raise ValueError('Collapsed guide station')
    uniform = np.linspace(0, arc[-1], len(path))
    targets = np.column_stack([np.interp(uniform, arc, uv[path, k]) for k in range(2)])
    sample = field_sampler(targets)
    if sample is None:
        raise ValueError('Uniform target has no unambiguous source field')
    drift = np.linalg.norm(current - frozen, axis=1) * 1000
    demand = np.linalg.norm(np.asarray(sample) - frozen[path], axis=1) * 1000
    blocked = [i for j, i in enumerate(path[1:-1],
        1) if i not in held and drift[i] > budget_mm - headroom_mm and (demand[j] > budget_mm)]
    evidence = [{'vertex': i, 'current_drift_mm': float(drift[i]),
        'uniform_target_demand_mm': float(demand[j])} for j, i in enumerate(path)]
    if not blocked:
        return {'blocked_vertices': [], 'segment': None, 'evidence': evidence,
            'scope': 'No blocker selected; not a capacity proof'}
    start = max(1, min((path.index(i) for i in blocked)) - anchor_edges)
    end = min(len(path) - 1, max((path.index(i) for i in blocked)) + 2)
    while start > 1 and drift[path[start]] > anchor_slack_mm:
        start -= 1
    while end < len(path) - 1 and drift[path[end]] > anchor_slack_mm:
        end += 1
    return {'blocked_vertices': blocked, 'segment': (path[start], path[end]), 'evidence': evidence,
        'scope': 'Bounded mixed-rail search selection only; preserve first-two identities, tails, all held points and parent chains'}

def forehead_lower_rail_candidate_demands(vertices, original_path, candidate_path, parameters, frozen_fields,
    field_sampler, existing_count, budget_mm=0.5):
    """Assess newly recruited existing transits against equal-arc station goals.
    Rejection is a bounded search heuristic, not immutable-node infeasibility.
    Slack old transits with compatible target fields remain eligible.
    """
    if budget_mm != 0.5:
        raise ValueError('Do not relax frozen field budget')
    v = np.asarray(vertices, float)
    uv = np.asarray(parameters, float)
    frozen = np.asarray(frozen_fields, float)
    original = list(original_path)
    candidate = list(candidate_path)
    arc = np.r_[0, np.cumsum(np.linalg.norm(np.diff(v[original], axis=0), axis=1))]
    if len(candidate) < 2 or np.any(np.diff(arc) <= 1e-09):
        raise ValueError('Invalid candidate arc')
    rows = []
    for station, i in enumerate(candidate):
        if i >= existing_count or i in original:
            continue
        target = np.array([np.interp(arc[-1] * station / (len(candidate) - 1), arc, uv[original,
            k]) for k in range(2)])
        field = field_sampler(target[None])
        if field is None:
            raise ValueError('Candidate station has no unambiguous source field')
        demand = float(np.linalg.norm(field[0] - frozen[i]) * 1000)
        rows.append({'vertex': i, 'target_parameter': target.tolist(), 'target_field_demand_mm': demand,
            'target_incompatible': demand > budget_mm})
    return {'newly_recruited_existing_transits': rows,
        'suggested_search_exclusions': [r['vertex'] for r in rows if r['target_incompatible']],
        'scope': 'Target-informed bounded search refinement only; does not prove every near-uniform placement impossible'}

def forehead_lower_rail_lateral_transits(vertices, original_path, candidate_path, existing_count,
    width_fraction=1.5):
    """Optional candidate enumeration after a proved local detour: avoid newly
    recruited OLD stations farther than a bounded local rail-width guess.
    Does not prohibit slack transits generally or prove geometry infeasible.
    """
    v = np.asarray(vertices, float)
    original = list(original_path)
    a = v[original[:-1]]
    edges = np.diff(v[original], axis=0)
    length2 = np.sum(edges * edges, axis=1)
    if np.any(length2 <= 1e-18) or not 0 < width_fraction <= 3:
        raise ValueError('Invalid bounded lateral corridor')
    width = float(np.median(np.sqrt(length2)))
    rows = []
    for i in candidate_path:
        if i >= existing_count or i in original:
            continue
        fraction = np.clip(np.sum((v[i] - a) * edges, axis=1) / length2, 0, 1)
        distance = float(np.linalg.norm(v[i] - (a + fraction[:, None] * edges), axis=1).min())
        rows.append({'vertex': i, 'distance_to_prior_rail_mm': 1000 * distance,
            'median_rail_edge_mm': 1000 * width, 'outside_local_guess': distance > width_fraction * width})
    return {'rows': rows,
        'suggested_search_exclusions': [r['vertex'] for r in rows if r['outside_local_guess']],
        'scope': 'Heuristic alternate-candidate enumeration; original vertices/fields/parent chains retained as supports'}

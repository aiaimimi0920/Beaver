"""Pure conservative actual-adjacency rebinding; preserves actual-guide obligations."""
import copy
import numpy as np
from forehead_lower_guard import forehead_lower_guard_topology, forehead_build_lower_guard, forehead_update_lower_guard, forehead_validate_lower_guard
from orbital_support_rule import orbital_turns

def forehead_lower_rail_rebind_guard(before, proposal, prior_triples_by_role, source_reference_points, *,
    origin_provenance=None):
    """Returns a copied LOCAL proposal and canonical adjunct evidence.
    Prior actual-adjacent triples are supplied explicitly, never recomputed from
    current height. Changed intervals use nearest ordered surviving anchors.
    This is conservative replacement COVERAGE, not a unique station map or proof
    of quality or global acceptance. Root coverage review required.
    """
    if not set(prior_triples_by_role) <= set(('blue', 'purple')):
        raise ValueError('Unsupported lower-guard role')
    p = copy.deepcopy(proposal)
    oldv = np.asarray(before['vertices'])
    v = np.asarray(p['vertices'])
    reference = np.asarray(source_reference_points, float)
    if reference.shape != v.shape:
        raise ValueError('Missing source-owned new-point references')
    _, _, old_hash = forehead_lower_guard_topology(before)
    mapped = set()
    records = []
    survives = 0
    retired = 0
    for role, triples in prior_triples_by_role.items():
        old = list(before['paths'][role])
        new = list(p['paths'][role])
        oldadj = {tuple(t) for t in zip(old, old[1:], old[2:])}
        newadj = {tuple(t) for t in zip(new, new[1:], new[2:])}
        common = set(old) & set(new)
        if [i for i in old if i in common] != [i for i in new if i in common]:
            raise ValueError('Changed common-vertex source ordering requires explicit correspondence')
        for raw in triples:
            t = tuple(map(int, raw))
            if t not in oldadj:
                raise ValueError('Prior actual guard is not old-path adjacency; retain historical nonadjacent evidence separately')
            record = {'role': role, 'original_triple': list(t),
                'before_angle_degrees': float(orbital_turns(oldv, np.asarray([t]))[0]),
                'current_numeric_angle_degrees': float(orbital_turns(v, np.asarray([t]))[0]),
                'numeric_limit_degrees': 20.0, 'retired_numeric_is_offguide_diagnostic': t not in newadj}
            if t in newadj:
                replacements = [t]
                span = list(t)
                record.update(status='survives',
                    source_interval_justification='Identical ordered actual path triple')
                survives += 1
            else:
                first, last = (old.index(t[0]), old.index(t[-1]))
                lower = next((i for i in range(first, -1, -1) if old[i] in common), None)
                upper = next((i for i in range(last, len(old)) if old[i] in common), None)
                if lower is None or upper is None:
                    raise ValueError('Retired guard has no surviving source interval anchors')
                a, b = (new.index(old[lower]), new.index(old[upper]))
                span = new[a:b + 1]
                replacements = [tuple(t) for t in zip(span, span[1:], span[2:])]
                if not replacements:
                    raise ValueError('Retired guard has no actual adjacent replacement coverage')
                record.update(status='replaced', source_order_bracket=[old[lower], old[upper]],
                    source_interval_justification='Conservative nearest ordered surviving-anchor interval; all new adjacent triples covered, not a unique station correspondence')
                retired += 1
            mapped.update(replacements)
            record['actual_new_path_span'] = span
            record['replacement_adjacent_triples'] = [list(t) for t in replacements]
            record['replacement_angles_degrees'] = orbital_turns(v, np.asarray(replacements)).tolist()
            records.append(record)
    guard = forehead_build_lower_guard(p, reference, inherited_triples=sorted(mapped),
        reason='Explicit source-owned topology rebind; see lower_turn_guard_rebind_evidence; retired nonadjacent numeric angles remain off-guide diagnostics')
    guard = forehead_update_lower_guard(p, v, guard)
    actual = forehead_validate_lower_guard(p, guard)
    angles = orbital_turns(v, np.asarray(actual))
    evidence = {'schema_version': 1, 'prior_topology_sha256': old_hash,
        'origin_provenance': copy.deepcopy(origin_provenance),
        'prior_actual_triples_by_role': copy.deepcopy(prior_triples_by_role), 'records': records,
        'surviving_original_count': survives, 'retired_original_count': retired,
        'new_active_adjacent_guard_count': len(actual), 'new_active_max_angle_degrees': float(angles.max()),
        'new_active_violations': [{'triple': list(t), 'angle_degrees': float(a)} for t, a in zip(actual,
        angles) if a > 20.0], 'retired_numeric_violations': [{'triple': r['original_triple'],
        'angle_degrees': r['current_numeric_angle_degrees']} for r in records if r['status'] == 'replaced' and r['current_numeric_angle_degrees'] > 20.0],
        'retired_numeric_rows_enforced_by_primary_adjacency_guard': False, 'coverage_review_required': True,
        'scope': 'Actual guide adjacency carries the 20-degree obligation. Retired nonadjacent angles remain recorded off-guide diagnostics. Conservative replacement coverage requires explicit review; no coordinate, field, chain, face-quality, or shape constraint is relaxed.'}
    if 'lower_turn_guard_rebind_evidence' in before:
        evidence['previous_rebind_evidence'] = copy.deepcopy(before['lower_turn_guard_rebind_evidence'])
    p['lower_turn_guard'] = guard
    p['lower_turn_guard_rebind_evidence'] = evidence
    return (p, evidence)

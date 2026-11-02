"""Bounded equal-arc redistribution on immutable source polylines.

No topology creation, character adapter, face-area coverage, global shape, or native
admission is implemented here. Full-channel adoption is deliberately forbidden.
"""
import math
from .geometry import finite_vector, validate_row, quad_metrics, edge_metrics, arc, cross, dot, evaluate, metrics, norm, row_at, sub
from .packet import corner_value, digest

def initial(source):
    p = source.data
    return {'source': source.sha256,
        'baseline': digest(p['frozen_fields']),
        'baseline_roles': digest(p['baseline_roles']),
        'rows': {uid: ((uid, 1.0),) for uid in p['points']}}

def validate_state(source, state):
    p = source.data
    if state['source'] != source.sha256 or state['baseline'] != digest(p['frozen_fields']) or state['baseline_roles'] != digest(p['baseline_roles']):
        raise ValueError('rebased state')
    if set(state['rows']) != set(p['points']):
        raise ValueError('point identity coverage')
    members = {uid: path for path in p['rails'].values() for uid in path}
    for uid, row in state['rows'].items():
        validate_row(row, p['points'])
        if uid in p['protected'] or uid not in members:
            if list(map(list, row)) != [[uid, 1.0]]:
                raise ValueError('protected/unsupported movement')
        else:
            path = members[uid]
            ids = [v for v, _ in row]
            if any((v not in path for v in ids)) or (len(ids) == 2 and path.index(ids[1]) != path.index(ids[0]) + 1):
                raise ValueError('nonadjacent source donors')
    for path in p['rails'].values():
        a = arc(p['points'], path)
        lookup = dict(zip(path, a))
        stations = [sum((lookup[v] * w for v, w in state['rows'][uid])) for uid in path]
        if any((b <= a for a, b in zip(stations, stations[1:]))):
            raise ValueError('station order')

def materialize(source, state):
    validate_state(source, state)
    p = source.data
    points = {uid: evaluate(p['points'], row) for uid, row in state['rows'].items()}
    fields = {uid: {key: evaluate({v: f[key] for v, f in p['frozen_fields'].items()}, row) for key in p['keys']} for uid,
        row in state['rows'].items()}
    keys = {key: {uid: finite_vector((points[uid][k] + fields[uid][key][k] for k in range(3))) for uid in points} for key in p['keys']}
    corners = {}
    for owner, f in p['faces'].items():
        corners[owner] = []
        for c in f['corners']:
            uid = c['vertex']
            row = state['rows'][uid]
            if list(map(list, row)) == [[uid, 1.0]]:
                corners[owner].append({'side': c['side'],
                    'category': f['category'],
                    'source_owners': [owner],
                    'uv': c['uv']})
            else:
                corners[owner].append(corner_value(source, row, c['side'], f['category']))
    return {'points': points, 'fields': fields, 'keys': keys, 'corners': corners}

def field_error(source, material):
    p = source.data
    return max((norm(sub(material['fields'][uid][key], f[key])) for uid,
        f in p['frozen_fields'].items() for key in p['keys']))

def oriented(source, points):
    p = source.data
    for f in p['faces'].values():
        v = f['vertices']
        diagonal = set(f['diagonal'])
        start = next((j for j in range(4) if {v[j], v[(j + 2) % 4]} == diagonal))
        w = v[start:] + v[:start]
        for a, b, c in [(w[0], w[1], w[2]), (w[0], w[2], w[3])]:
            old = cross(sub(p['points'][b], p['points'][a]), sub(p['points'][c], p['points'][a]))
            new = cross(sub(points[b], points[a]), sub(points[c], points[a]))
            if norm(old) == 0 or norm(new) == 0 or dot(old, new) <= 0:
                return False
    return True

def rail_nonworse(candidate, reference):
    return all((candidate[k] <= reference[k] + 1e-12 for k in ('ratio',
        'worst_relative',
        'peak_turn')))

def redistribute(source,
    binding,
    state=None,
    max_step=0.003,
    field_limit=0.0005,
    trials=16,
    policy='RAIL_PROPOSAL_V1'):
    source.check_binding(binding)
    if policy not in ('RAIL_PROPOSAL_V1', 'SYNTHETIC_JOINT_NONREGRESSION_V1'):
        raise ValueError('unknown policy')
    if not math.isfinite(max_step) or max_step <= 0 or (not 0 < field_limit <= 0.0005) or (not isinstance(trials,
        int)) or (not 1 <= trials <= 32):
        raise ValueError('bounded policy')
    state = initial(source) if state is None else state
    before = materialize(source, state)
    p = source.data
    transverse = source.transverse_domains()
    if field_error(source, before) > field_limit:
        raise ValueError('input original field budget exceeded')
    active = {name: path for name,
        path in p['rails'].items() if any((uid not in p['protected'] for uid in path))}
    if not active:
        raise ValueError('no movable rail')
    old_metrics = {name: metrics(before['points'], path) for name, path in p['rails'].items()}
    old_gap = {n: edge_metrics(before['points'], e) for n, e in transverse.items()}
    old_quad = quad_metrics(before['points'], p['faces'])
    frozen_gap = {n: edge_metrics(p['points'], e) for n, e in transverse.items()}
    frozen_quad = quad_metrics(p['points'], p['faces'])
    if policy == 'SYNTHETIC_JOINT_NONREGRESSION_V1':
        baseline_ok = all((old_gap[n]['ratio'] <= frozen_gap[n]['ratio'] + 1e-12 and old_gap[n]['worst_relative'] <= frozen_gap[n]['worst_relative'] + 1e-12 for n in old_gap))
        baseline_ok = baseline_ok and old_quad['max_aspect'] <= frozen_quad['max_aspect'] + 1e-12 and (old_quad['min_angle'] >= frozen_quad['min_angle'] - 1e-12) and (old_quad['max_angle'] <= frozen_quad['max_angle'] + 1e-12)
        baseline_ok = baseline_ok and all((rail_nonworse(old_metrics[n], metrics(p['points'], path)) for n,
            path in active.items()))
        if not baseline_ok:
            raise ValueError('input violates frozen joint bounds')
    for attempt in range(trials):
        step = 2.0 ** (-attempt)
        rows = dict(state['rows'])
        for name, path in active.items():
            distances = arc(p['points'], path)
            lookup = dict(zip(path, distances))
            anchors = [j for j, uid in enumerate(path) if uid in p['protected']]
            for left, right in zip(anchors, anchors[1:]):
                for j in range(left + 1, right):
                    uid = path[j]
                    old = sum((lookup[v] * w for v, w in state['rows'][uid]))
                    target = distances[left] + (distances[right] - distances[left]) * (j - left) / (right - left)
                    rows[uid] = row_at(path, distances, old + step * (target - old))
        candidate = {'source': source.sha256,
            'baseline': state['baseline'],
            'baseline_roles': state['baseline_roles'],
            'rows': rows}
        try:
            result = materialize(source, candidate)
        except ValueError:
            continue
        max_move = max((norm(sub(result['points'][uid], before['points'][uid])) for uid in rows))
        if max_move > max_step or field_error(source,
            result) > field_limit or (not oriented(source, result['points'])):
            continue
        new_metrics = {name: metrics(result['points'], path) for name, path in p['rails'].items()}
        strict = any((new_metrics[n]['ratio'] < old_metrics[n]['ratio'] - 1e-10 for n in active))
        nonworse = all((rail_nonworse(new_metrics[n], old_metrics[n]) for n in active))
        if not strict or not nonworse:
            continue
        gap = {n: edge_metrics(result['points'], e) for n, e in transverse.items()}
        quad = quad_metrics(result['points'], p['faces'])
        joint = all((gap[n]['ratio'] <= old_gap[n]['ratio'] + 1e-12 and gap[n]['worst_relative'] <= old_gap[n]['worst_relative'] + 1e-12 for n in gap))
        joint = joint and quad['max_aspect'] <= old_quad['max_aspect'] + 1e-12 and (quad['min_angle'] >= old_quad['min_angle'] - 1e-12) and (quad['max_angle'] <= old_quad['max_angle'] + 1e-12)
        if policy == 'SYNTHETIC_JOINT_NONREGRESSION_V1' and (not joint):
            continue
        changed = sorted((uid for uid in rows if rows[uid] != state['rows'][uid]))
        affected = sorted((owner for owner,
            f in p['faces'].items() if set(f['vertices']).intersection(changed)))
        return {'state': candidate,
            'materialized': result,
            'before': old_metrics,
            'after': new_metrics,
            'max_step': max_move,
            'original_field_error': field_error(source, result),
            'readset': binding,
            'changed_points': changed,
            'incident_faces': affected,
            'transverse_before': {n: edge_metrics(before['points'], e) for n, e in transverse.items()},
            'transverse_after': {n: edge_metrics(result['points'], e) for n, e in transverse.items()},
            'policy': policy,
            'synthetic_joint_admitted': policy == 'SYNTHETIC_JOINT_NONREGRESSION_V1' and joint,
            'quad_before': old_quad,
            'quad_after': quad,
            'topology_changed': False,
            'native_ready': False,
            'full_model_success': False,
            'pending': ['full face-domain coverage', 'all-channel transport', '11-pose bidirectional 1mm', '729 orientation', 'native save/readback', 'GLB/current-engine', 'visual acceptance']}
    raise ValueError('no admitted bounded rail step')

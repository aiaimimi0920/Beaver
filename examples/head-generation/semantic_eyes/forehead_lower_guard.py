"""Retained lower-guide membership: geometry may add guards, never remove them."""
import copy
import hashlib
import json
import numpy as np


def forehead_lower_guard_topology(proposal):
    paths = {name: list(map(int, proposal['paths'][name])) for name in ('blue', 'purple')}
    faces = [list(map(int, q)) for q in proposal['faces']]
    edges = {tuple(sorted((a, b))) for q in faces for a, b in zip(q, q[1:]+q[:1])}
    triples = []
    for path in paths.values():
        if len(path) < 3 or len(set(path)) != len(path) or any(tuple(sorted(e)) not in edges for e in zip(path, path[1:])):
            raise ValueError('Lower guard requires complete actual source-edge paths')
        triples.extend(tuple(t) for t in zip(path, path[1:], path[2:]))
    digest = hashlib.sha256(json.dumps({'faces': faces, 'paths': paths}, separators=(',', ':')).encode()).hexdigest()
    return paths, triples, digest


def forehead_validate_lower_guard(proposal, guard):
    paths, adjacent, digest = forehead_lower_guard_topology(proposal)
    stored_paths = {name: list(map(int, path)) for name, path in guard.get('paths', {}).items()}
    if (guard.get('schema_version') != 1 or guard.get('topology_sha256') != digest
            or stored_paths != paths or guard.get('cutoff_height') != float(proposal['mouth_height']+.03)):
        raise ValueError('Lower guard topology/cutoff changed; explicit source-derived rebuild required')
    triples = [tuple(map(int, t)) for t in guard['triples']]
    if not triples or len(set(triples)) != len(triples) or not set(triples) <= set(adjacent):
        raise ValueError('Retained lower guard has missing/invalid path triples')
    return triples


def forehead_build_lower_guard(proposal, reference_points, inherited_triples=(), reason='source-initialization'):
    """Explicit initialization/rebuild on the ACTUAL new path adjacency.

    For topology edits the caller supplies frozen-source-derived reference points
    and correctly rebased inherited triples, with an explicit reason. This does
    not authorize dropping old requirements or certify the topology transaction.
    """
    if not isinstance(reason, str) or not reason.strip():
        raise ValueError('Explicit lower-guard source/rebuild reason required')
    paths, adjacent, digest = forehead_lower_guard_topology(proposal)
    points = np.asarray(reference_points, float)
    cutoff = float(proposal['mouth_height']+.03)
    inherited = {tuple(map(int, t)) for t in inherited_triples}
    if not inherited <= set(adjacent):
        raise ValueError('Inherited triples need explicit actual-adjacency rebasing')
    if not np.isfinite(cutoff) or not np.isfinite(points).all():
        raise ValueError('Nonfinite lower-guard reference')
    members = inherited | {t for t in adjacent if points[t[1], 2] < cutoff}
    guard = {'schema_version': 1, 'topology_sha256': digest, 'paths': paths,
             'cutoff_height': cutoff, 'triples': [list(t) for t in adjacent if t in members],
             'reference_reason': reason, 'geometry_updates_only_add_members': True}
    forehead_validate_lower_guard(proposal, guard)
    return guard


def forehead_update_lower_guard(proposal, points, guard):
    """Union current entrants with retained members; topology change fails closed."""
    retained = set(forehead_validate_lower_guard(proposal, guard))
    _, adjacent, _ = forehead_lower_guard_topology(proposal)
    points = np.asarray(points, float)
    if not np.isfinite(points).all():
        raise ValueError('Nonfinite lower-guard geometry')
    members = retained | {t for t in adjacent if points[t[1], 2] < guard['cutoff_height']}
    result = copy.deepcopy(guard)
    result['triples'] = [list(t) for t in adjacent if t in members]
    return result


def forehead_lower_guard_retains(before, after):
    """Geometry-only continuation cannot erase membership or rebuild silently."""
    if 'lower_turn_guard' not in before:
        return
    old = before['lower_turn_guard']
    new = after.get('lower_turn_guard')
    if new is None:
        raise ValueError('Continuation removed retained lower guard')
    old_rows = set(forehead_validate_lower_guard(before, old))
    new_rows = set(forehead_validate_lower_guard(after, new))
    if (old['topology_sha256'] != new['topology_sha256'] or
            old['cutoff_height'] != new['cutoff_height'] or not old_rows <= new_rows):
        raise ValueError('Continuation changed or dropped retained lower membership')
    required = forehead_update_lower_guard(after, after['vertices'], old)
    if not {tuple(t) for t in required['triples']} <= new_rows:
        raise ValueError('Continuation failed to retain newly entering lower triples')

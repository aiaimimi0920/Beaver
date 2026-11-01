"""Explicit cross-phase fairing obligations and support-topology migration."""
import copy as forehead_guard_copy
import hashlib as forehead_guard_hashlib
import json as forehead_guard_json
import numpy as np
from forehead_fairing_stage import ForeheadFairingStage
from forehead_role_fairing import ForeheadRoleFairingStage
from orbital_support_rule import orbital_turns


def forehead_fairing_topology_identity(proposal):
    data = {'count': len(proposal['vertices']), 'faces': [list(map(int, f)) for f in proposal['faces']],
            'paths': {r: list(map(int, p)) for r, p in proposal['paths'].items()}}
    return forehead_guard_hashlib.sha256(forehead_guard_json.dumps(data, sort_keys=True, separators=(',', ':')).encode()).hexdigest()


def forehead_prepare_fairing_stage(proposal, points, reduction=None, weights=None,
                                  turn_reduction=0., turn_goals=None, turn_roles=None, guard_mode=None):
    guard = proposal.get('whole_guide_fairing_guard')
    reference = proposal.get('whole_guide_fairing_reference')
    version = 1 if guard is None else guard.get('version')
    modes = {1:'pointwise-v1', 2:'role-max-bv-v2'}
    if type(version) is not int or version not in modes or (guard_mode is not None and guard_mode != modes[version]):
        raise ValueError('Unknown fairing mode or explicit guard migration required')
    if guard is None and reference is not None:
        raise ValueError('Fairing reference lacks explicit retained guard capture')
    if guard is None and reduction is None:
        return None
    if guard is not None:
        if guard.get('topology_identity') != forehead_fairing_topology_identity(proposal):
            raise ValueError('Fairing guard needs explicit topology migration')
        if reference != guard.get('reference'):
            raise ValueError('Frozen fairing normalization changed')
    stage_class = ForeheadFairingStage if version == 1 else ForeheadRoleFairingStage
    stage = stage_class(points, proposal['paths'], proposal['uniform_station_reference_length_mm'],
        reference, 0. if reduction is None else reduction, weights, turn_reduction, turn_goals, turn_roles)
    if guard is not None:
        caps = np.asarray(guard['turn_caps_degrees'], float)
        energy = float(guard['energy_cap'])
        if caps.shape != stage.turn_caps.shape or not np.isfinite(caps).all() or np.any(caps < 0) or not np.isfinite(energy) or energy < 0:
            raise ValueError('Invalid retained fairing caps')
        if guard.get('triples') != stage.triples.tolist():
            raise ValueError('Retained turn adjacency changed')
        # Ordinary phases retain the exact admitted caps, not newly measured caps.
        stage.turn_caps = caps.copy() if reduction is None else np.minimum(caps, stage.turn_caps)
        stage.target_energy = energy if reduction is None else min(energy, stage.target_energy)
        if version == 2:
            role_caps = guard['role_max_caps_degrees']
            bv_caps = np.asarray(guard['bending_variation_caps'], float)
            if set(role_caps) != set(stage.paths) or any(not np.isfinite(v) or v < 0 for v in role_caps.values()):
                raise ValueError('Invalid retained role maximum caps')
            repeated = np.concatenate([np.full(len(path)-2,role_caps[r]) for r,path in stage.paths.items()])
            if not np.array_equal(caps,repeated) or bv_caps.shape != stage.bv_caps.shape or not np.isfinite(bv_caps).all() or np.any(bv_caps < 0):
                raise ValueError('Invalid retained role B/V caps or ownership')
            stage.set_role_caps(role_caps if reduction is None else {r:min(role_caps[r],stage.role_caps[r]) for r in stage.paths})
            stage.bv_caps = bv_caps.copy() if reduction is None else np.minimum(stage.bv_caps,bv_caps)
    return stage


def forehead_capture_fairing_guard(proposal, stage=None):
    """Return guard only; caller persists it AFTER all scalar admission gates.

    Legacy component capture requires its saved fairing evidence and rechecks it;
    it is not permission to invent a normalization from an arbitrary proposal.
    """
    p = np.asarray(proposal['vertices'], float)
    if stage is None:
        if proposal.get('whole_guide_fairing_guard', {}).get('version') == 2:
            raise ValueError('Existing v2 guard requires explicit v2 stage capture')
        evidence = proposal.get('metrics', {}).get('guide_fairing', {})
        reference = proposal.get('whole_guide_fairing_reference')
        if not reference or evidence.get('persistent_reference') != reference:
            raise ValueError('Missing admitted fairing evidence for capture')
        stage = ForeheadFairingStage(p, proposal['paths'], proposal['uniform_station_reference_length_mm'], reference, 0.)
        stage.target_energy = float(evidence['target_energy'])
        stage.turn_caps = np.asarray(evidence['raw_turn_caps_degrees'], float)
    if not stage.accepts(p):
        raise ValueError('Cannot capture unadmitted fairing geometry')
    guard = {'version': 1, 'topology_identity': forehead_fairing_topology_identity(proposal),
            'reference': forehead_guard_copy.deepcopy(stage.reference), 'triples': stage.triples.tolist(),
            'energy_cap': stage.energy_gradient(p)[0],
            'turn_caps_degrees': orbital_turns(p, stage.triples).tolist()}
    if isinstance(stage, ForeheadRoleFairingStage):
        previous = proposal['whole_guide_fairing_guard']
        metrics = stage.role_metrics(p)
        guard.update(version=2, semantics='role-max-and-separate-B-V',
            role_max_caps_degrees={r:m['local_peak_degrees'] for r,m in metrics.items()},
            bending_variation_caps=stage.bv_world(p)[0].tolist(),
            turn_caps_degrees=np.concatenate([np.full(len(path)-2,metrics[r]['local_peak_degrees']) for r,path in stage.paths.items()]).tolist(),
            migration=forehead_guard_copy.deepcopy(previous['migration']),
            retired_pointwise_v1_guard=forehead_guard_copy.deepcopy(previous['retired_pointwise_v1_guard']))
    return guard


def forehead_migrate_support_fairing_guard(before, after):
    """Explicit support-only migration: same IDs/guide coordinates and old rows.

    This does not admit new topology, fields, UVs, quality or shape. Their complete
    operator admission is separately mandatory. Guide rerouting is forbidden.
    """
    if before['paths'] != after['paths'] or before.get('whole_guide_fairing_reference') != after.get('whole_guide_fairing_reference'):
        raise ValueError('Guide topology/normalization migration requires separate proof')
    nodes = sorted({i for p in before['paths'].values() for i in p})
    if not np.array_equal(np.asarray(before['vertices'])[nodes], np.asarray(after['vertices'])[nodes]):
        raise ValueError('Support migration moved a guide')
    old_rows = before['topology']['lineage']; new_rows = after['topology']['lineage']
    new_rows = {int(i): row for i, row in new_rows.items()}
    for i, row in old_rows.items():
        other = new_rows.get(int(i))
        if other is None or not np.array_equal(row['donors'], other['donors']) or not np.array_equal(row['weights'], other['weights']):
            raise ValueError('Support migration changed old frozen field rows')
    guard = forehead_guard_copy.deepcopy(before['whole_guide_fairing_guard'])
    forehead_prepare_fairing_stage(before, np.asarray(before['vertices']))
    guard['topology_identity'] = forehead_fairing_topology_identity(after)
    return guard


def forehead_fairing_guard_retains(before, after):
    """Independent continuation admission; no scheduling/target policy change."""
    if 'whole_guide_fairing_guard' not in before:
        if 'whole_guide_fairing_reference' in before:
            raise ValueError('Missing explicit legacy fairing guard capture')
        return
    previous = forehead_prepare_fairing_stage(before, np.asarray(before['vertices']))
    current = forehead_prepare_fairing_stage(after, np.asarray(after['vertices']))
    if current is None or current.reference != previous.reference:
        raise ValueError('Continuation discarded fairing normalization/guards')
    if before['whole_guide_fairing_guard']['version'] != after['whole_guide_fairing_guard']['version']:
        raise ValueError('Continuation changed guard semantics without explicit migration')
    if current.target_energy > previous.target_energy or np.any(current.turn_caps > previous.turn_caps) or (isinstance(current, ForeheadRoleFairingStage) and np.any(current.bv_caps > previous.bv_caps)):
        raise ValueError('Continuation relaxed retained fairing caps')
    if not current.accepts(np.asarray(after['vertices'])):
        raise ValueError('Continuation violates saved fairing guards')


def forehead_migrate_fairing_guard_v2(proposal):
    """Explicit v1 retirement; require old saved geometry to pass every v1 row.

    Return metadata only. Geometry, frozen normalizers and all model gates remain
    unchanged. Subsequent v2 results may redistribute individual turns and must
    never be described as satisfying retired pointwise-v1 constraints.
    """
    old = proposal.get('whole_guide_fairing_guard', {})
    if old.get('version') != 1:
        raise ValueError('Migration requires an explicit admitted v1 guard')
    points = np.asarray(proposal['vertices'], float)
    stage = forehead_prepare_fairing_stage(proposal, points)
    if not stage.accepts(points):
        raise ValueError('Cannot retire violated v1 fairing guards')
    metrics = stage.role_metrics(points)
    new = forehead_guard_copy.deepcopy(old)
    new.update(version=2, semantics='role-max-and-separate-B-V',
        role_max_caps_degrees={r:m['local_peak_degrees'] for r,m in metrics.items()},
        bending_variation_caps=np.array([[metrics[r]['bending'],metrics[r]['variation']] for r in stage.paths]).ravel().tolist(),
        turn_caps_degrees=np.concatenate([np.full(len(path)-2,metrics[r]['local_peak_degrees']) for r,path in stage.paths.items()]).tolist(),
        migration={'old_guard_sha256':forehead_guard_hashlib.sha256(forehead_guard_json.dumps(old,sort_keys=True,separators=(',',':'),default=forehead_fairing_json_value).encode()).hexdigest(),
                   'old_semantics':'pointwise-v1','new_semantics':'role-max-and-separate-B-V-v2',
                   'normalization_reset':False,'pointwise_v1_not_claimed_for_future_results':True},
        retired_pointwise_v1_guard=forehead_guard_copy.deepcopy(old))
    return new


def forehead_fairing_json_value(value):
    """Canonical numeric serialization for immutable guard evidence hashes."""
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, np.generic):
        return value.item()
    raise TypeError('Unsupported fairing evidence value')

"""Final shared-guide goals, independent of permissive continuation caps."""
import numpy as np
from forehead_flow_acceptance import measure_forehead_flow
from forehead_physical_stage import ForeheadPhysicalFlowStage
from forehead_matched_gap_stage import ForeheadMatchedGapStage
from forehead_matched_gap_validation import forehead_matched_gap_dense_errors


def measure_forehead_shared_acceptance(author, proposal, atlas=None):
    """Measure actual reconstructed points. Never upgrade a working-stage pass."""
    points = np.asarray(proposal['vertices'], dtype=float)
    source = np.asarray(proposal['source_chart'], dtype=float)
    paths = proposal['paths']
    names = ('red', 'orange', 'green', 'blue', 'purple')
    if set(paths) != set(names) or not np.isfinite(points).all():
        raise ValueError('Shared acceptance needs five finite, source-owned guides')
    if len(source) != len(author['vertices']) or len(points) < len(source):
        raise ValueError('Shared acceptance source ownership differs')
    params = np.vstack((source, np.zeros((len(points)-len(source), 3))))
    for index, row in proposal['lineage'].items():
        params[int(index)] = np.asarray(row['weights']) @ source[row['donors']]
    stage = ForeheadMatchedGapStage(author, proposal, params, reduction=0., atlas=atlas)
    gaps = forehead_matched_gap_dense_errors(stage, points, params)
    goals = dict(zip(names, (18., 20., 25., 30., 33.)))
    reference = proposal.get('uniform_station_reference_length_mm')
    if reference is None or set(reference) != set(paths):
        raise ValueError('Frozen guide length references are required for final admission')
    flows, stations, failures = {}, {}, []
    old = np.asarray(author['vertices'])
    for name, path in paths.items():
        flows[name] = measure_forehead_flow(points, path)
        lengths = np.linalg.norm(np.diff(points[path], axis=0), axis=1)
        deviation = float(np.max(np.abs(lengths/lengths.mean()-1)))
        if not np.isfinite(reference[name]) or reference[name] <= 0:
            raise ValueError('Invalid frozen guide length reference')
        inflation = float(lengths.sum()*1000/reference[name])
        front_cap = max(6., ForeheadPhysicalFlowStage.front_turn(old[path[:2]]))
        front = ForeheadPhysicalFlowStage.front_turn(points[path[:2]])
        stations[name] = {'maximum_relative_mean_deviation': deviation,
                          'length_inflation_ratio': inflation,
                          'front_turn_degrees': front, 'front_cap_degrees': front_cap}
        if flows[name]['sampled_peak_degrees'] > goals[name]:
            failures.append('physical:'+name)
        if deviation > .1:
            failures.append('station:'+name)
        if inflation > 1.05:
            failures.append('length:'+name)
        if front > front_cap+1e-8:
            failures.append('front:'+name)
    if gaps is None:
        failures.append('invalid_source_atlas')
    else:
        failures.extend('gap:'+domain for domain, error in gaps.items() if error > .1)
    return {'passed': not failures, 'failures': failures, 'flows': flows,
            'stations': stations, 'shared_gap_maximum_relative_deviation': gaps,
            'physical_caps_degrees': goals, 'relative_uniformity_cap': .1,
            'maximum_length_inflation_ratio': 1.05,
            'atlas_sections_per_direction': 1025,
            'includes_all_in_domain_source_vertices': True,
            'working_caps_do_not_define_final_acceptance': True,
            'scope': 'Guide geometry only. Independent posed surface error, source fields, '
                     'saved native mesh, GLB, engine and visual acceptance remain required.'}


def validate_forehead_shared_acceptance(author, proposal, atlas=None):
    result = measure_forehead_shared_acceptance(author, proposal, atlas)
    if not result['passed']:
        raise ValueError('Final shared guide goals failed: '+', '.join(result['failures']))
    return result

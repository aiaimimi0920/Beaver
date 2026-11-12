"""Fixed physical-scale flow evidence, distinct from solver objective samples."""
import numpy as np
from forehead_spacing import measure_forehead_station_spacing
from forehead_gap_measure import measure_forehead_transverse_spacing


def measure_forehead_flow(points, path, step=.01):
    curve = np.asarray(points, float)[path]
    segments = np.linalg.norm(np.diff(curve, axis=0), axis=1)
    length = np.r_[0., np.cumsum(segments)]
    if segments.min() <= 1e-9 or length[-1] <= 2*step:
        raise ValueError('Collapsed or too-short forehead flow')
    centers = np.unique(np.r_[np.linspace(step, length[-1]-step, 20001),
                              np.clip(length, step, length[-1]-step),
                              np.clip(length-step, step, length[-1]-step),
                              np.clip(length+step, step, length[-1]-step)])
    samples = [np.column_stack([np.interp(centers+offset, length, curve[:, k])
                                for k in range(3)]) for offset in (-step, 0., step)]
    left, right = samples[1]-samples[0], samples[2]-samples[1]
    angles = np.degrees(np.arccos(np.clip(np.einsum('ij,ij->i', left, right)
                        /np.linalg.norm(left, axis=1)/np.linalg.norm(right, axis=1), -1, 1)))
    upper = np.minimum(curve[:-1, 2], curve[1:, 2]) >= 1.615
    paired = upper[:-1] & upper[1:]
    ratios = np.maximum(segments[:-1]/segments[1:], segments[1:]/segments[:-1])
    return {'fixed_scale_mm': step*1000, 'sampled_centers': len(centers),
            'sampled_peak_degrees': float(angles.max()),
            'upper_max_adjacent_length_ratio': float(ratios[paired].max()),
            'whole_path_max_min_length_ratio': float(segments.max()/segments.min()),
            'scope': 'Dense polyline chord-angle measurement, not an analytic curvature supremum'}


def validate_forehead_flow_result(old_open, proposal):
    points = np.asarray(proposal['vertices'], float)
    paths = proposal['guide_paths']['positive']
    caps = {'red': 18., 'orange': 20., 'green': 25., 'blue': 30., 'purple': 33.}
    metrics = {name: measure_forehead_flow(points, path) for name, path in paths.items()}
    for name, row in metrics.items():
        if row['sampled_peak_degrees'] > caps[name]:
            raise ValueError('Final fixed-scale flow exceeds current style bound: '+name)
        if name in ('green', 'blue', 'purple') and row['upper_max_adjacent_length_ratio'] > 1.8:
            raise ValueError('Final upper flow contains an abrupt density jump: '+name)
    current = np.asarray(proposal['open_delta'], np.float32)
    previous = np.asarray(old_open, np.float32)
    maximum = float(np.linalg.norm(current[:len(previous)]-previous, axis=1).max()*1000)
    if maximum > .50001:
        raise ValueError('Original-vertex cumulative MouthOpen transport exceeds 0.5 mm')
    return {'flows': metrics,
            'whole_guide_spacing': measure_forehead_station_spacing(points, paths),
            'full_curve_distances': measure_forehead_transverse_spacing(points, paths),
            'original_vertex_cumulative_open_delta_mm': maximum,
            'new_vertex_field_policy': 'Frozen author-surface donor interpolation',
            'style_thresholds_are_recipe_guards_not_user_acceptance': True}

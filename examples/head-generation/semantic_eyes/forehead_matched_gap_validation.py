"""Dense section and exact per-chart source-vertex extrema, with branch ownership."""
import numpy as np
from forehead_gap_measure import closest_forehead_polyline

def forehead_matched_gap_dense_errors(stage, points, params):
    result = stage.verify_fast.evaluate(points, params, details=True)
    if not result['valid']:
        return None
    errors = {key: 0.0 for key in stage.domains}
    for detail in result['details']:
        a, b = (detail['first'], detail['second'])
        pair = a + '-' + b
        domain = pair + ':' + detail['chart']
        target = stage.targets[pair]
        for side, (source, other) in enumerate(((a, b), (b, a))):
            samples = detail['world_hits'][side]
            locations = detail['curve_parameters'][side]
            first = int(np.ceil(float(locations.min()) - 1e-09))
            last = int(np.floor(float(locations.max()) + 1e-09))
            nodes = np.asarray(stage.paths[source])[np.arange(first, last + 1)]
            samples = np.vstack((samples, points[nodes]))
            gaps = closest_forehead_polyline(samples, points[stage.paths[other]])[0] * 1000
            errors[domain] = max(errors[domain], float(np.max(np.abs(gaps / target - 1))))
    return errors

"""Full-curve distances; angular turns and other shorter guides cannot hide tails."""
import numpy as np


def sample_forehead_polyline(curve, count):
    curve = np.asarray(curve, float)
    if curve.ndim != 2 or curve.shape[1] != 3 or len(curve) < 2:
        raise ValueError('Invalid guide coordinates')
    lengths = np.linalg.norm(np.diff(curve, axis=0), axis=1)
    if not np.isfinite(curve).all() or lengths.min() <= 1e-9 or count < 3:
        raise ValueError('Collapsed guide or insufficient samples')
    arc = np.r_[0., np.cumsum(lengths)]
    stations = np.linspace(0., arc[-1], count)
    samples = np.column_stack([np.interp(stations, arc, curve[:, k]) for k in range(3)])
    return samples, float(arc[-1])


def closest_forehead_polyline(samples, curve):
    """Return true Euclidean closest-segment distances and endpoint-domain flags.

    Beyond-endpoint flags are diagnostic only. No flagged sample is removed
    from the full-curve statistics. This is not geodesic distance or a proof of
    anatomical correspondence between curves with different terminal roles.
    """
    curve = np.asarray(curve, float)
    edges = np.diff(curve, axis=0)
    square = np.einsum('ij,ij->i', edges, edges)
    if np.any(square <= 1e-18):
        raise ValueError('Collapsed target guide')
    relative = samples[:, None] - curve[None, :-1]
    fraction = np.clip(np.einsum('ijk,jk->ij', relative, edges) / square, 0, 1)
    distances = np.linalg.norm(relative - fraction[:, :, None] * edges[None], axis=2)
    selected = distances.argmin(axis=1)
    rows = np.arange(len(samples))
    fractions = fraction[rows, selected]
    before = ((selected == 0) & (fractions <= 1e-12)
              & ((samples-curve[0]) @ edges[0] < -1e-12))
    after = ((selected == len(edges)-1) & (fractions >= 1-1e-12)
             & ((samples-curve[-1]) @ edges[-1] > 1e-12))
    return distances[rows, selected], before | after


def measure_forehead_transverse_spacing(points, paths, sample_count=257):
    """Measure every adjacent pair in both directions over the complete curves."""
    points = np.asarray(points, float)
    names = ('red', 'orange', 'green', 'blue', 'purple')
    curves = {name: points[np.asarray(paths[name], int)] for name in names}
    sampled = {name: sample_forehead_polyline(curve, sample_count)
               for name, curve in curves.items()}
    records, combined = [], []
    for first, second in zip(names, names[1:]):
        for source, target in ((first, second), (second, first)):
            samples, arc_length = sampled[source]
            gaps, beyond = closest_forehead_polyline(samples, curves[target])
            vertex_gaps, _ = closest_forehead_polyline(curves[source], curves[target])
            extrema = np.r_[gaps, vertex_gaps]
            if not np.isfinite(extrema).all() or extrema.min() <= 1e-9:
                raise ValueError('Coincident or invalid adjacent guide samples')
            combined.append(gaps)
            records.append({
                'source': source, 'target': target,
                'source_arc_length_mm': arc_length * 1000,
                'minimum_mm': float(extrema.min() * 1000),
                'maximum_mm': float(extrema.max() * 1000),
                'mean_mm': float(gaps.mean() * 1000),
                'max_min_ratio': float(extrema.max() / extrema.min()),
                'coefficient_of_variation': float(gaps.std() / gaps.mean()),
                'beyond_target_endpoint_samples': int(beyond.sum()),
                'beyond_endpoint_fraction': float(beyond.mean()),
                'all_samples_retained': True,
            })
    combined = np.concatenate(combined)
    minimum = min(row['minimum_mm'] for row in records)
    maximum = max(row['maximum_mm'] for row in records)
    return {
        'minimum_mm': minimum, 'maximum_mm': maximum,
        'max_min_ratio': maximum / minimum,
        'coefficient_of_variation': float(combined.std() / combined.mean()),
        'samples_per_direction': sample_count,
        'extra_extrema_checks': 'Every original guide vertex',
        'directions': records,
        'scope': 'Whole curves, both directions, physical arc-length samples; no angular truncation, '
                 'no excluded tails; Euclidean, not geodesic or anatomical-correspondence proof',
        'visual_acceptance': 'Not established by distance statistics',
    }

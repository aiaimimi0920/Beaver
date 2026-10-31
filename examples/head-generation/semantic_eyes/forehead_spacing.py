"""Whole-guide station evidence and cumulative frozen field references."""
import numpy as np


def forehead_station_residuals(points, paths):
    points = np.asarray(points, float)
    residuals = []
    for path in paths.values():
        lengths = np.linalg.norm(np.diff(points[np.asarray(path, int)], axis=0), axis=1)
        if len(lengths) < 2 or not np.isfinite(lengths).all() or lengths.min() <= 1e-9:
            raise ValueError('Invalid forehead guide station lengths')
        residuals.extend(lengths / lengths.mean() - 1.)
    return np.asarray(residuals)


def measure_forehead_station_spacing(points, paths):
    points = np.asarray(points, float)
    forehead_station_residuals(points, paths)
    result = {}
    for name, path in paths.items():
        lengths = np.linalg.norm(np.diff(points[np.asarray(path, int)], axis=0), axis=1)
        result[name] = {
            'segments': len(lengths),
            'minimum_mm': float(lengths.min() * 1000),
            'maximum_mm': float(lengths.max() * 1000),
            'mean_mm': float(lengths.mean() * 1000),
            'max_min_ratio': float(lengths.max() / lengths.min()),
            'coefficient_of_variation': float(lengths.std() / lengths.mean()),
            'maximum_relative_mean_deviation': float(abs(lengths / lengths.mean() - 1).max()),
            'scope': 'Every source edge of the whole guide, including fixed endpoints',
        }
    return result


def forehead_frozen_field(old_delta, point_count, initial_lineage):
    """Never reset original-vertex transport budgets after intermediate relocation."""
    old_delta = np.asarray(old_delta, float)
    if old_delta.ndim != 2 or old_delta.shape[1] != 3 or point_count < len(old_delta):
        raise ValueError('Invalid frozen forehead field dimensions')
    frozen = np.zeros((point_count, 3))
    frozen[:len(old_delta)] = old_delta
    for index, row in initial_lineage.items():
        if index >= len(old_delta):
            frozen[index] = np.asarray(row['weights']) @ old_delta[row['donors']]
    return frozen

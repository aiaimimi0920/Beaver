"""Actual Euclidean finite-rail gaps, separate from support cross-edge lengths."""
import numpy as np

def forehead_finite_rail_distance_matrix(first, second):
    a = np.asarray(first, float)
    c = np.asarray(second, float)
    if a.ndim != 2 or c.ndim != 2 or a.shape[1] != 3 or (c.shape[1] != 3) or (len(a) < 2) or (len(c) < 2) or (not np.isfinite(a).all()) or (not np.isfinite(c).all()):
        raise ValueError('Finite3D paths required')
    e = np.diff(a, axis=0)[:, None, :]
    f = np.diff(c, axis=0)[None, :, :]
    r = a[:-1, None, :] - c[None, :-1, :]
    aa = np.sum(e * e, 2)
    bb = np.sum(e * f, 2)
    cc = np.sum(f * f, 2)
    dd = np.sum(e * r, 2)
    ee = np.sum(f * r, 2)
    if np.min(aa) <= 0 or np.min(cc) <= 0:
        raise ValueError('Collapsed rail segment')
    pairs = [(0, np.clip(ee / cc, 0, 1)), (1, np.clip((ee + bb) / cc, 0, 1)), (np.clip(-dd / aa, 0, 1), 0), (np.clip((bb - dd) / aa, 0, 1), 1)]
    values = []
    for s, t in pairs:
        values.append(np.linalg.norm(r + np.asarray(s)[..., None] * e - np.asarray(t)[..., None] * f, axis=2))
    det = aa * cc - bb * bb
    safe = det > np.finfo(float).eps * aa * cc
    den = np.where(safe, det, 1.0)
    s = (bb * ee - cc * dd) / den
    t = (aa * ee - bb * dd) / den
    inside = safe & (s >= 0) & (s <= 1) & (t >= 0) & (t <= 1)
    interior = np.linalg.norm(r + s[..., None] * e - t[..., None] * f, axis=2)
    values.append(np.where(inside, interior, np.inf))
    return np.min(values, axis=0)

def forehead_finite_rail_profile(first, second):
    matrix = forehead_finite_rail_distance_matrix(first, second)
    distance = np.r_[matrix.min(axis=1), matrix.min(axis=0)]
    lengths = np.r_[np.linalg.norm(np.diff(first, axis=0), axis=1), np.linalg.norm(np.diff(second, axis=0), axis=1)]
    mean = float(np.average(distance, weights=lengths))
    if mean <= 0:
        raise ValueError('Coincident or intersecting rails')
    cv = float(np.sqrt(np.average((distance - mean) ** 2, weights=lengths)) / mean)
    return {'minimum_finite_gap_m': float(matrix.min()), 'symmetric_segment_minima_m': distance.tolist(), 'physical_length_weights_m': lengths.tolist(), 'length_weighted_gap_CV': cv, 'length_weighted_mean_gap_m': mean, 'metric': '3D finite segment Euclidean gap, not surface geodesic or projected width'}

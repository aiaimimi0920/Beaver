"""Explicit chart-triangle to immutable author-quad continuous-value transport.

No categorical interpolation, nearest-donor choice, or UV seam averaging.
"""
import numpy as np


def facial_quad_weights(canonical_triangle, barycentric):
    """Return four bilinear weights; triangle corners are canonical quad corners."""
    canonical = np.asarray(canonical_triangle, dtype=float)
    barycentric = np.asarray(barycentric, dtype=float)
    corners = {(0., 0.), (1., 0.), (1., 1.), (0., 1.)}
    if (canonical.shape != (3, 2) or not np.isfinite(canonical).all()
            or len(set(map(tuple, canonical))) != 3
            or not set(map(tuple, canonical)) <= corners):
        raise ValueError('Expected three distinct canonical quad corners')
    if (barycentric.shape != (3,) or not np.isfinite(barycentric).all()
            or np.any(barycentric < 0)
            or abs(float(barycentric.sum()) - 1.) > 1e-12):
        raise ValueError('Expected nonnegative normalized barycentric coordinates')
    # Normalization corrects only the checked representation-level sum error.
    s, t = (barycentric / barycentric.sum()) @ canonical
    return np.array([(1-s)*(1-t), s*(1-t), s*t, (1-s)*t])


def facial_quad_transport(quad_values, canonical_triangle, barycentric):
    """Transport explicitly continuous per-owner values, returning value and weights.

    quad_values has four rows in the source quad's oriented order. UV corner
    values must be supplied for the explicitly selected face/UV island. Integer
    labels are rejected rather than silently interpolated.
    """
    values = np.asarray(quad_values)
    if (values.ndim < 1 or len(values) != 4 or values.dtype.kind != 'f'
            or not np.isfinite(values).all()):
        raise ValueError('Expected four finite floating continuous-value rows')
    weights = facial_quad_weights(canonical_triangle, barycentric)
    return np.tensordot(weights, values, axes=(0, 0)), weights


def facial_transport_candidates(candidate_values, *, absolute_tolerance=0.):
    """Return an agreeing continuous value, otherwise preserve seam ambiguity.

    This is not an owner selector. The caller retains every candidate's owner
    and weights; tolerance is explicit in the value's units, never a geometry
    admission tolerance. Categorical data are not accepted.
    """
    tolerance = float(absolute_tolerance)
    if not np.isfinite(tolerance) or tolerance < 0:
        raise ValueError('Invalid comparison tolerance')
    values = [np.asarray(v) for v in candidate_values]
    if not values:
        raise ValueError('No source candidates')
    first = values[0]
    if any(v.dtype.kind != 'f' or v.shape != first.shape
           or not np.isfinite(v).all() for v in values):
        raise ValueError('Invalid continuous source candidate')
    if any(np.max(np.abs(v-first), initial=0.) > tolerance for v in values[1:]):
        raise ValueError('Ambiguous source seam; explicit owner selection required')
    return first.copy()

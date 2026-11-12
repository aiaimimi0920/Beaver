"""Smooth ordered-edge aspect rows, equivalent to max(length)/min(length)."""
import numpy as np


def forehead_aspect_geometry(points, quads):
    points, quads = np.asarray(points, float), np.asarray(quads, int)
    if (points.ndim != 2 or points.shape[1] != 3 or not np.isfinite(points).all()
            or quads.ndim != 2 or quads.shape[1] != 4 or not len(quads)
            or quads.min() < 0 or quads.max() >= len(points)
            or any(len(set(row)) != 4 for row in quads)):
        raise ValueError('Invalid forehead aspect geometry')
    corners = points[quads]
    edges = np.roll(corners, -1, axis=1)-corners
    squared = np.sum(edges*edges, axis=2)
    if not np.isfinite(squared).all() or np.any(squared <= 0):
        raise ValueError('Forehead aspect requires positive finite edge lengths')
    return edges, squared


def forehead_aspect_reference(points, quads):
    """Freeze once at the original fit start, never at a trial or warm restart."""
    return forehead_aspect_geometry(points, quads)[1].mean(axis=1)


def forehead_pairwise_aspect(points, quads, ratio=7., reference_squared_length=None,
                             gradient=False):
    """Twelve quad-major rows (R²*l_j²-l_i²)/frozen_scale, for i != j.

    With positive edges these are exactly equivalent to max(l)/min(l) <= R.
    No minimum/maximum branch is selected. Fixed positive scaling changes no
    feasible set. Final scalar/quantized max/min, orientation and all field and
    shape guards remain mandatory. Optional derivative has shape (Q,12,4,3),
    with respect to each quad's world-coordinate corners; caller accumulates
    shared vertices and handles author-donor boundary derivative fallback.
    """
    edges, squared = forehead_aspect_geometry(points, quads)
    if not np.isfinite(ratio) or ratio < 1:
        raise ValueError('Invalid forehead aspect bound')
    count = len(squared)
    reference = (np.ones(count) if reference_squared_length is None else
                 np.asarray(reference_squared_length, float))
    if (reference.shape != (count,) or not np.isfinite(reference).all()
            or np.any(reference <= 0)):
        raise ValueError('Invalid frozen forehead aspect scaling')
    pairs = np.asarray([(i, j) for i in range(4) for j in range(4) if i != j])
    i, j = pairs.T
    values = ((ratio*ratio*squared[:, j]-squared[:, i])/reference[:, None]).ravel()
    if not gradient:
        return values
    derivative = np.zeros((count, 12, 4, 3))
    for row, (a, b) in enumerate(pairs):
        ga = -2*edges[:, a]/reference[:, None]
        gb = 2*ratio*ratio*edges[:, b]/reference[:, None]
        derivative[:, row, a] -= ga
        derivative[:, row, (a+1) % 4] += ga
        derivative[:, row, b] -= gb
        derivative[:, row, (b+1) % 4] += gb
    return values, derivative, pairs

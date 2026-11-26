"""Private prototype: smooth local children of the existing role extrema."""
import numpy as np


def forehead_extrema_edges(points):
    p = np.asarray(points, float)
    if p.ndim != 2 or p.shape[1] != 3 or len(p) < 3 or not np.isfinite(p).all():
        raise ValueError('Invalid role geometry')
    edges = np.diff(p, axis=0)
    lengths = np.linalg.norm(edges, axis=1)
    if np.any(lengths <= 0):
        raise ValueError('Collapsed role edge')
    gradients = np.stack((-edges / lengths[:, None], edges / lengths[:, None]), axis=1)
    return lengths * 1000, gradients * 1000


def forehead_extrema_turns(points, frame=None):
    """3D unsigned angles or projected SIGNED angles and local world gradients.

    Signed projected angles are later represented by both upper and lower
    inequalities, retaining zero-turn differentiability without selecting a sign.
    Exact/near 3D singularities and projected branch cuts remain unsupported.
    """
    p = np.asarray(points, float)
    forehead_extrema_edges(p)
    if frame is not None:
        frame = np.asarray(frame, float)
        if (frame.shape != (2, 3) or not np.isfinite(frame).all()
                or not np.isclose(frame[0] @ frame[0], 1)
                or not np.isclose(frame[1] @ frame[1], 1)
                or not np.isclose(frame[0] @ frame[1], 0)):
            raise ValueError('Invalid role camera frame')
    q = p if frame is None else p @ frame.T
    edges = np.diff(q, axis=0)
    lengths = np.linalg.norm(edges, axis=1)
    if np.any(lengths <= 0):
        raise ValueError('Collapsed projected role edge')
    unit = edges / lengths[:, None]
    dots = np.sum(unit[:-1] * unit[1:], axis=1)
    if frame is None:
        angles = np.arccos(np.clip(dots, -1, 1))
        denominator = np.sqrt(np.maximum(0, 1 - dots * dots))
        invalid = denominator <= 1e-10
        safe = np.where(invalid, 1, denominator)
        ga = -(unit[1:] - dots[:, None] * unit[:-1]) / (lengths[:-1] * safe)[:, None]
        gb = -(unit[:-1] - dots[:, None] * unit[1:]) / (lengths[1:] * safe)[:, None]
    else:
        cross = unit[:-1, 0] * unit[1:, 1] - unit[:-1, 1] * unit[1:, 0]
        angles = np.arctan2(cross, dots)
        invalid = np.pi - abs(angles) <= 1e-8
        ga = np.c_[edges[:-1, 1], -edges[:-1, 0]] / lengths[:-1, None] ** 2
        gb = np.c_[-edges[1:, 1], edges[1:, 0]] / lengths[1:, None] ** 2
        ga, gb = ga @ frame, gb @ frame
    gradient = np.stack((-ga, ga - gb, gb), axis=1) * (180 / np.pi)
    gradient[invalid] = 0
    return np.degrees(angles), gradient, invalid

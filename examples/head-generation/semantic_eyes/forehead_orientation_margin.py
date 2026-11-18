"""Fixed-source chart area targets for bounded restoration, not final gates."""
import numpy as np


def forehead_orientation_integer_array(values, name, dimensions):
    a = np.asarray(values)
    if a.ndim != dimensions or a.dtype.kind not in 'iu' or a.dtype.kind == 'b':
        raise ValueError(name + ' must be an integer array')
    return a.astype(np.int64, copy=False)


def forehead_signed_chart_cross_mm2(points, quads, signs, chart_radius):
    """Use the SAME immutable source radius on smooth and float32 points."""
    p = np.asarray(points, float)
    q = forehead_orientation_integer_array(quads, 'quads', 2)
    s = np.asarray(signs, float)
    radius = np.asarray(chart_radius)
    if (p.ndim != 2 or p.shape[1] != 3 or q.shape[1] != 4 or
            s.shape != q.shape or radius.ndim != 0 or
            not np.isfinite(radius) or radius <= 0 or
            not np.isfinite(p).all() or not np.isfinite(s).all() or
            np.any(np.abs(s) != 1) or np.any(q < 0) or np.any(q >= len(p))):
        raise ValueError('Invalid fixed-radius chart inputs')
    if any(len(set(row)) != 4 for row in q):
        raise ValueError('Quad corners must be distinct')
    chart = np.column_stack((float(radius)*np.arctan2(p[:, 0], -p[:, 1]), p[:, 2]))
    edge = np.roll(chart[q], -1, axis=1)-chart[q]
    cross = edge[:, :, 0]*np.roll(edge[:, :, 1], -1, axis=1)
    cross -= edge[:, :, 1]*np.roll(edge[:, :, 0], -1, axis=1)
    return cross*s*1e6


def forehead_orientation_margin_targets(targets, rows, raw_per_scaled,
        smooth_mm2, rounded_mm2, variable_rows, safety_mm2=1e-4,
        accepted_mm2=None, interior_cap_mm2=None):
    """Append stronger working targets; scales convert raw mm² to solver rows.

    accepted_mm2 is captured ONCE at the original accepted stage start. Optional
    floor min(accepted/2, cap) never replaces final orientation/mesh checks.
    """
    out = np.asarray(targets, float).copy()
    r = forehead_orientation_integer_array(rows, 'rows', 1)
    scales = np.asarray(raw_per_scaled, float)
    smooth = np.asarray(smooth_mm2, float)
    rounded = np.asarray(rounded_mm2, float)
    variable = np.asarray(variable_rows)
    safety = np.asarray(safety_mm2)
    if (out.ndim != 1 or scales.shape != out.shape or
            smooth.ndim != 1 or rounded.shape != smooth.shape or
            variable.shape != smooth.shape or variable.dtype.kind != 'b' or
            len(r) != len(smooth) or len(set(r)) != len(r) or
            np.any(r < 0) or np.any(r >= len(out)) or
            safety.ndim != 0 or not np.isfinite(safety) or safety <= 0 or
            not all(np.isfinite(a).all() for a in (out, scales, smooth, rounded)) or
            np.any(out < 0) or np.any(scales <= 0)):
        raise ValueError('Invalid orientation targets, rows, scales or shapes')
    bad = rounded <= 0
    if np.any(bad & ~variable):
        raise ValueError('Immutable rounded orientation fails')
    if (accepted_mm2 is None) != (interior_cap_mm2 is None):
        raise ValueError('Accepted areas and interior cap must be supplied together')
    floor = np.zeros(len(r))
    if accepted_mm2 is not None:
        accepted = np.asarray(accepted_mm2, float)
        cap = np.asarray(interior_cap_mm2)
        if (accepted.shape != smooth.shape or not np.isfinite(accepted).all() or
                np.any(accepted <= 0) or cap.ndim != 0 or not np.isfinite(cap) or cap <= 0):
            raise ValueError('Original accepted area and cap must be positive finite')
        floor[variable] = np.minimum(.5*accepted[variable], float(cap))
    required = np.where(bad, np.maximum(smooth-rounded, 0)+float(safety), 0)
    required = np.maximum(required, floor)
    out[r] = np.maximum(out[r], required/scales[r])
    evidence = [{'row': int(r[k]), 'smooth_mm2': float(smooth[k]),
        'rounded_mm2': float(rounded[k]), 'required_raw_mm2': float(required[k]),
        'rounding_failed': bool(bad[k]), 'accepted_interior_floor_mm2': float(floor[k]),
        'safety_mm2': float(safety)} for k in np.flatnonzero(required > 0)]
    return out, evidence

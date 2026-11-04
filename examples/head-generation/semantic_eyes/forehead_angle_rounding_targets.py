"""Float32 working clearance for failed or explicitly movable corners; no hard-gate changes."""
import numpy as np
from orbital_support_rule import orbital_quad_quality
from forehead_angle_rounding_bound import (ForeheadAngleFraction,
    forehead_angle_fraction_up, forehead_float32_angle_bound)


def forehead_float32_angle_targets(points, quads, scales, lower_rows, upper_rows,
        smooth_angles, saved_angles, lower_hard=25., upper_hard=150.,
        lower_fit=25.003, upper_fit=149.997, safety_degrees=.001, variable_ids=None):
    """Additional scaled targets; explicit variable IDs enable both-sided preflight.

    Same original evaluator, original corner ownership and fixed scales. Caller
    takes max with all existing policies and retains targets across iterations.
    Future candidates must be re-evaluated; this is not a replacement admission.
    IDs describe original potential movement, not a nonzero local derivative or
    freedom under temporary held columns. Do not weaken retained targets if a
    held leg becomes incompatible. With no IDs, preserve the failure-only API.
    """
    p = np.asarray(points, float)
    faces = np.asarray(quads)
    scale = np.asarray(scales, float)
    low, high = np.asarray(lower_rows), np.asarray(upper_rows)
    smooth, saved = np.asarray(smooth_angles, float), np.asarray(saved_angles, float)
    limits = np.asarray([lower_hard, upper_hard, lower_fit, upper_fit, safety_degrees], float)
    if (p.ndim != 2 or p.shape[1:] != (3,) or not np.isfinite(p).all()
            or faces.ndim != 2 or faces.shape[1:] != (4,) or faces.dtype.kind not in 'iu'
            or np.any(faces < 0) or np.any(faces >= len(p))
            or scale.ndim != 1 or not np.isfinite(scale).all() or np.any(scale <= 0)
            or low.ndim != 1 or high.ndim != 1 or low.dtype.kind not in 'iu' or high.dtype.kind not in 'iu'
            or len(low) != 4*len(faces) or high.shape != low.shape
            or smooth.shape != (len(faces), 4) or saved.shape != smooth.shape
            or not np.isfinite(smooth).all() or not np.isfinite(saved).all()
            or np.any(low < 0) or np.any(high < 0) or np.any(low >= len(scale)) or np.any(high >= len(scale))
            or len(set(np.r_[low, high].tolist())) != len(low)+len(high)
            or not np.isfinite(limits).all() or not 0 <= lower_hard <= lower_fit < upper_fit <= upper_hard <= 180
            or safety_degrees <= 0):
        raise ValueError('Invalid float32 angle target ownership or policy')
    failing = np.flatnonzero((saved.ravel() < lower_hard) | (saved.ravel() > upper_hard))
    movable = np.zeros(smooth.size, dtype=bool)
    if variable_ids is not None:
        ids = np.asarray(variable_ids)
        if (ids.ndim != 1 or ids.dtype.kind not in 'iu'
                or len(set(ids.tolist())) != len(ids)
                or np.any(ids < 0) or np.any(ids >= len(p))):
            raise ValueError('Invalid original variable IDs for angle preflight')
        membership = np.isin(faces, ids)
        for corner in range(4):
            movable[corner::4] = membership[:, [(corner-1) % 4, corner, (corner+1) % 4]].any(axis=1)
    selected = np.union1d(failing, np.flatnonzero(movable))
    targets = np.zeros(len(scale))
    if not len(selected):
        return targets, []
    # Fail closed if caller's reused measurements belong to another candidate.
    actual_smooth, _ = orbital_quad_quality(p, faces)
    actual_saved, _ = orbital_quad_quality(p.astype(np.float32).astype(float), faces)
    if not np.array_equal(actual_smooth, smooth) or not np.array_equal(actual_saved, saved):
        raise ValueError('Float32 angle targets do not match original candidate measurements')
    evidence = []
    for index in selected:
        face, corner = divmod(int(index), 4)
        vertices = faces[face, [(corner-1) % 4, corner, (corner+1) % 4]]
        bound, detail = forehead_float32_angle_bound(p[vertices], smooth.ravel()[index], saved.ravel()[index])
        sides = (False, True) if movable[index] else (bool(saved.ravel()[index] > upper_hard),)
        for upper in sides:
            row = int(high[index] if upper else low[index])
            hard = ForeheadAngleFraction.from_float(float(upper_hard if upper else lower_hard))
            fitted = ForeheadAngleFraction.from_float(float(upper_fit if upper else lower_fit))
            present = hard-fitted if upper else fitted-hard
            required = max(ForeheadAngleFraction(0), ForeheadAngleFraction.from_float(bound)+
                ForeheadAngleFraction.from_float(float(safety_degrees))-present)
            targets[row] = forehead_angle_fraction_up(required/ForeheadAngleFraction.from_float(float(scale[row])))
            if required or index in failing:
                evidence.append({'row': row, 'bound_degrees': bound,
                    'additional_raw_target_degrees': forehead_angle_fraction_up(required),
                    **detail, **({'proactive_original_variable_corner': True} if movable[index] else {})})
    return targets, evidence

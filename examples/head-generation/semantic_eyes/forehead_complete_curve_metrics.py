"""Role-independent physical side-curve diagnostics, not surface authority."""
import numpy as np

def forehead_complete_side_curve(points, camera_right=(0.0, 1.0, 0.0), camera_up=(0.0, 0.0, 1.0)):
    p = np.asarray(points, float)
    r = np.asarray(camera_right, float)
    u = np.asarray(camera_up, float)
    if p.ndim != 2 or p.shape[1] != 3 or len(p) < 4 or (not np.isfinite(p).all()):
        raise ValueError('Need finite ordered 3D role path')
    if not np.isclose(r @ r, 1) or not np.isclose(u @ u, 1) or (not np.isclose(r @ u, 0)):
        raise ValueError('Camera axes must be orthonormal')
    q = p @ np.stack([r, u]).T
    e = np.diff(q, axis=0)
    l = np.linalg.norm(e, axis=1)
    physical = np.linalg.norm(np.diff(p, axis=0), axis=1)
    if np.min(l) <= 0 or np.min(physical) <= 0:
        raise ValueError('Collapsed projected or physical station')
    tangent = e / l[:, None]
    turn = np.arctan2(tangent[:-1, 0] * tangent[1:, 1] - tangent[:-1, 1] * tangent[1:, 0], np.sum(tangent[:-1] * tangent[1:], axis=1))
    dual = (l[:-1] + l[1:]) / 2
    curvature = turn / dual
    between = (dual[:-1] + dual[1:]) / 2
    fairness = float(np.sum(np.diff(curvature) ** 2 / between) * l.sum() ** 3)
    return {'signed_turn_degrees': np.degrees(turn).tolist(), 'signed_curvature_per_m': curvature.tolist(), 'curvature_variation_energy': fairness, 'turn_sign_reversals': int(np.sum(turn[:-1] * turn[1:] < 0)), 'physical_station_lengths_m': physical.tolist(), 'physical_spacing_CV': float(np.std(physical) / np.mean(physical)), 'projected_lengths_m': l.tolist(), 'camera_right': r.tolist(), 'camera_up': u.tolist(), 'geometry_modified': False, 'candidate_admitted': False}

"""Bounded current-surface fallback; never skips or changes original nasal gate."""
import numpy as np
from scipy.optimize import least_squares

def adapt_nasal_patch_parameters(surface, boundary_points, new_points, quads, lineage, validate_fn):
    if not np.isfinite(np.asarray(boundary_points, float)).all() or not np.isfinite(np.asarray(new_points, float)).all():
        raise ValueError('Nonfinite nasal parameter input')
    try:
        quality = validate_fn(surface, boundary_points, new_points, quads)
        return (new_points, lineage, {'status': 'unchanged_original_gate_passed', 'quality': quality})
    except ValueError as error:
        if str(error) != 'Nasal source trial angle or author-surface budget exceeded':
            raise
        original_failure = str(error)
    boundary = np.asarray(boundary_points, float)
    original = np.asarray(new_points, float)
    base = np.r_[boundary, original]
    n = len(boundary)
    q = np.asarray(quads, int)
    if q.ndim != 2 or q.shape[1] != 4 or len(lineage) != len(original):
        raise ValueError('Unsupported nasal parameter fallback topology')
    bary = np.array([(1 / 3, 1 / 3, 1 / 3), (0.6, 0.2, 0.2), (0.2, 0.6, 0.2), (0.2, 0.2, 0.6), (0.45, 0.45, 0.1), (0.45, 0.1, 0.45), (0.1, 0.45, 0.45)])
    triangles = np.array([[0, 1, 2], [0, 2, 3], [1, 2, 3], [1, 3, 0]])

    def measurements(points, face_ids):
        a = points[q[face_ids]]
        e = np.roll(a, 1, axis=1) - a
        f = np.roll(a, -1, axis=1) - a
        norm = np.linalg.norm(e, axis=2) * np.linalg.norm(f, axis=2)
        if np.any(norm < 1e-12):
            raise ValueError('Fallback encountered collapsed edge')
        angles = np.degrees(np.arccos(np.clip(np.sum(e * f, 2) / norm, -1, 1)))
        samples = np.einsum('wj,tijk->tiwk', bary, a[:, triangles]).reshape(-1, 3)
        errors = np.array([s[1] - surface.sample(s[0], s[2])[1] for s in samples]).reshape(len(face_ids), 28)
        return (angles, errors)
    angles, errors = measurements(base.astype(np.float32).astype(float), np.arange(len(q)))
    bad = np.flatnonzero((angles.min(1) < 10) | (angles.max(1) > 170) | (np.abs(errors).max(1) > 0.00100001))
    if not 1 <= len(bad) <= 4:
        raise ValueError('Nasal fallback exceeds bounded failing-face profile')
    ids = sorted({int(i) for fi in bad for i in q[fi] if i >= n})
    if not 1 <= len(ids) <= 12:
        raise ValueError('Nasal fallback exceeds bounded variable profile')
    affected = np.flatnonzero(np.isin(q, ids).any(1))

    def positions(z):
        points = base.copy()
        xy = base[ids][:, [0, 2]] + 0.001 * z.reshape(-1, 2)
        for i, x in zip(ids, xy):
            points[i] = surface.sample(*x)
        return points
    evaluations = 0

    def objective(z):
        nonlocal evaluations
        a, e = measurements(positions(z), affected)
        evaluations += 1
        return np.r_[z * 0.02, np.maximum(0, a.ravel() - 169.0) * 0.1, np.maximum(0, 10.0 - a.ravel()) * 0.1, np.maximum(0, np.abs(e.ravel()) - 0.00098) * 1000]
    result = least_squares(objective, np.zeros(len(ids) * 2), bounds=(-0.8, 0.8), diff_step=0.001, max_nfev=25, ftol=1e-07, xtol=1e-07, gtol=1e-07)
    proposed = positions(result.x).astype(np.float32).astype(float)
    displacement = np.linalg.norm(proposed[ids] - base[ids], axis=1)
    if max(displacement) > 0.00100001:
        raise ValueError('Nasal fallback exceeds1mm point displacement budget')
    updated = original.copy()
    updated_lineage = list(lineage)
    changed = []
    for i, d in zip(ids, displacement):
        if d <= 1e-10:
            continue
        donors, weights, _ = surface.lineage(*proposed[i, [0, 2]])
        updated[i - n] = proposed[i]
        updated_lineage[i - n] = (donors, weights)
        changed.append(i - n)
    quality = validate_fn(surface, boundary_points, updated, quads)
    return (updated, updated_lineage, dict(status='bounded_current_surface_parameter_adaptation', original_failure=original_failure, failing_patch_faces=bad.tolist(), affected_patch_faces=affected.tolist(), variable_patch_slots=ids, moved_new_point_slots=changed, max_displacement_m=float(max(displacement)), optimizer_converged=bool(result.success), optimizer_evaluations=evaluations, quality=quality, warning='Old gate repaired only; later source stages, actual export, fields, poses, visual and other profiles still require validation.'))

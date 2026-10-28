"""Check actual position-generated mouth targets using the current export selector."""
import itertools
import numpy as np
from nasal_spacing_shape import guard_nasal_spacing_shape


def guard_mouth_corner_shapes(old, faces, new, affected, old_open, new_open, selector, motion):
    old, new = np.asarray(old, dtype=np.float32), np.asarray(new, dtype=np.float32)
    used = sorted({i for fi in affected for i in faces[fi]})
    old_deltas = [(old + np.asarray(old_open, dtype=np.float32)) - old]
    new_deltas = [(new + np.asarray(new_open, dtype=np.float32)) - new]
    names = ['MouthRound', 'MouthSpread', 'MouthPucker']
    proofs = []
    for name in names:
        pair = []
        for points in (old, new):
            delta = np.zeros_like(points)
            for i in used:
                raw = np.asarray(motion(points[i].astype(float), name), dtype=np.float32)
                if raw.shape != (3,) or not np.isfinite(raw).all():
                    raise ValueError('Invalid generated mouth target')
                delta[i] = (points[i] + raw) - points[i]
            pair.append(delta)
        old_deltas.append(pair[0])
        new_deltas.append(pair[1])
        proofs.append(guard_nasal_spacing_shape(old, faces, new, faces, affected,
            pair[0], pair[1], selector, selector_delta=new_open, pose_name=name))
    proofs.append(guard_nasal_spacing_shape(old, faces, new, faces, affected,
        np.sum(old_deltas, axis=0), np.sum(new_deltas, axis=0), selector,
        selector_delta=new_open, pose_name='MouthOpen+Round+Spread+Pucker'))
    triangles = []
    for fi in sorted(affected):
        q = faces[fi]
        choices = selector(new.astype(float), q, (new + new_open).astype(float))
        if not choices:
            raise ValueError('Missing current mouth export diagonal')
        diagonal = max(choices, key=choices.get)
        local = ((0, 1, 2), (0, 2, 3)) if diagonal == 0 else ((0, 1, 3), (1, 2, 3))
        triangles.extend(tuple(q[i] for i in tri) for tri in local)
    triangles = np.asarray(triangles)
    def normals(points):
        p = points[triangles]
        return np.cross(p[:, 1] - p[:, 0], p[:, 2] - p[:, 0])
    neutral = normals(new.astype(float))
    minimum = 1.
    for weights in itertools.product((0., .5, 1.), repeat=4):
        points = new.astype(float).copy()
        for weight, delta in zip(weights, new_deltas):
            points += weight * delta
        current = normals(points)
        length = np.linalg.norm(current, axis=1) * np.linalg.norm(neutral, axis=1)
        if np.min(length) <= 1e-20:
            raise ValueError('Generated mouth combination collapsed')
        cosine = np.einsum('ij,ij->i', neutral, current) / length
        if np.min(cosine) <= 0:
            raise ValueError('Generated mouth combination reversed')
        minimum = min(minimum, float(np.min(cosine)))
    return {'sampled_targets': proofs, 'orientation_states': 81, 'minimum_cosine': minimum,
            'scope': 'Current-position generated mouth targets and current paired export; '
                     'sampled local checks, not final native export or continuous collision proof.'}

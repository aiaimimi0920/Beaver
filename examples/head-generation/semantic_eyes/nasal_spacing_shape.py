"""Dense sampled shape guard for the actual paired source export selector."""
import numpy as np
from chin_source_shape import ChinFrozenDistance, chin_shape_exact_distance


def guard_nasal_spacing_shape(old, old_faces, new, new_faces, affected,
                              old_delta, new_delta, selector, subdivisions=32):
    if int(subdivisions) != subdivisions or subdivisions < 32:
        raise ValueError('Nasal spacing shape verification requires at least 32 subdivisions')
    count = int(subdivisions)
    old = np.asarray(old, dtype=np.float32)
    new = np.asarray(new, dtype=np.float32)
    old_delta = np.asarray(old_delta, dtype=np.float32)
    new_delta = np.asarray(new_delta, dtype=np.float32)
    if not (old.shape == new.shape == old_delta.shape == new_delta.shape):
        raise ValueError('Nasal spacing shape arrays disagree')
    if not all(np.isfinite(values).all() for values in (old, new, old_delta, new_delta)):
        raise ValueError('Nonfinite nasal spacing shape field')
    old_poses = [old.astype(float), (old + old_delta).astype(float)]
    new_poses = [new.astype(float), (new + new_delta).astype(float)]
    indices = sorted(affected)
    source_quads = [old_faces[i] for i in indices]
    triangles, selected = [], []
    for index in indices:
        face = new_faces[index]
        options = selector(new_poses[0], face, new_poses[1])
        if not options:
            raise ValueError('No paired export diagonal for nasal spacing shape verification')
        diagonal = max(options, key=lambda value: options[value])
        selected.append(diagonal)
        local = ((0, 1, 2), (0, 2, 3)) if diagonal == 0 else ((0, 1, 3), (1, 2, 3))
        triangles.extend(tuple(face[k] for k in tri) for tri in local)
    triangles = np.asarray(triangles, dtype=int)
    barycentric = np.asarray([[i / count, j / count, 1 - (i + j) / count]
                             for i in range(count + 1) for j in range(count + 1 - i)])
    cases = []
    for pose, (source, candidate) in enumerate(zip(old_poses, new_poses)):
        field = ChinFrozenDistance(source, source_quads, count)
        samples = np.einsum('ij,kjl->kil', barycentric, candidate[triangles]).reshape(-1, 3)
        forward = field.distance(samples)
        reverse = chin_shape_exact_distance(field.points, candidate, triangles)
        for direction, distances in (('forward', forward), ('reverse', reverse)):
            maximum = float(np.max(distances))
            if not np.isfinite(maximum) or maximum > .001:
                raise ValueError('Nasal spacing exceeds the 1 mm sampled source-shape bound')
            cases.append({'pose': 'neutral' if pose == 0 else 'MouthOpen',
                          'direction': direction, 'max_mm': maximum * 1000,
                          'samples': len(distances)})
    return {'subdivisions': count, 'limit_mm': 1., 'cases': cases,
            'paired_selected_diagonals': selected,
            'scope': 'Sampled bidirectional current author surface versus paired export; '
                     'not a continuous bound or actual saved-export certificate'}

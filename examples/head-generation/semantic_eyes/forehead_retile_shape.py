"""Paired-export and generated-pose checks for different-size source retiles."""
import itertools
import numpy as np
from chin_source_shape import ChinFrozenDistance, chin_shape_exact_distance


def guard_forehead_retile_poses(old, old_faces, new, new_faces, old_ids, new_ids,
                               old_open, new_open, selector, blink, mouth, subdivisions=32):
    if int(subdivisions) != subdivisions or subdivisions < 32:
        raise ValueError('Forehead retile requires at least 32 surface subdivisions')
    old, new = np.asarray(old, np.float32), np.asarray(new, np.float32)
    old_open, new_open = np.asarray(old_open, np.float32), np.asarray(new_open, np.float32)
    if old.shape != old_open.shape or new.shape != new_open.shape:
        raise ValueError('Forehead retile pose fields have different vertex ownership')
    if not all(np.isfinite(x).all() for x in (old, new, old_open, new_open)):
        raise ValueError('Nonfinite forehead pose source')
    source_quads = [old_faces[i] for i in old_ids]
    triangles, diagonals = [], []
    for i in new_ids:
        q = new_faces[i]
        options = selector(new.astype(float), q, (new+new_open).astype(float))
        if len(q) != 4 or not options:
            raise ValueError('Forehead retile has no valid automatic paired-export diagonal')
        diagonal = max(options, key=options.get)
        diagonals.append(diagonal)
        local = ((0, 1, 2), (0, 2, 3)) if diagonal == 0 else ((0, 1, 3), (1, 2, 3))
        triangles.extend(tuple(q[j] for j in tri) for tri in local)
    triangles = np.asarray(triangles, int)
    old_used = sorted({i for q in source_quads for i in q})
    new_used = sorted({i for j in new_ids for i in new_faces[j]})
    old_fields, new_fields = {'MouthOpen': old_open}, {'MouthOpen': new_open}
    for name in ('MouthRound', 'MouthSpread', 'MouthPucker', 'blink.L', 'blink.R'):
        for points, used, fields in ((old, old_used, old_fields), (new, new_used, new_fields)):
            delta = np.zeros_like(points)
            for i in used:
                p = points[i].astype(float)
                if name.startswith('blink.'):
                    enabled = p[0] < 0 if name == 'blink.L' else p[0] > 0
                    raw = np.array([0., 0., blink(p) if enabled else 0.])
                else:
                    raw = np.asarray(mouth(p, name), np.float32)
                if raw.shape != (3,) or not np.isfinite(raw).all():
                    raise ValueError('Invalid current-position forehead expression field')
                delta[i] = (points[i]+raw.astype(np.float32))-points[i]
            fields[name] = delta
    names = list(new_fields)
    cases = [('neutral', np.zeros_like(old), np.zeros_like(new))]
    cases.extend((name, old_fields[name], new_fields[name]) for name in names)
    for label, included in (
            ('MouthOpen+Round+Spread+Pucker', names[:4]),
            ('MouthOpen+BlinkBoth', ['MouthOpen', 'blink.L', 'blink.R']),
            ('BlinkBoth', ['blink.L', 'blink.R']),
            ('all_six_full', names)):
        cases.append((label, sum(old_fields[n] for n in included), sum(new_fields[n] for n in included)))
    count = int(subdivisions)
    barycentric = np.asarray([[i/count, j/count, 1-(i+j)/count]
                             for i in range(count+1) for j in range(count+1-i)])
    proofs = []
    for name, source_delta, target_delta in cases:
        source = (old+source_delta).astype(float)
        target = (new+target_delta).astype(float)
        field = ChinFrozenDistance(source, source_quads, count)
        samples = np.einsum('ij,kjl->kil', barycentric, target[triangles]).reshape(-1, 3)
        forward = field.distance(samples)
        reverse = chin_shape_exact_distance(field.points, target, triangles)
        row = {'pose': name, 'forward_mm': float(forward.max()*1000),
               'reverse_mm': float(reverse.max()*1000),
               'forward_samples': len(forward), 'reverse_samples': len(reverse)}
        print('Forehead source pose:', row, flush=True)
        if not np.isfinite([row['forward_mm'], row['reverse_mm']]).all() or max(
                row['forward_mm'], row['reverse_mm']) > 1:
            raise ValueError('Forehead retile exceeds the 1 mm sampled source-shape bound: '+name)
        proofs.append(row)

    def normals(points):
        q = points[triangles]
        return np.cross(q[:, 1]-q[:, 0], q[:, 2]-q[:, 0])

    neutral = normals(new.astype(float))
    minimum = 1.
    for values in itertools.product((0., .5, 1.), repeat=len(names)):
        points = new.astype(float).copy()
        for amount, name in zip(values, names):
            points += amount*new_fields[name]
        normal = normals(points)
        norm = np.linalg.norm(neutral, axis=1)*np.linalg.norm(normal, axis=1)
        if norm.min() <= 1e-20:
            raise ValueError('Forehead expression combination collapsed a triangle')
        cosine = np.einsum('ij,ij->i', neutral, normal)/norm
        if cosine.min() <= 0:
            raise ValueError('Forehead expression combination reversed a triangle')
        minimum = min(minimum, float(cosine.min()))
    return {'subdivisions': count, 'limit_mm': 1., 'cases': proofs,
            'paired_selected_diagonals': diagonals,
            'orientation_states': 3**len(names), 'minimum_orientation_cosine': minimum,
            'old_patch_quads': len(old_ids), 'new_patch_quads': len(new_ids),
            'scope': 'Sampled bidirectional original author surface versus current automatic export; '
                     'not a continuous collision bound or an actual saved-export certificate'}

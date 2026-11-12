"""Verify correspondence through an intentional source-shape adjustment."""
import itertools
import numpy as np


def guard_forehead_author_poses(old, new, faces, changed, open_delta, blink, mouth):
    old, new = np.asarray(old, np.float32), np.asarray(new, np.float32)
    changed = set(changed)
    patch = [q for q in faces if changed.intersection(q)]
    if not patch or any(len(q) != 4 for q in patch):
        raise ValueError('Author regularization must affect only quad skin')
    used = sorted({i for q in patch for i in q})
    triangles = np.asarray([t for q in patch for t in
                           ([q[0], q[1], q[2]], [q[0], q[2], q[3]],
                            [q[0], q[1], q[3]], [q[1], q[2], q[3]])])
    fields = []
    for points in (old, new):
        values = [np.asarray(open_delta, float)]
        for name in ('MouthRound', 'MouthSpread', 'MouthPucker', 'blink.L', 'blink.R'):
            delta = np.zeros_like(points)
            for i in used:
                p = points[i].astype(float)
                if name.startswith('blink.'):
                    enabled = p[0] < 0 if name == 'blink.L' else p[0] > 0
                    raw = [0., 0., blink(p) if enabled else 0.]
                else:
                    raw = mouth(p, name)
                raw = np.asarray(raw, np.float32)
                if raw.shape != (3,) or not np.isfinite(raw).all():
                    raise ValueError('Invalid regenerated author-pose field')
                delta[i] = (points[i]+raw)-points[i]
            values.append(delta.astype(float))
        fields.append(values)
    def normals(points):
        q = points[triangles]
        return np.cross(q[:, 1]-q[:, 0], q[:, 2]-q[:, 0])
    minimum, maximum = 1., 0.
    for weights in itertools.product((0., .5, 1.), repeat=6):
        posed = [points.astype(float)+sum(w*f for w, f in zip(weights, values))
                 for points, values in zip((old, new), fields)]
        a, b = map(normals, posed)
        norm = np.linalg.norm(a, axis=1)*np.linalg.norm(b, axis=1)
        if norm.min() <= 1e-20:
            raise ValueError('Author shape collapses a posed triangle')
        cosine = np.einsum('ij,ij->i', a, b)/norm
        if cosine.min() <= 0:
            raise ValueError('Author shape reverses a posed triangle')
        minimum = min(minimum, float(cosine.min()))
        maximum = max(maximum, float(np.linalg.norm(posed[1][used]-posed[0][used], axis=1).max()*1000))
    if maximum > 2.001:
        raise ValueError('Author shape exceeds the 2 mm correspondence cap in a generated pose')
    return {'states': 729, 'maximum_corresponding_vertex_displacement_mm': maximum,
            'minimum_corresponding_normal_cosine': minimum, 'both_diagonals_checked': True,
            'scope': 'Same-connectivity bounded author-shape change; not a collision proof '
                     'and not the separate 1 mm retile preservation check'}

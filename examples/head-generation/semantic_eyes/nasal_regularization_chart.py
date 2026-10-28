"""CPU-only source-relative connected-face donor walking; no nearest-depth tie-break."""
from collections import defaultdict
import numpy as np

class ConnectedQuadChart:

    def __init__(self, vertices, quads, regions):
        self.v = np.asarray(vertices, float)
        self.q = {i: tuple(map(int, q)) for i, (q, r) in enumerate(zip(quads, regions)) if r == 1 and len(q) == 4}
        self.at = defaultdict(set)
        edges = defaultdict(list)
        for i, q in self.q.items():
            for x in q:
                self.at[x].add(i)
            for a, b in zip(q, q[1:] + q[:1]):
                edges[tuple(sorted((a, b)))].append(i)
        self.near = defaultdict(set)
        for owners in edges.values():
            for i in owners:
                self.near[i].update(set(owners) - {i})

    def inverse(self, face, xy):
        q = self.v[list(self.q[face])]
        p = q[:, [0, 2]]
        u = w = 0.5
        for _ in range(25):
            weights = np.array([(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w])
            J = np.column_stack(((1 - w) * (p[1] - p[0]) + w * (p[2] - p[3]), (1 - u) * (p[3] - p[0]) + u * (p[2] - p[1])))
            if abs(np.linalg.det(J)) < 1e-14:
                return None
            d = np.linalg.solve(J, weights @ p - xy)
            u -= d[0]
            w -= d[1]
            if max(abs(d)) < 1e-12:
                break
        if min(u, w, 1 - u, 1 - w) < -1e-08:
            return None
        weights = np.array([(1 - u) * (1 - w), u * (1 - w), u * w, (1 - u) * w])
        weights = np.maximum(weights, 0)
        weights /= weights.sum()
        return dict(face=face, donors=self.q[face], weights=weights, point=weights @ q, margin=min(u, w, 1 - u, 1 - w))

    def sample_from_vertex(self, vertex, xy):
        start = self.v[vertex, [0, 2]]
        active = set(self.at[vertex])
        walk = []
        if np.linalg.norm(xy - start) < 1e-11:
            return dict(donors=(vertex,), weights=np.array([1.0]), point=self.v[vertex].copy(), walk=[])
        for t in np.linspace(1 / 64, 1, 64):
            point = start + t * (xy - start)
            found = [x for fi in active if (x := self.inverse(fi, point)) is not None]
            inside = [x for x in found if x['margin'] > 1e-07]
            if len(inside) > 1:
                raise ValueError('Multiple interior charts reachable from the source vertex')
            choices = inside or found
            if not choices:
                raise ValueError('Connected chart walk left its local donor domain')
            if any((np.linalg.norm(x['point'] - choices[0]['point']) > 1e-07 for x in choices)):
                raise ValueError('Shared boundary is geometrically discontinuous')
            item = min(choices, key=lambda x: tuple(sorted((tuple(self.v[z]) for z in x['donors']))))
            fi = item['face']
            walk.append(fi)
            active = {fi} | self.near[fi]
        item['walk'] = list(dict.fromkeys(walk))
        return item

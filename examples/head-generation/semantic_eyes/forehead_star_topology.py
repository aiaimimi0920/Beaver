"""Conforming all-quad closed-star refinement with frozen source lineage."""
from collections import Counter
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from forehead_flow_rule import forehead_chart, forehead_convex_chart


def forehead_edge_counts(faces):
    return Counter(tuple(sorted((a, b))) for q in faces
                   for a, b in zip(q, q[1:]+q[:1]))


def forehead_guide_corners(faces, path):
    """Find turns that follow adjacent edges of one face, rather than a flow."""
    result = []
    incident = {}
    for fi, q in enumerate(faces):
        for i in q:
            incident.setdefault(i, []).append(fi)
    for station, (a, b, c) in enumerate(zip(path, path[1:], path[2:]), 1):
        for fi in incident[b]:
            q = faces[fi]
            k = q.index(b)
            if len(q) == 4 and {q[(k-1) % 4], q[(k+1) % 4]} == {a, c}:
                result.append({'station': station, 'vertex': b, 'face': fi,
                               'opposite': q[(k+2) % 4]})
    return result


class ForeheadStarTopology:
    """Split radial edges and each incident quad into three conforming quads.

    The outer star boundary is unchanged. Every new point is sampled directly
    from an original authoring quad, never from mutable intermediate fields.
    Child edges retain explicit original-edge provenance for crease transport.
    """
    def __init__(self, vertices, faces, regions, paths):
        self.old = np.asarray(vertices, float)
        self.original = [list(map(int, q)) for q in faces]
        self.regions = list(map(int, regions))
        self.points = self.old.tolist()
        self.old_chart = forehead_chart(self.old)
        self.chart = self.old_chart.tolist()
        self.faces = [q[:] for q in self.original]
        self.owners = list(range(len(faces)))
        self.paths = {name: list(map(int, p)) for name, p in paths.items()}
        self.lineage, self.operations, self.surfaces = {}, [], {}
        self.original_edges = forehead_edge_counts(self.original)
        self.edge_parent = {edge: edge for edge in self.original_edges}

    def add_point(self, chart_point, owner_ids):
        key = tuple(sorted(set(owner_ids)))
        if key not in self.surfaces:
            if any(self.regions[i] != 1 or len(self.original[i]) != 4 for i in key):
                raise ValueError('Forehead refinement crosses a non-skin donor')
            surface = AuthorQuadSurface(self.old_chart,
                [self.original[i] for i in key], [1]*len(key))
            surface.vertices = self.old.copy()
            self.surfaces[key] = surface
        donors, weights, point = self.surfaces[key].lineage(chart_point[0], chart_point[2])
        index = len(self.points)
        self.points.append(np.asarray(point, np.float32).astype(float).tolist())
        self.chart.append(list(map(float, chart_point)))
        self.lineage[index] = {'donors': list(donors), 'weights': list(weights)}
        return index

    def refine_star(self, pivot):
        selected = [i for i, q in enumerate(self.faces) if pivot in q]
        if not selected or any(len(self.faces[i]) != 4 or
                               self.regions[self.owners[i]] != 1 for i in selected):
            raise ValueError('Forehead pivot is not a skin-quad star')
        counts = forehead_edge_counts(self.faces)
        neighbors = sorted({j for i in selected for q in [self.faces[i]]
                            for j in (q[(q.index(pivot)-1) % 4], q[(q.index(pivot)+1) % 4])})
        if len(neighbors) != len(selected) or any(
                counts[tuple(sorted((pivot, j)))] != 2 for j in neighbors):
            raise ValueError('Forehead refinement requires a closed conforming star')
        mids, centers = {}, {}
        for j in neighbors:
            donors = [self.owners[i] for i in selected if j in self.faces[i]]
            mid = self.add_point((np.asarray(self.chart[pivot])+self.chart[j])/2, donors)
            mids[j] = mid
            edge = tuple(sorted((pivot, j)))
            parent = self.edge_parent.pop(edge)
            self.edge_parent[tuple(sorted((pivot, mid)))] = parent
            self.edge_parent[tuple(sorted((mid, j)))] = parent
        for i in selected:
            q = self.faces[i]
            k = q.index(pivot)
            a, b, c, d = q[k+1:]+q[:k+1]
            center = self.add_point(np.mean(np.asarray(self.chart)[q], axis=0), [self.owners[i]])
            e, f = mids[c], mids[a]
            children = [[a, b, center, f], [b, c, e, center], [center, e, d, f]]
            centers[i] = center
            self.faces[i] = children[0]
            self.faces.extend(children[1:])
            self.owners.extend([self.owners[i], self.owners[i]])
            for child in children:
                for first, second in zip(child, child[1:]+child[:1]):
                    self.edge_parent.setdefault(tuple(sorted((first, second))), None)
        for name, path in self.paths.items():
            expanded = [path[0]]
            for a, b in zip(path, path[1:]):
                other = b if a == pivot else a if b == pivot else None
                if other in mids:
                    expanded.append(mids[other])
                expanded.append(b)
            self.paths[name] = expanded
        self.operations.append({'pivot': pivot, 'faces': selected, 'midpoints': mids,
                                'centers': centers})
        return centers

    def finish(self):
        counts = forehead_edge_counts(self.faces)
        if any(n not in (1, 2) for n in counts.values()):
            raise ValueError('Forehead refinement created a nonmanifold edge')
        if {e for e, n in counts.items() if n == 1} != {
                e for e, n in self.original_edges.items() if n == 1}:
            raise ValueError('Forehead refinement changed an open boundary')
        before = len(self.old)-len(self.original_edges)+len(self.original)
        after = len(self.points)-len(counts)+len(self.faces)
        if before != after or set(counts) != set(self.edge_parent):
            raise ValueError('Forehead refinement lost Euler or edge provenance')
        if not np.array_equal(np.asarray(self.points)[:len(self.old)], self.old):
            raise ValueError('Topological refinement moved an original point')
        changed = [i for i, (q, owner) in enumerate(zip(self.faces, self.owners))
                   if q != self.original[owner]]
        forehead_convex_chart(forehead_chart(self.points), np.asarray([self.faces[i] for i in changed]))
        for path in self.paths.values():
            if len(path) != len(set(path)) or any(tuple(sorted((a, b))) not in counts
                                                for a, b in zip(path, path[1:])):
                raise ValueError('Refined forehead guide is not a simple edge path')
            if forehead_guide_corners(self.faces, path):
                raise ValueError('Refined forehead guide still follows a quad corner')
        return {'vertices': self.points, 'chart': self.chart, 'faces': self.faces,
                'face_owners': self.owners, 'lineage': self.lineage,
                'paths': self.paths, 'operations': self.operations,
                'retiled_old_faces': sorted({self.owners[i] for i in changed}),
                'retiled_new_faces': changed,
                'edge_lineage': [{'edge': list(e), 'parent': list(p) if p is not None else None}
                                 for e, p in self.edge_parent.items()],
                'euler': before, 'old_vertices': len(self.old)}

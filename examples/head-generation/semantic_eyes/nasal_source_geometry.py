"""Geometry of a current authoring-quad surface, independent of saved meshes."""
from collections import Counter, defaultdict
import numpy as np


def cross2(a, b):
    return float(a[0]*b[1]-a[1]*b[0])


def polygon_area(points):
    return abs(sum(cross2(a,b) for a,b in zip(points,np.roll(points,-1,axis=0))))*.5


def clipped_polygon(points, boundary):
    """Convex source-chart clipping; keep geometry, not mesh index ordering."""
    boundary = np.asarray(boundary,dtype=float)
    if sum(cross2(a,b) for a,b in zip(boundary,np.roll(boundary,-1,axis=0))) < 0:
        boundary = boundary[::-1]
    polygon = list(np.asarray(points,dtype=float))
    for a,b in zip(boundary,np.roll(boundary,-1,axis=0)):
        previous,polygon = polygon,[]
        if not previous:
            break
        for c,d in zip(previous,previous[1:]+previous[:1]):
            dc,de = cross2(b-a,c-a),cross2(b-a,d-a)
            if dc >= -1e-14:
                polygon.append(c)
            if (dc < 0) != (de < 0):
                polygon.append(c+(d-c)*dc/(dc-de))
    return np.asarray(polygon)


def point_segment_distance(point, a, b):
    delta = b-a
    length2 = float(delta@delta)
    if length2 <= 1e-20:
        raise ValueError('Collapsed source boundary edge')
    t = np.clip(float((point-a)@delta)/length2,0.,1.)
    return float(np.linalg.norm(point-(a+t*delta)))


def segment_distance(a, b, c, d):
    if (cross2(b-a,c-a)*cross2(b-a,d-a) < 0
            and cross2(d-c,a-c)*cross2(d-c,b-c) < 0):
        return 0.
    return min(point_segment_distance(a,c,d),point_segment_distance(b,c,d),
               point_segment_distance(c,a,b),point_segment_distance(d,a,b))


def edge_incidence(faces):
    result = defaultdict(list)
    for index,face in enumerate(faces):
        cycle = list(map(int,face))
        for a,b in zip(cycle,cycle[1:]+cycle[:1]):
            result[tuple(sorted((a,b)))].append(index)
    return result


def patch_boundary(faces, selected):
    faces = [tuple(map(int,f)) for f in faces]
    counts = Counter(tuple(sorted((a,b))) for index in selected
                     for a,b in zip(faces[index],faces[index][1:]+faces[index][:1]))
    if any(n > 2 for n in counts.values()):
        raise ValueError('Nonmanifold source patch')
    return {edge for edge,n in counts.items() if n == 1}


def canonical_boundary(vertices, faces, selected, side=1):
    """Start at the unique lower end of the nasal midline boundary chain."""
    faces = [tuple(map(int,f)) for f in faces]
    edges = patch_boundary(faces,selected)
    adjacency = defaultdict(set)
    for a,b in edges:
        adjacency[a].add(b)
        adjacency[b].add(a)
    if not adjacency or any(len(v) != 2 for v in adjacency.values()):
        raise ValueError('Source patch boundary is not a simple cycle')
    axis = {i for i in adjacency if abs(vertices[i,0]) <= 1e-8}
    ends = [i for i in axis if len(adjacency[i]&axis) == 1]
    if len(ends) != 2 or len(axis) < 3:
        raise ValueError('Expected one nasal-axis boundary chain')
    start = min(ends,key=lambda i:vertices[i,2])
    if abs(vertices[ends[0],2]-vertices[ends[1],2]) < .005:
        raise ValueError('Ambiguous nasal boundary anchor')
    first = [i for i in adjacency[start] if side*vertices[i,0] > 1e-8]
    if len(first) != 1:
        raise ValueError('Nasal boundary has no unique outward neighbor')
    cycle = [start,first[0]]
    while True:
        following = list(adjacency[cycle[-1]]-{cycle[-2]})
        if len(following) != 1:
            raise ValueError('Ambiguous boundary continuation')
        nxt = following[0]
        if nxt == start:
            break
        if nxt in cycle:
            raise ValueError('Boundary contains multiple cycles')
        cycle.append(nxt)
    if len(cycle) != len(adjacency):
        raise ValueError('Source patch has more than one boundary')
    used = {i for j in selected for i in faces[j]}
    all_edges = {tuple(sorted((a,b))) for j in selected
                 for a,b in zip(faces[j],faces[j][1:]+faces[j][:1])}
    if len(used)-len(all_edges)+len(selected) != 1:
        raise ValueError('Source replacement domain is not a disk')
    return cycle


class AuthorQuadSurface:
    """The actual current quad surface, including its exact straight boundaries.

    This is deliberately distinct from a nonlinear procedural field between
    the old boundary vertices and from an arbitrary export triangulation.
    """
    def __init__(self, vertices, faces, regions):
        self.vertices = np.asarray(vertices,dtype=float).copy()
        self.quads = [tuple(map(int,f)) for f,r in zip(faces,regions) if r == 1 and len(f) == 4]
        if not len(self.quads) or not np.all(np.isfinite(self.vertices)):
            raise ValueError('Missing finite authoring skin quads')
        self.xy = self.vertices[np.asarray(self.quads,dtype=int)][:,:,[0,2]]
        self.lower,self.upper = self.xy.min(axis=1),self.xy.max(axis=1)

    def lineage(self, x, height):
        point = np.array([x,height],dtype=float)
        candidates = np.flatnonzero(np.all(point >= self.lower-1e-9,axis=1)
                                    & np.all(point <= self.upper+1e-9,axis=1))
        found = []
        for index in candidates:
            q = self.xy[index]
            u = v = .5
            for _ in range(20):
                weights = np.array([(1-u)*(1-v),u*(1-v),u*v,(1-u)*v])
                jac = np.column_stack(((1-v)*(q[1]-q[0])+v*(q[2]-q[3]),
                                       (1-u)*(q[3]-q[0])+u*(q[2]-q[1])))
                if abs(float(np.linalg.det(jac))) < 1e-14:
                    break
                delta = np.linalg.solve(jac,weights@q-point)
                u -= float(delta[0]); v -= float(delta[1])
                if max(abs(delta)) < 1e-12:
                    break
            weights = np.array([(1-u)*(1-v),u*(1-v),u*v,(1-u)*v])
            if (-1e-7 <= u <= 1+1e-7 and -1e-7 <= v <= 1+1e-7
                    and np.linalg.norm(weights@q-point) < 1e-9):
                weights = np.maximum(weights,0.)
                weights /= weights.sum()
                ids = self.quads[index]
                found.append((ids,weights,weights@self.vertices[list(ids)],min(u,v,1-u,1-v)))
        if not found:
            raise ValueError('Nasal sample lies outside the current source surface')
        if sum(margin > 1e-7 for *_,margin in found) > 1:
            raise ValueError('Overlapping authoring-quad chart')
        if any(np.linalg.norm(value-found[0][2]) > 1e-7 for _,_,value,_ in found[1:]):
            raise ValueError('Discontinuous authoring-quad boundary')
        ids,weights,value,_ = found[0]
        return tuple(ids),tuple(map(float,weights)),value

    def sample(self, x, height):
        return self.lineage(x,height)[2].copy()

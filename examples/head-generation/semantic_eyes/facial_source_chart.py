"""Utility-only positive-weight capped source UV chart, with numeric certification.

Temporary caps affect parameterization only. Nothing edits author geometry,
fields, guides, or source topology. SciPy is imported only when solving.
"""
from collections import Counter, defaultdict
import numpy as np


def facial_chart_integer_rows(values, width, name):
    array = np.asarray(values)
    if (array.ndim != 2 or array.shape[1] != width
            or array.dtype.kind not in 'iu' or len(array) == 0):
        raise ValueError('Invalid ' + name)
    return array.astype(np.int64, copy=True)


def facial_chart_cross(a, b):
    return a[..., 0]*b[..., 1] - a[..., 1]*b[..., 0]


def facial_chart_overlap_area(first, second):
    """Convex triangle intersection area, used only for certification."""
    polygon = list(first)
    for a, b in zip(second, np.roll(second, -1, axis=0)):
        old, polygon = polygon, []
        if not old:
            break
        for x, y in zip(old, old[1:] + old[:1]):
            sx = float(facial_chart_cross(b-a, x-a))
            sy = float(facial_chart_cross(b-a, y-a))
            if sx >= 0:
                polygon.append(x)
            if (sx < 0) != (sy < 0):
                polygon.append(x + (y-x)*(sx/(sx-sy)))
    if len(polygon) < 3:
        return 0.
    points = np.asarray(polygon)
    return abs(float(np.sum(facial_chart_cross(
        points, np.roll(points, -1, axis=0))))) / 2


def facial_chart_validate(vertices, quads, triangles, owners, boundaries, ids):
    points = np.asarray(vertices, dtype=float)
    ids = np.asarray(ids)
    if (points.ndim != 2 or points.shape[1] != 3
            or not np.isfinite(points).all() or ids.shape != (len(points),)
            or ids.dtype.kind not in 'iu' or len(set(ids.tolist())) != len(ids)):
        raise ValueError('Invalid source vertices or original IDs')
    quads = facial_chart_integer_rows(quads, 4, 'source quads')
    triangles = facial_chart_integer_rows(triangles, 3, 'oriented triangles')
    owners = np.asarray(owners)
    if (owners.shape != (len(triangles),) or owners.dtype.kind not in 'iu'
            or np.any(owners < 0) or np.any(owners >= len(quads))):
        raise ValueError('Invalid triangle owner indices')
    known = set(ids.tolist())
    if set(quads.ravel()) != known or any(len(set(q)) != 4 for q in quads):
        raise ValueError('Source domain must use exactly the supplied original IDs')
    if len({tuple(sorted(q)) for q in quads}) != len(quads):
        raise ValueError('Duplicate source quads')
    canonical = []
    unit = np.array([[0., 0.], [1., 0.], [1., 1.], [0., 1.]])
    def cycle(t):
        t = tuple(t)
        return min(t[i:]+t[:i] for i in range(len(t)))
    for face, q in enumerate(quads):
        selected = triangles[owners == face]
        options = [((q[0], q[1], q[2]), (q[0], q[2], q[3])),
                   ((q[0], q[1], q[3]), (q[1], q[2], q[3]))]
        if len(selected) != 2 or not any(
                {cycle(t) for t in selected} == {cycle(t) for t in option}
                for option in options):
            raise ValueError('Triangles must partition every oriented source quad')
    for tri, owner in zip(triangles, owners):
        canonical.append(unit[[list(quads[owner]).index(i) for i in tri]])
    directed = Counter((int(i), int(j)) for tri in triangles
                       for i, j in zip(tri, np.roll(tri, -1)))
    edges = Counter(tuple(sorted(e)) for e in directed.elements())
    if any(n > 2 for n in edges.values()) or any(
            directed[(i, j)] != 1 or directed[(j, i)] != 1
            for (i, j), n in edges.items() if n == 2):
        raise ValueError('Nonmanifold or inconsistently oriented source edges')
    expected = {e for e in directed if edges[tuple(sorted(e))] == 1}
    loops = []
    for values in boundaries:
        loop = np.asarray(values)
        if (loop.ndim != 1 or loop.dtype.kind not in 'iu' or len(loop) < 3
                or len(set(loop.tolist())) != len(loop) or not set(loop) <= known):
            raise ValueError('Invalid ordered boundary loop')
        loops.append(loop.astype(np.int64).tolist())
    if not loops or sum(map(len, loops)) != len(set(i for l in loops for i in l)):
        raise ValueError('Boundary loops must be vertex-disjoint')
    supplied = {(i, j) for l in loops for i, j in zip(l, l[1:]+l[:1])}
    if supplied != expected:
        raise ValueError('Ordered boundaries do not equal actual source boundary')
    if len(known)-len(edges)+len(triangles) != 2-len(loops):
        raise ValueError('Expected connected genus-zero surface with these holes')
    boundary_vertices = {i for loop in loops for i in loop}
    for vertex in known:
        link = defaultdict(set)
        for triangle in triangles:
            if vertex in triangle:
                a, b = [int(i) for i in triangle if i != vertex]
                link[a].add(b)
                link[b].add(a)
        seen, queue = {next(iter(link))}, [next(iter(link))]
        for i in queue:
            for j in link[i]-seen:
                seen.add(j)
                queue.append(j)
        degrees = sorted(map(len, link.values()))
        expected_ends = 2 if vertex in boundary_vertices else 0
        if (len(seen) != len(link) or degrees.count(1) != expected_ends
                or any(d not in (1, 2) for d in degrees)):
            raise ValueError('Nonmanifold source vertex link')
    return points.copy(), quads, triangles, owners.copy(), loops, np.array(canonical)


def facial_source_chart(vertices, quads, triangles, triangle_owners, boundaries,
                        *, original_ids=None, overlap_tolerance=1e-13):
    """Build a unit-circle chart; boundaries[0] is the explicit outer loop.

    Faces/triangles contain original IDs, owners index quads. Every source quad
    must have exactly two chosen consistently oriented triangles. Extra source
    vertices, omitted faces, nonmanifold edges, and ambiguous boundaries reject.
    The returned certificate is numeric, not exact-arithmetic geometry proof.
    """
    if original_ids is None:
        original_ids = np.arange(len(vertices), dtype=np.int64)
    tolerance = float(overlap_tolerance)
    if not np.isfinite(tolerance) or tolerance < 0 or tolerance > 1e-10:
        raise ValueError('Invalid unit-chart overlap certification tolerance')
    points, quads, triangles, owners, loops, canonical = facial_chart_validate(
        vertices, quads, triangles, triangle_owners, boundaries, original_ids)
    ids = np.asarray(original_ids, dtype=np.int64)
    index = {int(i): j for j, i in enumerate(ids)}
    graph = {j: set() for j in range(len(ids))}
    local_triangles = np.array([[index[int(i)] for i in t] for t in triangles])
    for t in local_triangles:
        for i, j in zip(t, np.roll(t, -1)):
            graph[i].add(j)
            graph[j].add(i)
    seen, queue = {0}, [0]
    for i in queue:
        for j in graph[i]-seen:
            seen.add(j)
            queue.append(j)
    if len(seen) != len(ids):
        raise ValueError('Disconnected source domain')
    caps = []
    for loop in loops[1:]:
        local = [index[i] for i in loop]
        center = len(graph)
        graph[center] = set(local)
        for i in local:
            graph[i].add(center)
        caps.extend([[j, i, center] for i, j in zip(local, local[1:]+local[:1])])
    outer = [index[i] for i in loops[0]]
    theta = np.arange(len(outer))*2*np.pi/len(outer)
    uv = np.zeros((len(graph), 2))
    uv[outer] = np.column_stack((np.cos(theta), np.sin(theta)))
    free = sorted(set(graph)-set(outer))
    lookup = {i: j for j, i in enumerate(free)}
    # Lazy optional numerical dependency: importing this utility never imports SciPy.
    from scipy.sparse import lil_matrix
    from scipy.sparse.linalg import spsolve
    matrix = lil_matrix((len(free), len(free)))
    rhs = np.zeros((len(free), 2))
    for i in free:
        row = lookup[i]
        matrix[row, row] = len(graph[i])
        for j in graph[i]:
            if j in lookup:
                matrix[row, lookup[j]] = -1
            else:
                rhs[row] += uv[j]
    if free:
        uv[free] = spsolve(matrix.tocsr(), rhs)
    if not np.isfinite(uv).all():
        raise ValueError('Nonfinite parameterization solve')
    all_triangles = np.vstack((local_triangles, np.array(caps, dtype=int).reshape(-1, 3)))
    z = uv[all_triangles]
    area = facial_chart_cross(z[:, 1]-z[:, 0], z[:, 2]-z[:, 0])
    if np.any(area <= 0):
        raise ValueError('Nonpositive chart triangle; no injectivity certificate')
    low, high = z.min(axis=1), z.max(axis=1)
    maximum_overlap = 0.
    for i, first in enumerate(z):
        candidates = np.flatnonzero(np.all(
            np.minimum(high[i], high)-np.maximum(low[i], low) > 0, axis=1))
        for j in candidates[candidates > i]:
            overlap = facial_chart_overlap_area(first, z[j])
            maximum_overlap = max(maximum_overlap, overlap)
            if overlap > tolerance:
                raise ValueError('Chart triangle overlap; no injectivity certificate')
    boundary_edges = [(index[i], index[j]) for loop in loops
                      for i, j in zip(loop, loop[1:]+loop[:1])]
    clearance = float('inf')
    def point_segment_distance(p, a, b):
        direction = b-a
        denominator = float(direction @ direction)
        if denominator <= 0:
            raise ValueError('Collapsed chart boundary edge')
        fraction = np.clip(float((p-a) @ direction)/denominator, 0., 1.)
        return float(np.linalg.norm(p-a-fraction*direction))
    for k, (i, j) in enumerate(boundary_edges):
        for m, n in boundary_edges[k+1:]:
            if len({i, j, m, n}) < 4:
                continue
            a, b, c, d = uv[[i, j, m, n]]
            if (facial_chart_cross(b-a, c-a)*facial_chart_cross(b-a, d-a) < 0
                    and facial_chart_cross(d-c, a-c)*facial_chart_cross(d-c, b-c) < 0):
                raise ValueError('Crossed chart boundary')
            distance = min(point_segment_distance(a, c, d),
                           point_segment_distance(b, c, d),
                           point_segment_distance(c, a, b),
                           point_segment_distance(d, a, b))
            clearance = min(clearance, distance)
            if distance <= 0:
                raise ValueError('Touching nonadjacent chart boundary')
    # Including caps in overlap certification ensures holes neither overlap each
    # other nor actual skin. Caps/virtual IDs are NEVER returned as domain faces.
    residual = float(np.max(np.abs(matrix.tocsr() @ uv[free]-rhs), initial=0.))
    return dict(original_ids=ids.tolist(), uv=uv[:len(ids)].tolist(),
                triangles=triangles.tolist(), source_quad_owners=owners.tolist(),
                canonical_triangle_st=canonical.tolist(), boundary_loops=loops,
                certificate=dict(actual_triangles=len(triangles),
                    virtual_cap_triangles=len(caps), virtual_caps_exported=False,
                    minimum_signed_double_area=float(area.min()),
                    maximum_triangle_overlap=maximum_overlap,
                    minimum_boundary_clearance=clearance,
                    overlap_tolerance=tolerance, linear_residual=residual,
                    source_geometry_modified=False, full_model_success=False))

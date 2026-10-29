"""Exact triangle distance with conservative enclosing-sphere pruning."""
import numpy as np
from scipy.spatial import cKDTree
from chin_source_shape import chin_shape_closest_triangles


def forehead_exact_distance(points, vertices, triangles, details=False):
    points = np.asarray(points, float)
    geometry = np.asarray(vertices, float)[np.asarray(triangles, int)]
    if not len(geometry) or not np.isfinite(geometry).all() or not np.isfinite(points).all():
        raise ValueError('Invalid exact-distance geometry')
    edges = geometry[:, 1:]-geometry[:, :1]
    if np.any(np.linalg.norm(np.cross(edges[:, 0], edges[:, 1]), axis=1) <= 1e-20):
        raise ValueError('Degenerate triangle in exact-distance geometry')
    centers = geometry.mean(axis=1)
    radii = np.linalg.norm(geometry-centers[:, None], axis=2).max(axis=1)
    tree = cKDTree(centers)
    maximum_radius = radii.max()
    output, best_ids, best_hits = [], [], []
    # Every triangle lies in its vertex-enclosing ball. A known triangle hit
    # gives an upper bound; no closer triangle can be outside bound + radius.
    padding = 1e-10*max(1., np.abs(geometry).max(), np.abs(points).max(initial=0.))
    for offset in range(0, len(points), 512):
        batch = points[offset:offset+512]
        nearest = tree.query(batch, k=1)[1]
        initial_hits = chin_shape_closest_triangles(geometry[nearest], batch)
        upper = np.linalg.norm(initial_hits-batch, axis=1)
        if not np.isfinite(upper).all():
            raise ValueError('Nonfinite exact-distance upper bound')
        candidates = tree.query_ball_point(batch, upper+maximum_radius+padding)
        retained = []
        for point, bound, possible, fallback in zip(batch, upper, candidates, nearest):
            ids = np.asarray(possible, int)
            lower = np.linalg.norm(centers[ids]-point, axis=1)-radii[ids]
            ids = ids[lower <= bound+padding]
            if not len(ids):
                ids = np.asarray([fallback], int)
            retained.append(ids)
        counts = np.asarray([len(ids) for ids in retained])
        ids = np.concatenate(retained)
        owners = np.repeat(np.arange(len(batch)), counts)
        hits = chin_shape_closest_triangles(geometry[ids], batch[owners])
        distances = np.linalg.norm(hits-batch[owners], axis=1)
        starts = np.r_[0, np.cumsum(counts)]
        choices = [starts[i]+int(distances[starts[i]:starts[i+1]].argmin())
                   for i in range(len(batch))]
        output.extend(distances[choices])
        best_ids.extend(ids[choices])
        best_hits.extend(hits[choices])
    result = np.asarray(output)
    return (result, np.asarray(best_ids), np.asarray(best_hits)) if details else result

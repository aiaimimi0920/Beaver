"""Actual source-edge metrics; no asset data, solver, or acceptance substitution."""
import math
from numbers import Real

def finite_scalar(value):
    if isinstance(value, bool) or not isinstance(value, Real) or (not math.isfinite(value)):
        raise ValueError('nonfinite derived numeric value')
    return value

def finite_vector(value):
    return tuple((finite_scalar(x) for x in value))

def validate_row(row, values):
    if not isinstance(row, (list, tuple)) or not 1 <= len(row) <= 2:
        raise ValueError('donor row')
    ids = []
    weights = []
    for item in row:
        if not isinstance(item, (list, tuple)) or len(item) != 2:
            raise ValueError('donor row')
        uid, w = item
        if not isinstance(uid, str) or uid not in values:
            raise ValueError('missing donor')
        finite_scalar(w)
        if not 0 < w <= 1:
            raise ValueError('nonconvex row')
        ids.append(uid)
        weights.append(w)
    if len(set(ids)) != len(ids) or math.fsum(weights) != 1.0:
        raise ValueError('nonconvex row')

def sub(a, b):
    return finite_vector((x - y for x, y in zip(a, b)))

def dot(a, b):
    value = sum((x * y for x, y in zip(a, b)))
    if not math.isfinite(value):
        raise ValueError('nonfinite derived dot product')
    return value

def norm(a):
    return math.sqrt(dot(a, a))

def cross(a, b):
    return finite_vector((a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0]))

def mix(a, b, t):
    return finite_vector(((1 - t) * x + t * y for x, y in zip(a, b)))

def metrics(points, path):
    edges = [sub(points[b], points[a]) for a, b in zip(path, path[1:])]
    lengths = [norm(e) for e in edges]
    if not lengths or min(lengths) <= 0:
        raise ValueError('degenerate rail')
    mean = finite_scalar(sum(lengths) / len(lengths))
    turns = [math.atan2(norm(cross(a, b)), dot(a, b)) for a, b in zip(edges, edges[1:])]
    units = [tuple((x / length for x in edge)) for edge, length in zip(edges, lengths)]
    bending = [sub(b, a) for a, b in zip(units, units[1:])]
    variation = [sub(b, a) for a, b in zip(bending, bending[1:])]
    return {'tangent_bending': sum((dot(x, x) for x in bending)) / max(1, len(bending)),
        'tangent_variation': sum((dot(x, x) for x in variation)) / max(1, len(variation)),
        'lengths': lengths,
        'ratio': finite_scalar(max(lengths) / min(lengths)),
        'worst_relative': max((abs(x / mean - 1) for x in lengths)),
        'turns': turns,
        'peak_turn': max(turns, default=0)}

def arc(points, path):
    lengths = metrics(points, path)['lengths']
    out = [0.0]
    for length in lengths:
        out.append(finite_scalar(out[-1] + length))
    return out

def row_at(path, distances, value):
    if value <= 0:
        return ((path[0], 1.0),)
    if value >= distances[-1]:
        return ((path[-1], 1.0),)
    for j in range(len(path) - 1):
        if distances[j] <= value <= distances[j + 1]:
            if value == distances[j]:
                return ((path[j], 1.0),)
            if value == distances[j + 1]:
                return ((path[j + 1], 1.0),)
            t = (value - distances[j]) / (distances[j + 1] - distances[j])
            return ((path[j], 1 - t), (path[j + 1], t))
    raise ValueError('outside source rail')

def evaluate(values, row):
    validate_row(row, values)
    vectors = {uid: finite_vector(values[uid]) for uid, _ in row}
    if len({len(v) for v in vectors.values()}) != 1:
        raise ValueError('donor dimensions')
    if len(row) == 1:
        return vectors[row[0][0]]
    return finite_vector((sum((finite_scalar(weight * vectors[uid][k]) for uid,
        weight in row)) for k in range(len(vectors[row[0][0]]))))

def edge_metrics(points, edges):
    lengths = [norm(sub(points[b], points[a])) for a, b in edges]
    if not lengths or min(lengths) <= 0:
        raise ValueError('degenerate edge group')
    mean = finite_scalar(sum(lengths) / len(lengths))
    return {'ratio': finite_scalar(max(lengths) / min(lengths)),
        'worst_relative': max((abs(x / mean - 1) for x in lengths)),
        'lengths': lengths}

def quad_metrics(points, faces):
    aspects = []
    angles = []
    for face in faces.values():
        v = face['vertices']
        lengths = [norm(sub(points[v[(j + 1) % 4]], points[v[j]])) for j in range(4)]
        if min(lengths) <= 0:
            raise ValueError('degenerate quad')
        aspects.append(finite_scalar(max(lengths) / min(lengths)))
        for j in range(4):
            a = sub(points[v[(j - 1) % 4]], points[v[j]])
            b = sub(points[v[(j + 1) % 4]], points[v[j]])
            angles.append(math.atan2(norm(cross(a, b)), dot(a, b)))
    return {'max_aspect': max(aspects), 'min_angle': min(angles), 'max_angle': max(angles)}

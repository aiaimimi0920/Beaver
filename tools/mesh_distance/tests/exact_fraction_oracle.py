"""Independent standard-library Fraction point/triangle minimum squared distance."""
from fractions import Fraction as F


def squared_distance(triangle, point):
    a, b, c = [[F(float(x)) for x in row] for row in triangle]
    p = [F(float(x)) for x in point]
    def sub(x, y):
        return [v - w for v, w in zip(x, y)]
    def dot(x, y):
        return sum(v * w for v, w in zip(x, y))
    candidates = []
    for start, end in ((a, b), (b, c), (c, a)):
        edge, offset = sub(end, start), sub(p, start)
        length = dot(edge, edge)
        fraction = max(F(0), min(F(1), dot(offset, edge) / length)) if length else F(0)
        hit = [x + fraction * y for x, y in zip(start, edge)]
        candidates.append(dot(sub(p, hit), sub(p, hit)))
    e, f, w = sub(b, a), sub(c, a), sub(p, a)
    aa, bb, cc, dd, ee = dot(e, e), dot(e, f), dot(f, f), dot(w, e), dot(w, f)
    determinant = aa * cc - bb * bb
    if determinant:
        u, v = (dd * cc - ee * bb) / determinant, (ee * aa - dd * bb) / determinant
        if u >= 0 and v >= 0 and u + v <= 1:
            hit = [x + u * y + v * z for x, y, z in zip(a, e, f)]
            candidates.append(dot(sub(p, hit), sub(p, hit)))
    return min(candidates)

"""Independent Decimal(100) triangle/edge oracle for bounded synthetic tests."""
from decimal import Decimal, localcontext
import numpy as np


def decimal_triangle(triangle, point):
    """Independent exhaustive triangle/edge oracle, Decimal(100), no BVH."""
    def sub(a, b):
        return [x-y for x, y in zip(a, b)]
    def dot(a, b):
        return sum(x*y for x, y in zip(a, b))
    def cross(a, b):
        return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
    with localcontext() as ctx:
        ctx.prec = 100
        a, b, c = [[Decimal(float(x)) for x in row] for row in triangle]
        p = [Decimal(float(x)) for x in point]
        ab, ac, ap = sub(b, a), sub(c, a), sub(p, a)
        n = cross(ab, ac)
        n2 = dot(n, n)
        candidates = []
        for x, y in [(a, b), (b, c), (c, a)]:
            e = sub(y, x)
            e2 = dot(e, e)
            t = min(Decimal(1), max(Decimal(0), dot(sub(p, x), e)/e2)) if e2 else Decimal(0)
            candidates.append([x[k]+t*e[k] for k in range(3)])
        if n2:
            u, v = dot(cross(ap, ac), n)/n2, dot(cross(ab, ap), n)/n2
            if u >= 0 and v >= 0 and u+v <= 1:
                candidates.append([a[k]+u*ab[k]+v*ac[k] for k in range(3)])
        distances = [dot(sub(p, h), sub(p, h)) for h in candidates]
        i = min(range(len(distances)), key=distances.__getitem__)
        return float(distances[i].sqrt()), np.array([float(x) for x in candidates[i]])

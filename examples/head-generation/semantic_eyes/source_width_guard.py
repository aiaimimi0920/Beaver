"""Exact float32 lateral-width guard. Proof and scope: WIDTH-REVERSAL-GUARD.md."""
from fractions import Fraction as F
from itertools import combinations
from math import isfinite, nextafter
from struct import pack, unpack


def f32(x):
    return unpack('f', pack('f', x))[0]


def limit_lateral_width_step(old, candidate, faces, moved, axes=(0, 1, 2), origin=0.):
    """Prevent new widening-chart reversals; retain existing narrowing.

    Test all touched same-side triples and revalidate stored float32 positions.
    """
    lat, dep, up = axes
    assert sorted(axes) == [0, 1, 2]
    old = [tuple(p) for p in old]
    candidate = [tuple(p) for p in candidate]
    moved = set(moved)
    assert len(old) == len(candidate)
    assert all(isfinite(c) and f32(c) == c for points in (old, candidate)
               for p in points for c in p), 'Expected finite stored float32 endpoints'
    assert all(p[dep] == q[dep] and p[up] == q[up]
               for p, q in zip(old, candidate)), 'Width guard requires lateral-only moves'
    assert all(p == candidate[i] for i, p in enumerate(old) if i not in moved)
    ro = F(origin)
    original = [tuple(F(c) for c in p) for p in old]
    proposed = [tuple(F(c) for c in p) for p in candidate]
    bound, protected, reversing = F(1), [], 0
    hemispheres = []
    for i in moved:
        x0, x1 = original[i][lat] - ro, proposed[i][lat] - ro
        assert x0, 'The envelope must not move a symmetry-plane vertex'
        side = 1 if x0 > 0 else -1
        hemispheres.append((i, side))
        if side*x1 < 0:
            bound = min(bound, x0/(x0-x1))
    for face in faces:
        if not moved.intersection(face):
            continue
        for ids in combinations(face, 3):
            a, b, c = (original[i] for i in ids)
            xs = [p[lat] - ro for p in (a, b, c)]
            if not (all(x > 0 for x in xs) or all(x < 0 for x in xs)):
                continue
            side = 1 if xs[0] > 0 else -1
            nx = ((b[dep]-a[dep])*(c[up]-a[up])
                  - (b[up]-a[up])*(c[dep]-a[dep]))
            if not nx:
                continue  # No x(depth,height) graph; nx cannot change in this edit.
            sign = -side * (1 if nx > 0 else -1)
            coeff = tuple(sign*v for v in (b[dep]-c[dep], c[dep]-a[dep], a[dep]-b[dep]))
            q0 = sum(k*original[i][lat] for i, k in zip(ids, coeff))
            if q0 <= 0:
                continue  # Do not globally enforce monotonicity or erase old narrowing.
            q1 = sum(k*proposed[i][lat] for i, k in zip(ids, coeff))
            protected.append((ids, coeff))
            if q1 < 0:
                reversing += 1
                bound = min(bound, q0/(q0-q1))

    def trial(alpha):
        points = list(old)
        for i in moved:
            p = list(old[i])
            p[lat] = f32(old[i][lat] + alpha*(candidate[i][lat]-old[i][lat]))
            points[i] = tuple(p)
        return points

    def valid(points):
        x = [F(p[lat]) for p in points]
        return (all(side*(x[i]-ro) >= 0 for i, side in hemispheres)
                and all(sum(k*x[i] for i, k in zip(ids, coeff)) >= 0
                        for ids, coeff in protected))

    alpha = float(bound)
    if F(alpha) > bound:
        alpha = nextafter(alpha, 0.)
    result = trial(alpha)
    rounding_steps = 0
    if not valid(result):
        # Float32 rounding can cross the bound. Find a validated safe step;
        # 53 is binary64 precision, not an aesthetic margin or maximality claim.
        low, high, result = 0., alpha, list(old)
        for _ in range(53):
            mid = (low+high)*.5
            if mid == low or mid == high:
                break
            points = trial(mid)
            rounding_steps += 1
            if valid(points):
                low, result = mid, points
            else:
                high = mid
        alpha = low
    assert valid(result)
    assert all(p[dep] == old[i][dep] and p[up] == old[i][up] for i, p in enumerate(result))
    assert all(p == old[i] for i, p in enumerate(result) if i not in moved)
    return result, {'protected_triangle_charts': len(protected),
                    'proposed_reversal_count': reversing,
                    'analytic_scale_limit': float(bound),
                    'applied_scale': alpha,
                    'float32_repair_steps': rounding_steps,
                    'stored_coordinate_no_new_reversal': True}


def adapt_width_envelope(source):
    import inspect
    anchor = "    assert not invalid,'Frontal ear-under envelope inverted a face'\n"
    if source.count(anchor) != 1:
        raise ValueError("Unexpected ear envelope area-gate contract")
    addition = "    accepted = [tuple(v.co) for v in data.vertices]\n    limited, lateral_guard = limit_lateral_width_step(\n        [tuple(p) for p in old], accepted,\n        [tuple(poly.vertices) for poly in data.polygons], set(targets))\n    for i in targets:\n        data.vertices[i].co = limited[i]\n    data.update()\n    area_valid = all(poly.area > 1e-12 and poly.normal.dot(normal) > 0\n                     for poly, normal in zip(data.polygons, normals)\n                     if any(i in targets for i in poly.vertices))\n    lateral_guard['area_fallback'] = not area_valid\n    if not area_valid:\n        for i in targets:\n            data.vertices[i].co = old[i]\n        data.update()\n        lateral_guard['applied_scale'] = 0.0\n    assert all(poly.area > 1e-12 and poly.normal.dot(normal) > 0\n               for poly, normal in zip(data.polygons, normals)\n               if any(i in targets for i in poly.vertices)), 'Limited envelope inverted a face'\n"
    source = source.replace(anchor, anchor + addition)
    old_return = "return {'selected_vertices':len(targets),"
    if source.count(old_return) != 1:
        raise ValueError("Unexpected ear envelope report contract")
    source = source.replace(old_return, "return {'lateral_width_guard':lateral_guard,'selected_vertices':len(targets),")
    imports = "\nfrom fractions import Fraction as F\nfrom itertools import combinations\nfrom math import isfinite, nextafter\nfrom struct import pack, unpack\n"
    return source + imports + inspect.getsource(f32) + "\n" + inspect.getsource(limit_lateral_width_step)

"""Current source-quad coordinates and bounded harmonic patch movement."""


def capture_mouth_patch_fields(vertices, rings, angles, height, calibration):
    """Capture lip rim before the independent outer-frame resample."""
    import numpy as np
    points = np.asarray(vertices, dtype=float)
    rim, upper = {}, set()
    for k, angle in enumerate(angles):
        start = points[rings[0][k], [0, 2]]
        ray = points[rings[-1][k], [0, 2]]-start
        length2 = float(ray@ray)
        if length2 <= 1e-16:
            raise ValueError('Collapsed mouth sampling ray')
        sine = float(np.sin(angle))
        key = 'upper_lip_edge_depth_delta' if sine >= 0 else 'lower_lip_edge_depth_delta'
        for ring in rings[:-1]:
            index = ring[k]
            t = float((points[index, [0, 2]]-start)@ray/length2)
            rim[index] = height*calibration[key]*sine*sine*(1-t)**2
            if sine > .001:
                upper.add(index)
    return {'rings': [list(r) for r in rings], 'rim': rim, 'upper': upper}


def mouth_patch_quad_weights(point, vertices, faces):
    """Invert current source quads, with ambiguity and domain checks."""
    import numpy as np
    solutions = []
    for face in faces:
        q = vertices[list(face)][:, [0, 2]]
        if np.any(point < q.min(axis=0)-1e-9) or np.any(point > q.max(axis=0)+1e-9):
            continue
        u = v = .5
        for _ in range(20):
            weights = np.array([(1-u)*(1-v), u*(1-v), u*v, (1-u)*v])
            jac = np.column_stack(((1-v)*(q[1]-q[0])+v*(q[2]-q[3]),
                                   (1-u)*(q[3]-q[0])+u*(q[2]-q[1])))
            if abs(np.linalg.det(jac)) < 1e-14:
                break
            delta = np.linalg.solve(jac, weights@q-point)
            u -= delta[0]
            v -= delta[1]
            if max(abs(delta)) < 1e-12:
                break
        weights = np.array([(1-u)*(1-v), u*(1-v), u*v, (1-u)*v])
        if (-1e-7 <= u <= 1+1e-7 and -1e-7 <= v <= 1+1e-7
                and np.linalg.norm(weights@q-point) < 1e-9):
            solutions.append((face, weights, min(u, v, 1-u, 1-v)))
    if not solutions:
        raise ValueError('New mouth station lies outside current source quads')
    if sum(margin > 1e-7 for _, _, margin in solutions) > 1:
        raise ValueError('Ambiguous overlapping source-quad sampling domain')
    return [(face, weights) for face, weights, _ in solutions]


def mouth_patch_feasible_step(old, direction, faces):
    """First convexity boundary along one direction; never a per-point clamp."""
    import numpy as np
    def cross(a, b):
        return a[0]*b[1]-a[1]*b[0]
    first = float('inf')
    for face in faces:
        q = old[list(face)][:, [0, 2]]
        d = direction[list(face)]
        edges = np.roll(q, -1, axis=0)-q
        change = np.roll(d, -1, axis=0)-d
        signs = [cross(edges[k], edges[(k+1)%4]) for k in range(4)]
        if not (min(signs) > 1e-12 or max(signs) < -1e-12):
            raise ValueError('Input mouth source quad is not strictly convex')
        for k in range(4):
            j = (k+1)%4
            coefficients = [cross(change[k], change[j]),
                            cross(change[k], edges[j])+cross(edges[k], change[j]), signs[k]]
            while len(coefficients) > 1 and abs(coefficients[0]) < 1e-18:
                coefficients.pop(0)
            for root in np.roots(coefficients):
                if abs(root.imag) < 1e-10 and root.real > 1e-10:
                    first = min(first, float(root.real))
    step = min(1., .9*first)
    if not np.isfinite(step) or step <= 0:
        raise ValueError('No positive harmonic source sampling step')
    return step, None if not np.isfinite(first) else first


def mouth_patch_quad_quality(points, faces):
    """Both source-quad diagonals must remain nondegenerate and coherent."""
    import numpy as np
    for face in faces:
        q = points[list(face)]
        e = np.roll(q[:, [0, 2]], -1, axis=0)-q[:, [0, 2]]
        cross = e[:, 0]*np.roll(e[:, 1], -1)-e[:, 1]*np.roll(e[:, 0], -1)
        if not (min(cross) > 1e-12 or max(cross) < -1e-12):
            raise ValueError('Redistributed mouth quad is not convex')
        normals = [np.cross(q[j]-q[i], q[k]-q[i])
                   for i, j, k in ((0,1,2), (0,2,3), (1,2,3), (1,3,0))]
        lengths = [float(np.linalg.norm(n)) for n in normals]
        if min(lengths) <= 1e-12:
            raise ValueError('Redistributed mouth quad is degenerate')
        if min(float(normals[a]@normals[b])/(lengths[a]*lengths[b])
               for a,b in ((0,1),(2,3))) <= .1:
            raise ValueError('Redistributed mouth quad diagonal disagreement')

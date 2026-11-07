"""Redistribute upper lip supports on semantic rays, with original shape guards."""


def refine_lip_patch(verts, rings, angles, terms, guarded_depth, height, calibration):
    import numpy as np
    from scipy.optimize import minimize

    ids = np.asarray(rings, dtype=int)
    a = np.asarray(angles, dtype=float)
    base = np.asarray(verts, dtype=float)[ids]
    if base.shape != (5, len(a), 3) or len(a) < 5:
        raise ValueError("Unexpected joint lip patch contract")
    if not np.all(np.isfinite(base)) or not np.all(np.isfinite(a)) or np.any(np.diff(a) <= 0):
        raise ValueError("Non-finite or unordered lip patch")
    if max(abs(base[0, 0, 0]), abs(base[0, -1, 0])) > 1e-8:
        raise ValueError("Lip patch requires midline endpoints")
    sine = np.sin(a)
    selected = np.flatnonzero(sine > 1e-7)
    active = [(r, int(k)) for k in selected for r in (1, 2)]
    if not active:
        raise ValueError("No upper lip patch supports")
    rays = base[-1][:, [0, 2]] - base[0][:, [0, 2]]
    ray_length = np.linalg.norm(rays, axis=1)
    if np.any(ray_length <= 1e-8):
        raise ValueError("Collapsed semantic lip ray")
    unit = rays / ray_length[:, None]
    quads = [(r, k) for r in range(4) for k in range(len(a)-1)
             if any((rr, kk) in active for rr in (r, r+1) for kk in (k, k+1))]
    faces = [[(r, k), (r+1, k), (r+1, k+1), (r, k+1)] for r, k in quads]
    edges = sorted({tuple(sorted((p, q))) for face in faces
                    for p, q in zip(face, face[1:]+face[:1])})

    def turns(points):
        mirrored = points[-2:0:-1].copy()
        mirrored[:, 0] *= -1
        loop = np.concatenate((points, mirrored))
        u, v = loop-np.roll(loop, 1, axis=0), np.roll(loop, -1, axis=0)-loop
        norms = np.linalg.norm(u, axis=1)*np.linalg.norm(v, axis=1)
        return np.degrees(np.arccos(np.clip(np.sum(u*v, axis=1)/norms, -1, 1)))[:len(a)]

    def radial(points):
        d = np.diff(points[:, selected], axis=0)
        norms = np.linalg.norm(d[:-1], axis=2)*np.linalg.norm(d[1:], axis=2)
        return np.degrees(np.arccos(np.clip(np.sum(d[:-1]*d[1:], axis=2)/norms, -1, 1))).T

    def lengths(points):
        return np.array([np.linalg.norm(points[p]-points[q]) for p, q in edges])

    def normals(points):
        q = np.array([[points[p] for p in f] for f in faces])
        return np.array([np.cross(q[:, j]-q[:, i], q[:, k]-q[:, i])
                         for i, j, k in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0))])

    def quality(points, reference):
        n, original = normals(points), normals(reference)
        norm = np.linalg.norm(n, axis=2)
        orientation = np.sum(n*original, axis=2)/(norm*np.linalg.norm(original, axis=2))
        diagonals = np.concatenate([np.sum(n[i]*n[j], axis=1)/(norm[i]*norm[j])
                                    for i, j in ((0, 1), (2, 3))])
        return np.r_[orientation.ravel()-.1, diagonals-.1, (norm.ravel()-1e-12)*1e6]

    old_circum = np.array([turns(base[r]) for r in (1, 2)])
    old_radial, old_lengths = radial(base), lengths(base)
    old_gaps = np.linalg.norm(np.diff(base[:, selected], axis=0), axis=2)
    affected = np.zeros_like(old_circum, dtype=bool)
    for k in selected:
        affected[:, max(0, k-1):min(len(a), k+2)] = True
    cache = {}

    def evaluate(values):
        key = tuple(values)
        if key in cache:
            return cache[key]
        points = base.copy()
        checks = []
        for (r, k), (along, depth_move) in zip(active, np.reshape(values, (-1, 2))):
            points[r, k, [0, 2]] += unit[k]*along/1000
            if k in (0, len(a)-1):
                points[r, k, 0] = base[r, k, 0]
            points[r, k, 1] -= depth_move/1000
            p = points[r, k]
            t = np.dot(p[[0, 2]]-base[0, k, [0, 2]], rays[k])/ray_length[k]**2
            raw, original = terms(float(p[0]), float(p[2]))
            dz = height*calibration['upper_lip_edge_depth_delta']*sine[k]**2*(1-t)**2
            depth = -p[1]-dz
            checks.extend(((.0042-1e-7-abs(raw-original))*1000,
                           (.0052-1e-7-abs(depth-original))*1000,
                           (.0015-1e-7-abs(depth-raw))*1000))
        circum = np.array([turns(points[r]) for r in (1, 2)])
        rad, length = radial(points), lengths(points)
        gaps = np.linalg.norm(np.diff(points[:, selected], axis=0), axis=2)
        margin = np.where(affected, -.003, .0001)
        checks.extend(((old_circum+margin-circum)/10).ravel())
        checks.extend(((old_radial[:, 0]+.0001-rad[:, 0])/10).ravel())
        checks.extend(((old_radial[:, 1:]+2.98-rad[:, 1:])/10).ravel())
        checks.extend((length-.9001*old_lengths)*1000)
        checks.extend((1.0999*old_lengths-length)*1000)
        checks.extend(((gaps-np.minimum(old_gaps, .001))*1000).ravel())
        checks.extend(quality(points, base))
        for k in selected:
            stations = (points[:, k][:, [0, 2]]-base[0, k, [0, 2]])@rays[k]/ray_length[k]**2
            checks.extend(np.diff(stations)-.005)
        objective = (np.mean(rad[:, 0]**2)/100+.25*np.mean(rad[:, 1:]**2)/100
                     +2*np.mean(((gaps[1]-gaps[0])*1000)**2)+.02*np.mean(values**2))
        result = objective, np.asarray(checks), points, rad, gaps
        cache.clear()
        cache[key] = result
        return result

    solved = minimize(lambda x: evaluate(x)[0], np.zeros(2*len(active)), method='SLSQP',
                      bounds=[(-1.5, 1.5)]*(2*len(active)),
                      constraints=[{'type':'ineq', 'fun':lambda x:evaluate(x)[1]}],
                      options={'maxiter':200, 'ftol':1e-8})
    objective, checks, points, rad, gaps = evaluate(solved.x)
    if not solved.success or not np.isfinite(objective) or not np.all(np.isfinite(checks)) or min(checks) < -1e-7:
        raise ValueError('Joint lip patch constraints failed: '+solved.message)
    quantized = points.astype(np.float32).astype(float)
    baseline_q = base.astype(np.float32).astype(float)
    q_circum = np.array([turns(quantized[r]) for r in (1, 2)])
    b_circum = np.array([turns(baseline_q[r]) for r in (1, 2)])
    q_radial, b_radial = radial(quantized), radial(baseline_q)
    q_lengths, b_lengths = lengths(quantized), lengths(baseline_q)
    if np.any(q_circum > b_circum+.001) or np.any(q_radial[:, 0] > b_radial[:, 0]):
        raise ValueError('Float32 joint lip patch tangent check failed')
    if np.any(q_radial[:, 1:] > b_radial[:, 1:]+3) or min(quality(quantized, baseline_q)) <= 0:
        raise ValueError('Float32 joint lip patch quad check failed')
    if np.any(q_lengths < .9*b_lengths) or np.any(q_lengths > 1.1*b_lengths):
        raise ValueError('Float32 joint lip patch spacing check failed')
    q_gaps = np.linalg.norm(np.diff(quantized[:, selected], axis=0), axis=2)
    b_gaps = np.linalg.norm(np.diff(baseline_q[:, selected], axis=0), axis=2)
    if np.any(q_gaps < np.minimum(b_gaps, .001)):
        raise ValueError('Float32 joint lip patch minimum gap failed')
    for k in selected:
        stations = (quantized[:, k][:, [0, 2]]-baseline_q[0, k, [0, 2]])@rays[k]/ray_length[k]**2
        if np.any(np.diff(stations) <= .005):
            raise ValueError('Float32 joint lip patch ordering failed')
    for candidate in (points, quantized):
        for r, k in active:
            p = candidate[r, k]
            x, y = float(p[0]), float(p[2])
            t = np.dot(p[[0, 2]]-base[0, k, [0, 2]], rays[k])/ray_length[k]**2
            dz = height*calibration['upper_lip_edge_depth_delta']*sine[k]**2*(1-t)**2
            raw, _ = terms(x, y)
            guarded_depth(x, y)
            checked = guarded_depth(x, y, -p[1]-dz-raw, upper_lip_local=True)+dz
            if abs(checked+p[1]) > 1e-10:
                raise ValueError('Joint lip patch reconstruction mismatch')
    for r, k in active:
        verts[int(ids[r, k])] = tuple(float(v) for v in points[r, k])
    return {'version':1, 'iterations':int(solved.nit), 'center_supports_free':True,
            'aperture_and_corners_fixed':True, 'original_reference_unchanged':True,
            'radial_before_deg':old_radial[:, 0].tolist(), 'radial_after_deg':rad[:, 0].tolist(),
            'support_gaps_before_m':old_gaps.tolist(), 'support_gaps_after_m':gaps.tolist(),
            'max_move_m':float(np.max(np.linalg.norm(points-base, axis=2))), 'visual_acceptance':False}

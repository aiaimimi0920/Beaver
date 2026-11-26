"""Redistribute a complete mouth/cheek source patch without moving landmarks."""


def redistribute_mouth_patch(vertices, faces, captured, levels, lateral, columns, terms):
    import numpy as np
    from scipy.sparse import lil_matrix
    from scipy.sparse.linalg import spsolve
    old = np.asarray(vertices, dtype=float)
    if not np.all(np.isfinite(old)) or len(captured) != 1:
        raise ValueError('Expected one finite half-face mouth patch')
    data = captured[0]
    rings = data['rings']
    if len(rings) != 5 or columns != len(lateral):
        raise ValueError('Mouth source-ring/grid contract changed')
    adj = [set() for _ in vertices]
    for face in faces:
        for a,b in zip(face, face[1:]+face[:1]):
            adj[a].add(b)
            adj[b].add(a)
    protected = set(rings[0]) | set(rings[1])
    free = set().union(*map(set, rings[2:]))
    for _ in range(2):
        free |= set().union(*(adj[i] for i in free))-protected
    free = {i for i in free if abs(old[i,0]) < .055 and 1.555 < old[i,2] < 1.614}
    nasal = {r*columns+c for r,h in enumerate(levels) for c,q in enumerate(lateral)
             if h >= 1.599 and q <= .18}
    protected |= nasal
    free -= protected
    ids = sorted(free)
    if not ids or any(old[i,0] < -1e-8 for i in ids):
        raise ValueError('Expected a nonempty positive half-face sampling patch')
    direction = np.zeros((len(old),2))
    # Mirror-Neumann height equation doubles off-axis neighbors at the midline.
    # Midline x stays exactly zero; other coordinates solve the same full graph.
    for axis, coordinate in enumerate((0,2)):
        active = [i for i in ids if axis == 1 or abs(old[i,0]) > 1e-8]
        slots = {index:k for k,index in enumerate(active)}
        matrix = lil_matrix((len(active),len(active)))
        rhs = np.zeros(len(active))
        for i in active:
            row = slots[i]
            for j in adj[i]:
                weight = 2 if axis == 1 and abs(old[i,0]) <= 1e-8 and abs(old[j,0]) > 1e-8 else 1
                matrix[row,row] += weight
                if j in slots:
                    matrix[row,slots[j]] -= weight
                else:
                    rhs[row] += weight*old[j,coordinate]
        solved = spsolve(matrix.tocsr(),rhs)
        if not np.all(np.isfinite(solved)):
            raise ValueError('Singular harmonic source patch')
        for i,value in zip(active,solved):
            direction[i,axis] = value-old[i,coordinate]
    affected = [tuple(f) for f in faces if set(f)&free]
    if any(len(f) != 4 for f in affected):
        raise ValueError('Mouth redistribution may only address source quads')
    step,boundary = mouth_patch_feasible_step(old,direction,affected)
    proposed = old.copy()
    proposed[:,0] += step*direction[:,0]
    proposed[:,2] += step*direction[:,1]
    rim = np.zeros(len(old))
    for i,value in data['rim'].items():
        rim[i] = value
    quads = [tuple(f) for f in faces if len(f) == 4
             and old[list(f),0].min() < .065
             and old[list(f),2].min() < 1.624 and old[list(f),2].max() > 1.545]
    source_ids = set().union(*map(set,quads))
    residual = np.zeros(len(old))
    for i in source_ids:
        raw,_ = terms(float(old[i,0]),float(old[i,2]))
        residual[i] = -old[i,1]-rim[i]-raw
    checks = []
    sampled_rim = {}
    for i in ids:
        samples = mouth_patch_quad_weights(proposed[i,[0,2]],old,quads)
        values = [(float(w@rim[list(f)]),float(w@residual[list(f)])) for f,w in samples]
        if max(np.linalg.norm(np.asarray(value)-values[0]) for value in values) > 1e-8:
            raise ValueError('Discontinuous source fields at shared quad boundary')
        dz,extra = values[0]
        sampled_rim[i] = dz
        raw,reference = terms(float(proposed[i,0]),float(proposed[i,2]))
        value = raw+extra
        upper = i in data['upper']
        limit = .0052 if upper else .0042
        if (abs(raw-reference) > .0042 or abs(value-reference) > limit
                or upper and abs(extra) > .0015):
            raise ValueError('Source redistribution exceeded original depth budget')
        proposed[i,1] = -(value+dz)
        checks.append((abs(value-reference),abs(extra) if upper else 0))
    move = np.linalg.norm(proposed-old,axis=1)
    # Explicit whole-patch station migration scope, not the preceding 3mm frame gate.
    if max(move) > .008:
        raise ValueError('Whole mouth-patch station migration exceeds 8mm trial bound')
    for candidate in (proposed,proposed.astype(np.float32).astype(float)):
        mouth_patch_quad_quality(candidate,affected)
        for face in affected:
            a = old[list(face)][:,[0,2]]
            b = candidate[list(face)][:,[0,2]]
            old_turn = ((a[1,0]-a[0,0])*(a[2,1]-a[1,1])-(a[1,1]-a[0,1])*(a[2,0]-a[1,0]))
            new_turn = ((b[1,0]-b[0,0])*(b[2,1]-b[1,1])-(b[1,1]-b[0,1])*(b[2,0]-b[1,0]))
            if old_turn*new_turn <= 0:
                raise ValueError('Source quad projection orientation changed')
        for i in ids:
            raw,reference = terms(float(candidate[i,0]),float(candidate[i,2]))
            value = -candidate[i,1]-sampled_rim[i]
            upper = i in data['upper']
            if (abs(raw-reference) > .0042 or abs(value-reference) > (.0052 if upper else .0042)
                    or upper and abs(value-raw) > .0015):
                raise ValueError('Stored source redistribution exceeded original depth budget')
    if not np.array_equal(proposed[list(protected)],old[list(protected)]):
        raise ValueError('Protected mouth/nasal source roles moved')
    if np.any(proposed[old[:,0] == 0,0] != 0):
        raise ValueError('Source midline symmetry changed')
    for i in ids:
        vertices[i] = tuple(proposed[i])
    return {'scope':'whole mouth/cheek source patch; not the preceding local 3mm frame gate',
            'half_face_stations':len(ids),'midline_stations':int(sum(old[i,0] == 0 for i in ids)),
            'uniform_step':step,'first_convexity_boundary':boundary,
            'max_station_migration_m':float(max(move)),'trial_migration_bound_m':.008,
            'max_original_depth_difference_m':max(a for a,b in checks),
            'max_upper_residual_m':max(b for a,b in checks),
            'aperture_first_support_nasal_fixed':True,'source_topology_unchanged':True,
            'field':'unchanged analytic raw surface plus current source-quad residual and rim',
            'validation':'Native pose, crossing and visual validation still required'}

"""Bounded authored lower-root stations for the validated slender oval profile."""


def flow_lower_root_stations(points, base, original, unit, fixed, quads):
    """Adjust source u1/u2 stations before the existing complete ear gates.

    Coordinates are normalized by the authored root span, not final mesh IDs.
    This is a retained-profile correction, not a universal ear fitting rule.
    """
    import numpy as np
    points = np.asarray(points, dtype=float)
    base = np.asarray(base, dtype=float)
    if points.shape != (40,3) or base.shape != points.shape or {2,3} & set(fixed):
        raise ValueError('Lower-root semantic station contract changed')
    if unit <= 1e-8:
        raise ValueError('Degenerate lower-root source scale')
    result = points.copy()
    side = 1. if original[0][0] > 0 else -1.
    offsets = {3:(.9345383048070901,.24970773356676892,.24827871511178387),
               2:(.554062142683339,.33810929783516713,.7685421727995846)}
    for index, delta in offsets.items():
        result[index] += np.asarray(delta)*[side,1.,1.]
    if np.linalg.norm(result-base,axis=1).max() >= 3.999999:
        raise ValueError('Lower-root correction exceeded original radial budget')
    # Check storage precision as well as the normalized source-domain bound.
    origin = np.asarray(original[0], dtype=float)
    stored = (result*unit+origin).astype('<f4').astype(float)
    stored[fixed] = np.asarray(original)[fixed]
    if np.linalg.norm(stored-np.asarray(original),axis=1).max() >= 3.999999*unit:
        raise ValueError('Stored lower-root geometry exceeded original radial budget')
    before = points*unit+origin
    if np.linalg.norm(stored[[2,3]]-before[[2,3]],axis=1).max() > .001:
        raise ValueError('Lower-root incremental movement exceeded one millimeter')
    affected = [q for q in quads if 2 in q or 3 in q]
    old_dot, _, _ = ear_quad_measures(points, affected)
    new_dot, _, _ = ear_quad_measures(result, affected)
    if new_dot.min() <= old_dot.min():
        raise ValueError('Lower-root worst quad twist did not improve')
    if np.linalg.norm(result[3]-result[2]) < np.linalg.norm(points[3]-points[2]):
        raise ValueError('Lower-root short segment became shorter')
    def chain_turns(values):
        directions = np.diff(values[[4,3,2,6]],axis=0)
        directions /= np.linalg.norm(directions,axis=1)[:,None]
        return np.degrees(np.arccos(np.clip((directions[:-1]*directions[1:]).sum(1),-1,1)))
    before_turns, after_turns = chain_turns(points), chain_turns(result)
    if after_turns.max() > min(before_turns.max(),80.1):
        raise ValueError('Lower-root chain still turns too abruptly')
    patterns = np.array([(0,1,2),(0,2,3),(0,1,3),(1,2,3)])
    def normals(values):
        triangles = values[np.asarray(affected)][:,patterns]
        return np.cross(triangles[:,:,1]-triangles[:,:,0],triangles[:,:,2]-triangles[:,:,0])
    old_normals, new_normals = normals(points), normals(result)
    old_lengths, new_lengths = np.linalg.norm(old_normals,axis=2), np.linalg.norm(new_normals,axis=2)
    alignment = (old_normals*new_normals).sum(2)/np.maximum(old_lengths*new_lengths,1e-20)
    if (new_lengths/np.maximum(old_lengths,1e-20)).min() < .9 or alignment.min() < .89999:
        raise ValueError('Lower-root local surface changed beyond retained bounds')

    return result, {'stations':[2,3], 'scale_m':float(unit),
                    'max_increment_mm':float(np.linalg.norm((result-points)*unit,axis=1).max()*1000),
                    'min_pair_dot_before':float(old_dot.min()),
                    'min_pair_dot_after':float(new_dot.min()),
                    'chain_turn_before':float(before_turns.max()),
                    'chain_turn_after':float(after_turns.max()),
                    'scope':'Slender oval source stations; full native validation required'}

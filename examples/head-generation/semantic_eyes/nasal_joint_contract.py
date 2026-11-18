"""Final-domain geometry checks for one detached nasal source transaction."""
import numpy as np


def nasal_oriented_cycle(face):
    q = tuple(map(int, face))
    return min(q[i:]+q[:i] for i in range(len(q)))


def nasal_quad_quality(vertices, quads):
    p = np.asarray(vertices, dtype=np.float32).astype(float)[np.asarray(quads, int)]
    if p.ndim != 3 or p.shape[1:] != (4,3) or not np.isfinite(p).all():
        raise ValueError('Joint nasal guard requires finite quads')
    a,b = np.roll(p,1,axis=1)-p, np.roll(p,-1,axis=1)-p
    norm = np.linalg.norm(a,axis=2)*np.linalg.norm(b,axis=2)
    if np.any(norm < 1e-12):
        raise ValueError('Joint nasal quad has a collapsed edge')
    angles = np.degrees(np.arccos(np.clip(np.sum(a*b,axis=2)/norm,-1,1)))
    cross = np.cross(a,b)[:,:,1]
    convex = np.all(cross>1e-12,axis=1)|np.all(cross< -1e-12,axis=1)
    bad = (angles.min(1)<10)|(angles.max(1)>170)|(~convex)
    return dict(min_angle=float(angles.min()),max_angle=float(angles.max()),
                invalid_faces=np.flatnonzero(bad).tolist(),checked_faces=len(p))


def validate_nasal_joint_domain(vertices, faces, regions, regularization, inner, proposed, final_faces):
    """Discard intermediate defects only when their exact cycles are removed."""
    removed = set(map(int,inner['removed_faces']))
    affected = set(map(int,regularization['affected_faces']))
    if not removed or max(removed)>=len(faces):
        raise ValueError('Invalid joint nasal removal ownership')
    if not all(regions[i]==1 and len(faces[i])==4 for i in removed):
        raise ValueError('Joint nasal removal is not skin quads')
    boundary = set(map(int,inner['boundary_positive']))
    # Both boundary halves must be checked, including every surviving neighbor.
    removed_vertices = {x for i in removed for x in faces[i]}
    kept_vertices = {x for i,q in enumerate(faces) if i not in removed for x in q}
    boundary |= removed_vertices & kept_vertices
    surviving = (affected | {i for i,q in enumerate(faces) if set(q)&boundary})-removed
    checks = [faces[i] for i in sorted(surviving)]+inner['new_quads']
    quality = nasal_quad_quality(proposed,checks)
    if quality['invalid_faces']:
        raise ValueError('Final joint nasal surviving or adjacent geometry failed')
    if not np.array_equal(np.asarray(proposed)[:len(vertices)], vertices):
        raise ValueError('Inner proposal moved retained source positions')
    expected = [q for i,q in enumerate(faces) if i not in removed]+inner['new_quads']
    if sorted(map(nasal_oriented_cycle,expected))!=sorted(map(nasal_oriented_cycle,final_faces)):
        raise ValueError('Joint nasal final face ownership mismatch')
    if (not regularization['outer_boundary_fixed'] or
            not regularization['protected_vertices_unchanged'] or
            regularization['maximum_move_m']>.00250001 or
            not inner['protected_first_support_untouched']):
        raise ValueError('Joint nasal source bounds changed')
    return dict(quality=quality,removed_cycles=[nasal_oriented_cycle(faces[i]) for i in sorted(removed)],
                surviving_affected_faces=len(affected-removed),boundary_adjacent_faces=len(surviving),
                deferred_intermediate_faces=sorted(affected&removed))

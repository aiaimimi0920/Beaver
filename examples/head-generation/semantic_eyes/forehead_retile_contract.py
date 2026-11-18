"""Source topology and frozen-donor contracts before native allocation."""
import numpy as np
from forehead_star_topology import forehead_edge_counts, forehead_guide_corners


def validate_forehead_retile(old, faces, regions, proposal):
    old = np.asarray(old, float)
    points = np.asarray(proposal['vertices'], np.float32).astype(float)
    final = [list(map(int, q)) for q in proposal['faces']]
    lineage = proposal['lineage']
    if not np.isfinite(points).all() or len(points) < len(old):
        raise ValueError('Invalid forehead retile vertex allocation')
    if set(range(len(old), len(points)))-set(lineage):
        raise ValueError('Appended forehead vertex lacks frozen lineage')
    if any(not np.array_equal(points[i], old[i]) for i in range(len(old)) if i not in lineage):
        raise ValueError('Forehead retile changed an unowned original vertex')
    before, after = forehead_edge_counts(faces), forehead_edge_counts(final)
    if any(n not in (1, 2) for n in after.values()):
        raise ValueError('Forehead retile is nonmanifold')
    if {e for e, n in before.items() if n == 1} != {e for e, n in after.items() if n == 1}:
        raise ValueError('Forehead retile changed an original open boundary')
    if len(old)-len(before)+len(faces) != len(points)-len(after)+len(final):
        raise ValueError('Forehead retile changed Euler characteristic')
    if len({frozenset(q) for q in final}) != len(final):
        raise ValueError('Forehead retile contains duplicate faces')
    cycles = set()
    for q, owner in zip(faces, regions):
        if owner == 1 and len(q) == 4:
            for ordered in (list(q), list(reversed(q))):
                cycles.update(tuple(ordered[k:]+ordered[:k]) for k in range(4))
    for i, row in lineage.items():
        donors, weights = tuple(row['donors']), np.asarray(row['weights'], float)
        if (len(donors) not in (2, 4) or len(set(donors)) != len(donors)
                or weights.shape != (len(donors),) or not np.isfinite(weights).all()
                or weights.min() < 0 or abs(weights.sum()-1) > 1e-8
                or any(j < 0 or j >= len(old) for j in donors)):
            raise ValueError('Invalid forehead frozen donor record')
        if len(donors) == 4 and (donors not in cycles or
                abs(weights[0]*weights[2]-weights[1]*weights[3]) > 1e-8):
            raise ValueError('Forehead donor is not an original bilinear skin quad')
        if len(donors) == 2 and tuple(sorted(donors)) not in before:
            raise ValueError('Forehead donor is not an original source edge')
        if not np.array_equal(points[i], np.asarray(weights @ old[list(donors)], np.float32)):
            raise ValueError('Forehead position differs from its frozen donor')
    old_ids, new_ids = set(proposal['affected_old']), set(proposal['affected_new'])
    for i, q in enumerate(final):
        owner = proposal['face_owners'][i]
        if owner < 0 or owner >= len(faces):
            raise ValueError('Forehead face has no original owner')
        if i in new_ids and (len(q) != 4 or regions[owner] != 1):
            raise ValueError('Forehead retile affects a non-skin quad')
        if i < len(faces) and i not in old_ids and list(faces[i]) != q:
            raise ValueError('Forehead retile changed an unrelated face cycle')
    for side in proposal['guide_paths'].values():
        for path in side.values():
            if any(tuple(sorted((a, b))) not in after for a, b in zip(path, path[1:])):
                raise ValueError('Forehead guide is not on the final mesh')
            if forehead_guide_corners(final, path):
                raise ValueError('Forehead guide retains a shared-quad elbow')
    if not np.array_equal(points[proposal['protected']], old[proposal['protected']]):
        raise ValueError('Forehead retile changed protected feature coordinates')
    return {'euler': len(old)-len(before)+len(faces),
            'open_edges': len([e for e, n in before.items() if n == 1]),
            'original_boundary_edges_exact': True, 'all_guides_are_mesh_edges': True,
            'shared_quad_guide_elbows': 0, 'frozen_donors_verified': True,
            'old_counts': [len(old), len(faces)], 'new_counts': [len(points), len(final)]}

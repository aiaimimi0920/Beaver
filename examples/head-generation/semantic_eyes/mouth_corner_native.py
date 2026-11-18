"""Prepare a small post-chin mouth-corner transaction from current source roles."""
import numpy as np
from scipy.spatial import cKDTree
from mouth_corner_rule import propose_lower_mouth_corner
from orbital_support_rule import orbital_turns, orbital_quad_quality, orbital_zero_blink_guard


def prepare_mouth_corner(vertices, faces, regions, features, nasal_paths, chin, calibrated_delta):
    old = np.asarray(vertices, dtype=float)
    if (chin.get('transaction', {}).get('committed') is not True
            or chin['transaction'].get('transfer_payload', {}).get('equal') is not True):
        raise ValueError('Mouth corner requires the completed current chin transaction')
    distance, matches = cKDTree(old).query(old * [-1, 1, 1], k=2)
    mirror = matches[:, 0]
    canonical = lambda q: min(tuple(q[k:]) + tuple(q[:k]) for k in range(len(q)))
    lookup = {canonical(q): i for i, q in enumerate(faces)}
    if len(lookup) != len(faces):
        raise ValueError('Duplicate current source cycle')
    protected = set(features['protected']) | {i for path in nasal_paths for i in path}
    chin_cycles = chin.get('proposal', {}).get('replacement_faces', [])
    if len(chin_cycles) != 37 or len({canonical(q) for q in chin_cycles}) != 37:
        raise ValueError('Current chin source certificate is incomplete')
    protected_chin_faces = set()
    for q in chin_cycles:
        for cycle in (q, [int(mirror[i]) for i in reversed(q)]):
            index = lookup.get(canonical(cycle))
            if index is None or regions[index] != 1:
                raise ValueError('Completed chin source cycle is missing')
            protected_chin_faces.add(index)
            protected.update(cycle)
    if len(protected_chin_faces) != 74:
        raise ValueError('Current bilateral chin source certificate is incomplete')
    protected |= {int(mirror[i]) for i in list(protected)}
    result = propose_lower_mouth_corner(old, faces, regions, features['oral'], protected)
    points = np.asarray(result['vertices'], dtype=np.float32).astype(float)
    lineage = {int(i): row for i, row in result['lineage'].items()}
    selected = result['roles']['selected']
    required = protected | {i for fi in selected for i in faces[fi]}
    required |= {i for q in result['ports'] for i in q}
    required |= {int(mirror[i]) for i in list(required)}
    for i in required:
        partner = int(mirror[i])
        if distance[i, 0] > 1e-7 or distance[i, 1] <= 1e-7 or int(mirror[partner]) != i:
            raise ValueError('Mouth corner mirror correspondence is ambiguous')
    negative = []
    for fi in selected:
        q = [int(mirror[i]) for i in reversed(faces[fi])]
        index = lookup.get(canonical(q))
        if index is None or regions[index] != 1:
            raise ValueError('Mirrored mouth support topology changed')
        negative.append(index)
    for i, row in list(lineage.items()):
        target = int(mirror[i])
        donors = [int(mirror[j]) for j in row['donors']]
        if target == i or target in lineage:
            raise ValueError('Mouth corner halves overlap')
        lineage[target] = {'donors': donors, 'weights': list(row['weights'])}
        points[target] = np.asarray(np.asarray(row['weights']) @ old[donors], dtype=np.float32)
    affected = sorted(set(selected) | set(negative))
    if len(lineage) != 4 or len(affected) != 12 or set(lineage) & protected:
        raise ValueError('Mouth corner transaction domain changed')
    used = {i for fi in affected for i in faces[fi]}
    if any(features['membership'][i] != {1} for i in used):
        raise ValueError('Mouth corner has mixed non-skin ownership')
    for reflected, ids in ((False, selected), (True, negative)):
        remap = lambda q: [int(mirror[i]) if reflected else i for i in q]
        path = remap(result['roles']['path'])
        ports = [remap(q) for q in result['ports']]
        rings = [remap(q) for q in result['roles']['rings']]
        quads = np.asarray([faces[i] for i in ids], dtype=int)
        a0, r0 = orbital_quad_quality(old, quads)
        a1, r1 = orbital_quad_quality(points, quads)
        if a1.min() < a0.min() - 1e-4 or a1.max() > a0.max() + 1e-4 or r1.max() > r0.max() + 1e-5:
            raise ValueError('Stored mouth quad quality regressed')
        if np.max(orbital_turns(points, ports) - orbital_turns(old, ports)) > 2:
            raise ValueError('Stored mouth interface exceeds explicit two-degree budget')
        triples = list(zip(path, path[1:], path[2:]))
        if max(orbital_turns(points, triples)) >= max(orbital_turns(old, triples)):
            raise ValueError('Stored mouth turn did not improve')
        for chain in [path] + rings:
            before = np.linalg.norm(np.diff(old[chain], axis=0), axis=1)
            after = np.linalg.norm(np.diff(points[chain], axis=0), axis=1)
            if after.max() / after.min() > before.max() / before.min():
                raise ValueError('Stored mouth support density regressed')
            if len(chain) == 4:
                growth = lambda e: max(np.maximum(e[1:] / e[:-1], e[:-1] / e[1:]))
                if growth(after) > growth(before):
                    raise ValueError('Stored mouth interval growth regressed')
        xy = points[quads][:, :, [0, 2]]
        edges = np.roll(xy, -1, axis=1) - xy
        cross = edges[:, :, 0] * np.roll(edges[:, :, 1], -1, axis=1)
        cross -= edges[:, :, 1] * np.roll(edges[:, :, 0], -1, axis=1)
        if np.min(cross) <= 1e-12:
            raise ValueError('Stored mouth source chart is not convex')
    blink = orbital_zero_blink_guard(old, points, sorted(used), calibrated_delta)
    metrics = {k: result[k] for k in ('port_before', 'port_after', 'turns_before', 'turns_after',
        'density_before', 'density_after', 'growth_before', 'growth_after', 'ring_before', 'ring_after')}
    return {'vertices': points, 'faces': faces, 'edited_faces': {}, 'lineage': lineage,
            'affected_faces': affected, 'roles': result['roles'], 'metrics': metrics,
            'additional_protected': protected, 'generated_blink_guard': blink,
            'scope': 'Current post-chin lower mouth support; fixed topology and exterior; '
                     'two-degree interface budget, nonworsening radial and circumoral density.'}

"""Prepare a bilateral current-source support-flow transaction without mutation."""
import numpy as np
from scipy.spatial import cKDTree
from orbital_support_bind import orbital_bind_strip
from orbital_support_rule import (orbital_propose_support, orbital_zero_blink_guard,
                                  orbital_turns, orbital_quad_quality)


def prepare_orbital_support(vertices, faces, regions, features, nasal_paths, calibrated_delta):
    old = np.asarray(vertices, dtype=float)
    distance, matches = cKDTree(old).query(old * [-1, 1, 1], k=2)
    mirror = matches[:, 0]
    protected = set(features['protected']) | {i for path in nasal_paths for i in path}
    protected |= {int(mirror[i]) for i in list(protected)}
    binding = orbital_bind_strip(old, faces, regions, features['apertures'], protected)
    result = orbital_propose_support(old, faces, binding['selected_faces'],
        binding['role_to_index'], binding['movable_roles'], binding['support_roles'], protected=protected)
    points = np.asarray(result['new_vertices'], dtype=np.float32).astype(float)
    lineage = {int(i): row for i, row in result['lineage'].items()}
    canonical = lambda q: min(tuple(q[k:]) + tuple(q[:k]) for k in range(len(q)))
    lookup = {canonical(q): i for i, q in enumerate(faces)}
    selected = result['source_face_ids']
    required = {i for fi in selected for i in faces[fi]} | set(result['chain'])
    required |= {i for triple in result['ports'] for i in triple}
    required |= {int(mirror[i]) for i in list(required)}
    for i in required:
        partner = int(mirror[i])
        if distance[i, 0] > 1e-7 or distance[i, 1] <= 1e-7 or int(mirror[partner]) != i:
            raise ValueError('Orbital source mirror correspondence is ambiguous')
        if features['membership'][i] != {1} or features['membership'][partner] != {1}:
            raise ValueError('Orbital source field has mixed non-skin ownership')
    negative = []
    for fi in selected:
        q = [int(mirror[i]) for i in reversed(faces[fi])]
        target = lookup.get(canonical(q))
        if target is None or regions[target] != 1:
            raise ValueError('Mirrored orbital support topology changed')
        negative.append(target)
    for i, row in list(lineage.items()):
        target = int(mirror[i])
        donors = [int(mirror[j]) for j in row['donors']]
        if target == i or target in lineage:
            raise ValueError('Orbital halves overlap')
        lineage[target] = {'donors': donors, 'weights': list(row['weights'])}
        points[target] = np.asarray(np.asarray(row['weights']) @ old[donors], dtype=np.float32)
    if set(lineage) & protected or len(lineage) != 10:
        raise ValueError('Orbital transaction changes protected source')
    affected = sorted(set(selected) | set(negative))
    if len(affected) != 22:
        raise ValueError('Orbital bilateral moving star changed')
    old_edges = {tuple(sorted((a, b))) for q in faces for a, b in zip(q, q[1:] + q[:1])}
    for reflected, ids in ((False, selected), (True, negative)):
        path = [int(mirror[i]) if reflected else i for i in result['chain']]
        ports = [[int(mirror[i]) if reflected else i for i in q] for q in result['ports']]
        if any(tuple(sorted((a, b))) not in old_edges for a, b in zip(path, path[1:])):
            raise ValueError('Orbital support chain is not an actual source path')
        quads = np.asarray([faces[i] for i in ids], dtype=int)
        old_angles, old_ratio = orbital_quad_quality(old, quads)
        new_angles, new_ratio = orbital_quad_quality(points, quads)
        if (new_angles.min() < old_angles.min() - 1e-5
                or new_angles.max() > old_angles.max() + 1e-5
                or new_ratio.max() > old_ratio.max() + 1e-5):
            raise ValueError('Stored orbital quad quality exceeds its original envelope')
        if np.max(orbital_turns(points, ports) - orbital_turns(old, ports)) > .5 + 1e-5:
            raise ValueError('Stored orbital transverse port turns too far')
        triples = list(zip(path, path[1:], path[2:]))
        if np.max(orbital_turns(points, triples)) >= np.max(orbital_turns(old, triples)):
            raise ValueError('Stored orbital support peak turn did not improve')
        for data in (old, points):
            xy = data[quads][:, :, [0, 2]]
            edges = np.roll(xy, -1, axis=1) - xy
            cross = edges[:, :, 0] * np.roll(edges[:, :, 1], -1, axis=1)
            cross -= edges[:, :, 1] * np.roll(edges[:, :, 0], -1, axis=1)
            if np.min(cross) <= 1e-12:
                raise ValueError('Stored orbital source chart is not strictly convex')
    blink_guard = orbital_zero_blink_guard(old, points, sorted(lineage), calibrated_delta)
    old_blink, new_blink = np.zeros_like(old, dtype=np.float32), np.zeros_like(old, dtype=np.float32)
    used = {i for fi in affected for i in faces[fi]}
    for i in used:
        old_closed = np.float32(float(old[i, 2]) + float(calibrated_delta(old[i])))
        new_closed = np.float32(float(points[i, 2]) + float(calibrated_delta(points[i])))
        old_blink[i, 2] = old_closed - np.float32(old[i, 2])
        new_blink[i, 2] = new_closed - np.float32(points[i, 2])
    return {'vertices': points, 'faces': faces, 'edited_faces': {}, 'lineage': lineage,
            'affected_faces': affected, 'roles': binding,
            'metrics': {key: result[key] for key in ('ports_before', 'ports_after', 'turns_before',
                'turns_after', 'quality_before', 'quality_after', 'source_relative_scale_m', 'solver')},
            'generated_blink_guard': blink_guard, 'old_blink_delta': old_blink, 'new_blink_delta': new_blink,
            'additional_protected': protected, 'scope': result['scope']}

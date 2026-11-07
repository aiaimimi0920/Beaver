"""Final-domain checks for a symmetric, bounded lower-mouth source retile."""
from collections import defaultdict
import numpy as np
from chin_source_roles import chin_oriented_cycle
from mouth_patch_geometry import mouth_patch_quad_quality


def chin_graph(faces):
    adjacency, incident, edges = defaultdict(set), defaultdict(list), defaultdict(list)
    for index, face in enumerate(faces):
        for k, vertex in enumerate(face):
            neighbor = face[(k + 1) % len(face)]
            adjacency[vertex].update((face[k - 1], neighbor))
            incident[vertex].append(index)
            edges[tuple(sorted((vertex, neighbor)))].append((vertex, neighbor))
    return adjacency, incident, edges


def chin_opposite(faces, adjacency, incident, first, middle):
    if len(adjacency[middle]) != 4:
        return None
    adjacent = {v for index in incident[middle] if first in faces[index]
                for v in faces[index]}
    candidates = adjacency[middle] - adjacent - {first}
    return next(iter(candidates)) if len(candidates) == 1 else None


def chin_turn(points, first, middle, last):
    a, b = points[middle] - points[first], points[last] - points[middle]
    denominator = np.linalg.norm(a) * np.linalg.norm(b)
    if denominator <= 1e-15:
        raise ValueError('Collapsed lower-mouth functional handoff')
    return float(np.degrees(np.arccos(np.clip(a @ b / denominator, -1, 1))))


def chin_validate_source_proposal(old, faces, points, final_faces, selected, lineage):
    old, points = np.asarray(old, float), np.asarray(points, float)
    selected, moved = set(selected), set(lineage)
    if old.shape != points.shape or not np.isfinite(points).all():
        raise ValueError('Invalid lower-mouth source point array')
    if len(faces) != len(final_faces) or len(selected) != 74 or len(moved) != 50:
        raise ValueError('Lower-mouth replacement changed its bounded symmetric domain')
    outside = sorted(set(range(len(old))) - moved)
    if not np.array_equal(old[outside], points[outside]):
        raise ValueError('Lower-mouth retile moved a protected source point')
    if any(tuple(face) != tuple(final_faces[i]) for i, face in enumerate(faces)
           if i not in selected):
        raise ValueError('Lower-mouth retile changed an outside source face')
    before = [tuple(faces[i]) for i in sorted(selected)]
    after = [tuple(final_faces[i]) for i in sorted(selected)]
    if any(len(q) != 4 or len(set(q)) != 4 for q in after):
        raise ValueError('Lower-mouth source replacement must contain genuine quads')
    used = {i for face in before for i in face}
    if {i for face in after for i in face} != used:
        raise ValueError('Lower-mouth retile changed source station ownership')
    before_adj, _, before_edges = chin_graph(before)
    after_adj, _, after_edges = chin_graph(after)
    boundary = {edge: values[0] for edge, values in before_edges.items() if len(values) == 1}
    if {edge: values[0] for edge, values in after_edges.items() if len(values) == 1} != boundary:
        raise ValueError('Lower-mouth retile changed oriented patch boundary')
    if len(before_edges) != len(after_edges):
        raise ValueError('Lower-mouth retile changed patch Euler characteristic')
    if any(len(values) > 2 or (len(values) == 2 and values[0] != values[1][::-1])
           for values in after_edges.values()):
        raise ValueError('Lower-mouth retile has inconsistent edge incidence')
    for vertex in used - moved:
        if len(before_adj[vertex]) != len(after_adj[vertex]):
            raise ValueError('Lower-mouth retile changed a fixed boundary fan degree')
    old_adj, old_inc, _ = chin_graph(faces)
    new_adj, new_inc, all_edges = chin_graph(final_faces)
    if len({tuple(sorted(face)) for face in final_faces}) != len(final_faces):
        raise ValueError('Lower-mouth retile introduced a duplicate face')
    if any(len(all_edges[edge]) != 2 for edge in after_edges):
        raise ValueError('Lower-mouth source patch is not manifold after assembly')
    ports = []
    for middle in sorted(used - moved):
        if len(old_adj[middle]) != 4:
            continue
        for first in sorted(old_adj[middle] - moved):
            last = chin_opposite(faces, old_adj, old_inc, first, middle)
            if last not in moved:
                continue
            new_last = chin_opposite(final_faces, new_adj, new_inc, first, middle)
            if new_last not in moved:
                raise ValueError('Lower-mouth retile broke a functional opposite-edge port')
            old_turn = chin_turn(old, first, middle, last)
            new_turn = chin_turn(points, first, middle, new_last)
            if (new_turn > old_turn + .5 or
                    (old[last] - old[middle]) @ (points[new_last] - points[middle]) <= 0):
                raise ValueError('Lower-mouth retile regressed a functional handoff')
            ports.append({'middle': middle, 'outside': first,
                          'old_turn': old_turn, 'new_turn': new_turn})
        if abs(old[middle, 0]) <= 1e-8:
            old_pair = sorted(old_adj[middle] & moved, key=lambda i: old[i, 0])
            new_pair = sorted(new_adj[middle] & moved, key=lambda i: points[i, 0])
            if not old_pair:
                continue
            if (len(old_pair) != 2 or len(new_pair) != 2
                    or not old[old_pair[0], 0] < 0 < old[old_pair[1], 0]
                    or not points[new_pair[0], 0] < 0 < points[new_pair[1], 0]
                    or chin_opposite(faces, old_adj, old_inc, old_pair[0], middle) != old_pair[1]
                    or chin_opposite(final_faces, new_adj, new_inc, new_pair[0], middle) != new_pair[1]):
                raise ValueError('Lower-mouth retile broke a mirrored axis port')
            old_turn = chin_turn(old, old_pair[0], middle, old_pair[1])
            new_turn = chin_turn(points, new_pair[0], middle, new_pair[1])
            if new_turn > old_turn + .5 or any(
                    (old[a] - old[middle]) @ (points[b] - points[middle]) <= 0
                    for a, b in zip(old_pair, new_pair)):
                raise ValueError('Lower-mouth retile regressed a mirrored axis handoff')
            ports.append({'middle': middle, 'kind': 'mirror_axis',
                          'old_turn': old_turn, 'new_turn': new_turn})
    donors = {chin_oriented_cycle(face) for face in before}
    for index, record in lineage.items():
        ids, weights = record['donors'], np.asarray(record['weights'], float)
        if (chin_oriented_cycle(ids) not in donors or weights.shape != (4,)
                or not np.isfinite(weights).all() or weights.min() < 0
                or abs(weights.sum() - 1) > 1e-8
                or abs(weights[0] * weights[2] - weights[1] * weights[3]) > 1e-8):
            raise ValueError('Lower-mouth lineage is not a current patch quad')
        expected = np.asarray(weights @ old[list(ids)], dtype=np.float32).astype(float)
        if not np.array_equal(expected, points[index]):
            raise ValueError('Stored lower-mouth point differs from frozen field lineage')
    mouth_patch_quad_quality(points, after)
    for source, replacement in zip(before, after):
        a, b = old[list(source)][:, [0, 2]], points[list(replacement)][:, [0, 2]]
        def signed_area(q):
            return float(np.sum(q[:, 0] * np.roll(q[:, 1], -1)
                                - q[:, 1] * np.roll(q[:, 0], -1)))
        if signed_area(a) * signed_area(b) <= 0:
            raise ValueError('Lower-mouth retile reversed its source chart orientation')
    return {'faces': len(after), 'moved': len(moved), 'fixed_points_exact': True,
            'oriented_boundary_preserved': True, 'functional_ports': ports,
            'runtime_acceptance': 'Pending actual export, field and expression verification'}

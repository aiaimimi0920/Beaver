"""Current-source-only fixed-topology infraorbital support redistribution.

CPU proposal only: callers own semantic binding, frozen field staging, paired
export/pose fidelity and an atomic native transaction. No file or preset IO.
"""
from collections import Counter, defaultdict
import numpy as np
from scipy.optimize import minimize
from nasal_source_geometry import AuthorQuadSurface, segment_distance


def orbital_turns(vertices, triples):
    q = vertices[np.asarray(triples, dtype=int)]
    a, b = q[:, 1] - q[:, 0], q[:, 2] - q[:, 1]
    denominator = np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1)
    if np.any(denominator <= 1e-20):
        raise ValueError('Collapsed orbital continuation')
    return np.degrees(np.arccos(np.clip(np.sum(a * b, axis=1) / denominator, -1, 1)))


def orbital_quad_quality(vertices, quads):
    q = vertices[quads]
    edges = np.roll(q, -1, axis=1) - q
    lengths = np.linalg.norm(edges, axis=2)
    if np.any(lengths <= 1e-10):
        raise ValueError('Collapsed orbital quad edge')
    cosine = np.sum(-np.roll(edges, 1, axis=1) * edges, axis=2)
    cosine /= np.roll(lengths, 1, axis=1) * lengths
    angles = np.degrees(np.arccos(np.clip(cosine, -1, 1)))
    return angles, lengths.max(axis=1) / lengths.min(axis=1)


def orbital_regular_triples(vertices, faces, moved):
    """Full-source adjacency, including triples whose fixed middle has a moved end."""
    neighbors, sectors = defaultdict(set), defaultdict(list)
    rank = {i: k for k, i in enumerate(sorted(range(len(vertices)),
             key=lambda i: tuple(vertices[i, [0, 2, 1]])))}
    for face in faces:
        for j, middle in enumerate(face):
            a, b = face[j - 1], face[(j + 1) % len(face)]
            neighbors[middle].update((a, b))
            sectors[middle].append((a, b))
    triples = []
    for middle, adjacent in neighbors.items():
        if len(adjacent) != 4 or len(sectors[middle]) != 4:
            continue
        for a in sorted(adjacent, key=rank.get):
            beside = {x if y == a else y for x, y in sectors[middle] if a in (x, y)}
            opposite = adjacent - beside - {a}
            if len(opposite) != 1:
                continue
            b = next(iter(opposite))
            if rank[a] < rank[b] and {a, middle, b} & moved:
                triples.append((a, middle, b))
    triples.sort(key=lambda t: tuple(rank[i] for i in t))
    return triples, neighbors


def orbital_propose_support(vertices, faces, selected_faces, role_to_index,
                            movable_roles, support_roles, *, protected=()):
    """Redistribute five semantic interior stations on eleven frozen source quads.

    The support roles describe the existing same-support edge chain, including
    one unchanged exterior station at each end. Numeric mesh IDs are only caller
    bindings, never selectors. All other vertices and every face cycle remain
    exact. Three adjacent transverse regular ports have a +0.5-degree cap;
    turns along the requested support chain are intentionally redistributed.
    """
    old = np.asarray(vertices, dtype=float).copy()
    source_faces = [list(map(int, face)) for face in faces]
    selected = list(map(int, selected_faces))
    moved = [int(role_to_index[name]) for name in movable_roles]
    path = [int(role_to_index[name]) for name in support_roles]
    quads = np.asarray([source_faces[i] for i in selected], dtype=int)
    if len(selected) != 11 or quads.shape != (11, 4) or len(set(moved)) != 5:
        raise ValueError('Unknown medial infraorbital strip contract')
    if not np.isfinite(old).all() or any(old[i, 0] <= 0 for i in moved):
        raise ValueError('Solve the finite positive half only, then mirror explicitly')
    incidence = Counter(tuple(sorted((a, b))) for face in quads.tolist()
                        for a, b in zip(face, face[1:] + face[:1]))
    boundary_edges = {edge for edge, count in incidence.items() if count == 1}
    boundary = {i for edge in boundary_edges for i in edge}
    used = set(quads.ravel())
    if (len(boundary) != 14 or len(used) != 19 or used - boundary != set(moved)
            or len(used) - len(incidence) + len(quads) != 1
            or max(incidence.values()) > 2):
        raise ValueError('Orbital domain is not the expected closed eleven-quad disk')
    boundary_neighbors = defaultdict(set)
    for a, b in boundary_edges:
        boundary_neighbors[a].add(b)
        boundary_neighbors[b].add(a)
    if any(len(row) != 2 for row in boundary_neighbors.values()):
        raise ValueError('Orbital boundary is not a simple cycle')
    reached, todo = set(), [next(iter(boundary))]
    while todo:
        index = todo.pop()
        if index not in reached:
            reached.add(index)
            todo.extend(boundary_neighbors[index] - reached)
    if reached != boundary:
        raise ValueError('Orbital boundary has multiple components')
    if set(moved) & set(protected):
        raise ValueError('Orbital strip includes a protected functional station')
    affected = {i for i, face in enumerate(source_faces) if set(face) & set(moved)}
    if affected != set(selected):
        raise ValueError('Orbital moving star escapes the selected disk')
    triples, neighbors = orbital_regular_triples(old, source_faces, set(moved))
    if any(b not in neighbors[a] for a, b in zip(path, path[1:])):
        raise ValueError('Semantic support path is not an actual source-edge path')
    if len(set(path) & set(moved)) != 2:
        raise ValueError('Expected exactly two movable same-support stations')
    ports = np.asarray([t for t in triples if t[1] not in path], dtype=int)
    support = np.asarray(list(zip(path, path[1:], path[2:])), dtype=int)
    if ports.shape != (3, 3):
        raise ValueError('Unknown transverse orbital interface contract')
    old_ports = orbital_turns(old, ports)
    old_turns = orbital_turns(old, support)
    old_angles, old_ratios = orbital_quad_quality(old, quads)
    surface = AuthorQuadSurface(old, quads, [1] * len(quads))
    rank = {i: k for k, i in enumerate(sorted(used, key=lambda i: tuple(old[i, [0, 2, 1]])))}
    edge_list = sorted(incidence, key=lambda e: tuple(sorted((rank[e[0]], rank[e[1]]))))
    edges = np.asarray(edge_list, dtype=int)
    edge_lengths = np.linalg.norm(old[edges[:, 0]] - old[edges[:, 1]], axis=1)
    scale = float(np.median(edge_lengths) / 4)
    start = old[moved][:, [0, 2]]
    xy = old[quads][:, :, [0, 2]]
    de = np.roll(xy, -1, axis=1) - xy
    original_cross = de[:, :, 0] * np.roll(de[:, :, 1], -1, axis=1)
    original_cross -= de[:, :, 1] * np.roll(de[:, :, 0], -1, axis=1)
    if np.any(original_cross <= 1e-12):
        raise ValueError('Current positive-side orbital source chart is not strictly convex')
    area = np.maximum(original_cross.mean(axis=1), 1e-12)
    cache = {}

    def sample(parameters):
        key = tuple(parameters)
        if key in cache:
            return cache[key]
        points, lineage = old.copy(), {}
        try:
            for index, station in zip(moved, start + parameters.reshape(-1, 2) * scale):
                donors, weights, point = surface.lineage(*station)
                points[index] = point
                lineage[index] = {'donors': list(donors), 'weights': list(weights)}
        except ValueError:
            return None
        cache[key] = points, lineage
        return points, lineage

    def objective(parameters):
        result = sample(parameters)
        if result is None:
            return 1e9
        points, _ = result
        turns = orbital_turns(points, support)
        lengths = np.linalg.norm(points[edges[:, 0]] - points[edges[:, 1]], axis=1)
        return float(np.sum((turns / 20) ** 2) + .06 * np.sum(parameters ** 2)
                     + .1 * np.sum(np.log(lengths / edge_lengths) ** 2))

    constraint_count = len(quads) * 15 + len(ports)

    def constraints(parameters):
        result = sample(parameters)
        if result is None:
            return np.full(constraint_count, -1000.)
        points, _ = result
        q = points[quads]
        xy = q[:, :, [0, 2]]
        edge = np.roll(xy, -1, axis=1) - xy
        cross = edge[:, :, 0] * np.roll(edge[:, :, 1], -1, axis=1)
        cross -= edge[:, :, 1] * np.roll(edge[:, :, 0], -1, axis=1)
        normals = np.asarray([np.cross(q[:, b] - q[:, a], q[:, c] - q[:, a])
                              for a, b, c in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0))])
        lengths = np.linalg.norm(normals, axis=2)
        coherence = np.asarray([np.sum(normals[a] * normals[b], axis=1)
                                / (lengths[a] * lengths[b]) for a, b in ((0, 1), (2, 3))])
        angles, ratios = orbital_quad_quality(points, quads)
        # Tiny reserves accommodate final float32 storage, not looser quality limits.
        return np.concatenate(((cross / area[:, None] - .01).ravel(),
            (coherence - .1001).ravel(),
            (old_ports + .5 - orbital_turns(points, ports)) / 10,
            (angles - old_angles.min() - .005).ravel() / 10,
            (old_angles.max() - .005 - angles).ravel() / 10,
            old_ratios.max() * (1 - 1e-4) - ratios))

    result = minimize(objective, np.zeros(len(moved) * 2), method='SLSQP',
                      bounds=[(-4., 4.)] * (len(moved) * 2),
                      constraints=[{'type': 'ineq', 'fun': constraints}],
                      options={'maxiter': 300, 'ftol': 1e-10, 'eps': max(1e-4, 1e-7 / scale)})
    solved = sample(result.x)
    if solved is None or not result.success or min(constraints(result.x)) < -1e-5:
        raise ValueError('Frozen-source orbital solve failed: ' + str(result.message) + '; minimum constraint=' + str(float(min(constraints(result.x)))))
    points, lineage = solved
    stored = points.astype(np.float32).astype(float)
    if np.max(orbital_turns(stored, support)) >= np.max(old_turns):
        raise ValueError('Orbital redistribution did not reduce the actual support corner')
    # Validate the actual stored geometry, not only the float64 solve proxy.
    if np.max(orbital_turns(stored, ports) - old_ports) > .50001:
        raise ValueError('Stored orbital transverse interface regressed')
    sq = stored[quads]
    se = np.roll(sq[:, :, [0, 2]], -1, axis=1) - sq[:, :, [0, 2]]
    sc = se[:, :, 0] * np.roll(se[:, :, 1], -1, axis=1)
    sc -= se[:, :, 1] * np.roll(se[:, :, 0], -1, axis=1)
    if np.any(sc <= 1e-12):
        raise ValueError('Stored orbital quad chart is not strictly convex')
    sn = np.asarray([np.cross(sq[:, b] - sq[:, a], sq[:, c] - sq[:, a])
                     for a, b, c in ((0, 1, 2), (0, 2, 3), (1, 2, 3), (1, 3, 0))])
    sl = np.linalg.norm(sn, axis=2)
    if np.any(sl <= 1e-12) or any(np.any(np.sum(sn[a] * sn[b], axis=1)
        / (sl[a] * sl[b]) <= .1) for a, b in ((0, 1), (2, 3))):
        raise ValueError('Stored orbital quad diagonal folds')
    for k, (a, b) in enumerate(edge_list):
        for c, d in edge_list[k + 1:]:
            if len({a, b, c, d}) == 4 and segment_distance(
                    stored[a, [0, 2]], stored[b, [0, 2]],
                    stored[c, [0, 2]], stored[d, [0, 2]]) <= 1e-10:
                raise ValueError('Stored orbital chart edges cross or touch')
    final_angles, final_ratios = orbital_quad_quality(stored, quads)
    if (final_angles.min() < old_angles.min() - 1e-4
            or final_angles.max() > old_angles.max() + 1e-4
            or final_ratios.max() > old_ratios.max() + 1e-5):
        raise ValueError('Stored orbital quality exceeds current-source envelope')
    if not np.array_equal(stored[list(boundary)], old.astype(np.float32)[list(boundary)]):
        raise ValueError('Stored orbital boundary changed')
    return {'new_vertices': points.tolist(), 'lineage': lineage,
            'source_face_ids': selected, 'replacement_faces': quads.tolist(),
            'mov': moved, 'boundary_pins': sorted(boundary), 'chain': path,
            'ports': ports.tolist(), 'ports_before': old_ports.tolist(),
            'ports_after': orbital_turns(stored, ports).tolist(),
            'turns_before': old_turns.tolist(),
            'turns_after': orbital_turns(stored, support).tolist(),
            'quality_before': [float(old_angles.min()), float(old_angles.max()), float(old_ratios.max())],
            'quality_after': [float(final_angles.min()), float(final_angles.max()), float(final_ratios.max())],
            'source_relative_scale_m': scale,
            'solver': {'iterations': int(result.nit), 'nfev': int(result.nfev),
                       'objective': float(result.fun), 'minimum_constraint': float(min(constraints(result.x)))},
            'scope': 'Smoother same-support edge curvature; all five extraordinary vertices and all topology retained. Native field/export/pose/visual acceptance required.'}


def orbital_zero_blink_guard(original, proposed, moved, calibrated_delta):
    """Mandatory pre-commit guard for a stage before position-derived blink.

    calibrated_delta receives a native-coordinate point and must evaluate the
    current generator's skin closure field with its own explicit frame binding.
    Frozen donor blink deltas are NOT a substitute for this generation policy.
    """
    old = np.asarray(original, dtype=np.float32).astype(float)
    new = np.asarray(proposed, dtype=np.float32).astype(float)
    rows = []
    for index in moved:
        before, after = float(calibrated_delta(old[index])), float(calibrated_delta(new[index]))
        if not np.isfinite([before, after]).all() or before != 0 or after != 0:
            raise ValueError('Orbital station crosses current analytic blink support')
        rows.append({'vertex': int(index), 'old_delta': before, 'new_delta': after})
    return {'all_moved_stations_generated_zero': True, 'stations': rows}

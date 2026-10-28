"""Semantic adjacency binding for the medial infraorbital source strip."""
from collections import Counter, defaultdict, deque
import numpy as np
from orbital_support_rule import orbital_regular_triples


def orbital_bind_strip(vertices, faces, regions, aperture_ids, protected_ids):
    vertices = np.asarray(vertices, dtype=float)
    faces = [list(map(int, f)) for f in faces]
    aperture = {int(i) for i in aperture_ids if vertices[i, 0] > 0}
    if len(aperture) < 8:
        raise ValueError('Missing named current positive-eye aperture')
    skin, incident = defaultdict(set), defaultdict(list)
    for fi, (face, region) in enumerate(zip(faces, regions)):
        for i in face:
            incident[i].append(fi)
        if region == 1:
            for a, b in zip(face, face[1:] + face[:1]):
                skin[a].add(b)
                skin[b].add(a)
    distance = {i: 0 for i in aperture}
    queue = deque(aperture)
    while queue:
        a = queue.popleft()
        for b in skin[a]:
            if b not in distance:
                distance[b] = distance[a] + 1
                queue.append(b)
    center = vertices[list(aperture)].mean(axis=0)

    def unique(candidates, role):
        if len(candidates) != 1:
            raise ValueError('Ambiguous current orbital role: ' + role)
        return next(iter(candidates))

    b = unique({i for i in skin if distance.get(i) == 2 and len(skin[i]) == 6
                and 0 < vertices[i, 0] < center[0] and vertices[i, 2] < center[2]},
               'medial second-support six-pole')
    a = unique({i for i in skin[b] if distance.get(i) == 2 and len(skin[i]) == 5
                and vertices[i, 0] > vertices[b, 0] and vertices[i, 2] < vertices[b, 2]},
               'lower second-support five-pole')
    e = unique({i for i in skin[a] if len(skin[i]) == 3}, 'lower bridge')
    d = unique({i for i in skin[b] & skin[e] if len(skin[i]) == 3}, 'upper bridge')
    c = unique({i for i in skin[b] if len(skin[i]) == 3 and i != d}, 'medial transition')
    entry = unique({i for i in skin[b] if distance.get(i) == 2 and i != a}, 'support entry')
    exit_ = unique({i for i in skin[a] if distance.get(i) == 2 and i != b}, 'support exit')
    movable = [a, b, c, d, e]
    if len(set(movable)) != 5 or set(movable) & set(protected_ids):
        raise ValueError('Orbital role binding crosses protected functional source')
    triples, full_neighbors = orbital_regular_triples(vertices, faces, set(range(len(vertices))))
    opposite = {}
    for x, y, z in triples:
        opposite[x, y] = z
        opposite[z, y] = x
    if (b, entry) not in opposite:
        raise ValueError('Orbital entry has no regular exterior continuation')
    path = [opposite[b, entry], entry, b, a, exit_]
    while (path[-2], path[-1]) in opposite:
        nxt = opposite[path[-2], path[-1]]
        if nxt in path or len(path) >= 16:
            raise ValueError('Orbital support continuation is cyclic or unbounded')
        path.append(nxt)
    if len(path) < 6:
        raise ValueError('Incomplete lateral support continuation')
    selected = {fi for i in movable for fi in incident[i]}
    rank = {i: k for k, i in enumerate(sorted(range(len(vertices)),
             key=lambda i: tuple(vertices[i, [0, 2, 1]])))}
    selected = sorted(selected, key=lambda fi: tuple(sorted(rank[i] for i in faces[fi])))
    if len(selected) != 11 or any(regions[i] != 1 or len(faces[i]) != 4 for i in selected):
        raise ValueError('Unexpected full eleven-quad orbital star')
    names = ['second_lower_turn', 'second_medial_turn', 'third_medial_transition',
             'third_upper_bridge', 'third_lower_bridge']
    roles = dict(zip(names, movable))
    support_names = []
    for k, i in enumerate(path):
        name = 'support_station_' + str(k)
        roles[name] = i
        support_names.append(name)
    return {'role_to_index': roles, 'movable_roles': names,
            'support_roles': support_names, 'selected_faces': selected}

"""Resolve only an unchanged, generator-attested posterior return strip."""
from collections import Counter, defaultdict
import numpy as np


def verify_nasal_return_provenance(vertices, faces, regions, record, required_pairs):
    if record.get('open_back') is not True:
        raise ValueError('Posterior return was not generated as an open strip')
    proof = record.get('generated_strip_provenance', {})
    if (proof.get('schema_version') != 1 or proof.get('coordinate_frame') !=
            'input Mesh local; unchanged coordinates required at consumption'):
        raise ValueError('Missing explicit generated return coordinate provenance')
    points = np.asarray(vertices, dtype=float)
    if not np.isfinite(points).all():
        raise ValueError('Nonfinite current return coordinates')
    lookup = defaultdict(list)
    for i, point in enumerate(points):
        lookup[tuple(point)].append(i)
    def resolve(point):
        key = tuple(point)
        if len(key) != 3 or len(lookup.get(key, [])) != 1:
            raise ValueError('Generated return point moved or is ambiguous')
        return lookup[key][0]
    def cycle(ids):
        ids = tuple(ids)
        return min(ids[k:] + ids[:k] for k in range(len(ids)))
    current = {cycle(face): i for i, face in enumerate(faces)}
    if len(current) != len(faces):
        raise ValueError('Duplicate oriented current source face')
    incidence = Counter(tuple(sorted((a, b))) for face in faces
                        for a, b in zip(face, face[1:] + face[:1]))
    certified = []
    for parent, rim in required_pairs:
        matches = [pair for pair in proof.get('parent_rim_pairs', [])
                   if tuple(pair['rim']) == tuple(points[rim])]
        if len(matches) != 1 or resolve(matches[0]['parent']) != parent or resolve(matches[0]['rim']) != rim:
            raise ValueError('No unique generated parent and rim correspondence')
        support = []
        for row in proof.get('support_cycles', []):
            if tuple(points[rim]) not in map(tuple, row['positions']):
                continue
            ids = tuple(resolve(point) for point in row['positions'])
            key = cycle(ids)
            if key not in current or int(regions[current[key]]) != row['region']:
                raise ValueError('Generated return support was retopologized or changed owner')
            support.append(current[key])
        actual = {i for i, face in enumerate(faces) if rim in face}
        if not support or len(support) != len(set(support)) or set(support) != actual:
            raise ValueError('Generated return support cardinality changed')
        expected_edges = set()
        for edge in proof.get('boundary_edges', []):
            if tuple(points[rim]) in map(tuple, edge):
                expected_edges.add(tuple(sorted(resolve(point) for point in edge)))
        actual_edges = {edge for edge, count in incidence.items() if count == 1 and rim in edge}
        if len(expected_edges) != 2 or actual_edges != expected_edges:
            raise ValueError('Generated open rim incidence changed')
        certified.append({'parent': int(parent), 'rim': int(rim),
                          'support_cycles': [list(faces[i]) for i in sorted(actual)],
                          'boundary_edges': [list(edge) for edge in sorted(actual_edges)]})
    if len(certified) != 2 or len({row['rim'] for row in certified}) != 2:
        raise ValueError('Two distinct generated posterior rim stations required')
    return {'coordinate_match': 'unique exact native coordinates', 'regions_and_cycles_exact': True,
            'existing_boundary_incidence_exact': True, 'pairs': certified}

"""Source-chart guide fairing and support domains for the refined quad patch."""
from collections import defaultdict
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from forehead_flow_rule import forehead_chart


def fair_forehead_guides(vertices, faces, regions, topology, features):
    old = np.asarray(vertices, float)
    source_faces = [list(map(int, q)) for q in faces]
    points = np.asarray(topology['vertices'], float)
    chart = np.asarray(topology['chart'], float)
    paths = list(topology['paths'].values())
    guide = {i for p in paths for i in p}
    star_old = {i for j in topology['retiled_old_faces'] for i in source_faces[j]}
    protected = set(topology['protected'])
    moving = ({i for p in paths for i in p[1:-1]} |
              set(range(len(old), len(points))) | star_old)-protected
    supports = moving-guide
    final_faces = topology['faces']
    neighbors = defaultdict(set)
    for q in final_faces:
        for a, b in zip(q, q[1:]+q[:1]):
            neighbors[a].add(b)
            neighbors[b].add(a)
    old_ids = sorted(set(topology['retiled_old_faces']) |
                     {j for j, q in enumerate(source_faces) if moving.intersection(q)})
    new_ids = [j for j, q in enumerate(final_faces) if moving.intersection(q)
               or topology['face_owners'][j] in topology['retiled_old_faces']]
    donor_points = {i for j in old_ids for i in source_faces[j]}
    donor_faces = [q for q, owner in zip(source_faces, regions)
                   if owner == 1 and len(q) == 4 and donor_points.intersection(q)
                   and min(old[q, 0]) >= -1e-8]
    source_chart = forehead_chart(old)
    surface = AuthorQuadSurface(source_chart, donor_faces, [1]*len(donor_faces))
    surface.vertices = old.copy()
    target = chart.copy()
    for iteration in range(8):
        previous = target.copy()
        for row, path in enumerate(paths):
            if row < 2 and iteration >= 2:
                continue
            for a, b, c in zip(path, path[1:], path[2:]):
                if b not in moving:
                    continue
                left = np.linalg.norm(previous[b, [0, 2]]-previous[a, [0, 2]])
                right = np.linalg.norm(previous[c, [0, 2]]-previous[b, [0, 2]])
                if min(left, right) <= 1e-9:
                    raise ValueError('Collapsed refined forehead guide station')
                midpoint = (previous[a]*right+previous[c]*left)/(left+right)
                target[b] += (.2 if row < 2 else .4)*(midpoint-previous[b])
        for _ in range(3):
            previous = target.copy()
            for i in supports:
                target[i] += .35*(np.mean(previous[list(neighbors[i])], axis=0)-previous[i])
    # Blend the relocation through the current oral frame rather than leaving
    # a hard frozen-height junction. The posed transport guard remains separate.
    mouth_height = float(old[list(features['oral']), 2].mean())
    fade = np.clip((mouth_height+.02-chart[:, 2])/.03, 0, 1)
    fade = fade*fade*(3-2*fade)
    target = chart+(target-chart)*(1-.7*fade[:, None])
    lineage = {}
    for i in sorted(moving):
        donors, weights, point = surface.lineage(target[i, 0], target[i, 2])
        points[i] = np.asarray(point, np.float32)
        lineage[i] = {'donors': list(donors), 'weights': list(weights)}
    if not np.array_equal(points[sorted(protected)], old[sorted(protected)]):
        raise ValueError('Guide fairing changed an original functional boundary')
    return {'vertices': points, 'faces': final_faces, 'lineage': lineage,
            'affected_old': old_ids, 'affected_new': new_ids,
            'moving': sorted(moving), 'support': sorted(supports),
            'source_faces': donor_faces, 'source_chart': source_chart,
            'paths': topology['paths'], 'protected': sorted(protected), 'mouth_height': mouth_height,
            'topology': topology}

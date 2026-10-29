"""Current-graph roles for the five forehead, temple and jaw flow corridors."""
from collections import defaultdict
import numpy as np
from forehead_flow_bind import bind_forehead_paths
from forehead_star_topology import (ForeheadStarTopology, forehead_edge_counts,
                                   forehead_guide_corners)


def forehead_interior_ear_route(vertices, faces, regions, binding):
    """Approach the upper ear through skin, without following the open rim.

    An interior rail cannot become tangent to a pinned boundary and continue
    along it without flattening intervening quads. The terminal station is the
    existing interior upper-ear support, whose position remains fixed.
    """
    vertices = np.asarray(vertices)
    adjacent, shared = defaultdict(set), defaultdict(set)
    for q, owner in zip(faces, regions):
        if owner != 1:
            continue
        for i in q:
            shared[i].update(q)
        for a, b in zip(q, q[1:]+q[:1]):
            adjacent[a].add(b)
            adjacent[b].add(a)
    boundary = {i for edge, count in forehead_edge_counts(faces).items()
                if count == 1 for i in edge}

    def one(values, label):
        values = list(values)
        if len(values) != 1:
            raise ValueError('Ambiguous current forehead '+label)
        return values[0]

    path = binding['positive_paths']['green']
    if len(path) != 19 or path[14] not in boundary or path[13] in boundary:
        raise ValueError('Unsupported upper-ear rim role layout')
    result = path[:14]
    result.append(one((i for i in adjacent[result[-1]] if i not in boundary
                       and i != result[-2] and vertices[i, 2] < vertices[result[-1], 2]-1e-7),
                      'interior descending station'))
    result.append(one(adjacent[result[-1]]-shared[result[-2]], 'interior continuation'))
    result.append(one(adjacent[result[-1]] & set(binding['ear_root_star']),
                      'upper-ear interior support'))
    if result[-1] in boundary or any(i in boundary for i in result[1:]):
        raise ValueError('Forehead ear approach unexpectedly follows an open rim')
    return result


def prepare_forehead_star_topology(vertices, faces, regions, report, features):
    """Bind authored semantic paths, then remove their shared-quad elbows.

    This is the current raw13 source recipe, with fail-closed topology checks.
    It uses no saved vertex IDs, historical artifacts or reference coordinates.
    """
    faces = [list(map(int, q)) for q in faces]
    binding = bind_forehead_paths(vertices, faces, regions, report)
    paths = {name: list(p) for name, p in binding['positive_paths'].items()}
    paths['green'] = forehead_interior_ear_route(vertices, faces, regions, binding)
    corners = {name: forehead_guide_corners(faces, path) for name, path in paths.items()}
    if ([len(corners[name]) for name in ('red', 'orange', 'green', 'blue', 'purple')]
            != [0, 0, 1, 1, 2]):
        raise ValueError('Unrecognized five-flow corner topology')
    upper, lower = corners['purple']
    blue, green = corners['blue'][0], corners['green'][0]
    if any(i in features['protected'] for i in
           (upper['vertex'], blue['vertex'], green['vertex'], lower['opposite'], lower['vertex'])):
        raise ValueError('Forehead pivot is a functional feature boundary')
    builder = ForeheadStarTopology(vertices, faces, regions, paths)
    centers = builder.refine_star(upper['vertex'])
    builder.refine_star(centers[upper['face']])
    builder.refine_star(blue['vertex'])
    # The outer turn needs support room beside the short, fixed rim edges.
    builder.refine_star(green['vertex'])
    builder.refine_star(lower['opposite'])
    result = builder.finish()
    boundary = {i for edge, count in builder.original_edges.items() if count == 1 for i in edge}
    result['protected'] = sorted(set(features['protected']) | boundary |
                                 set(binding['ear_root_star']) |
                                 {i for path in result['paths'].values() for i in (path[0], path[-1])})
    result['roles'] = {'original_binding': binding,
                       'original_corner_roles': corners,
                       'green_endpoint_policy': 'Pinned existing interior upper-ear support; not an open-rim continuation',
                       'semantic_recipe': 'Current raw13 graph; conforming closed-star quad refinement'}
    return result

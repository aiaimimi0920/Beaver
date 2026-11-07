"""Reconnect a compressed guide endpoint without moving the original open rim."""
from collections import defaultdict, deque
from copy import deepcopy
from itertools import combinations
import heapq
import numpy as np
from forehead_star_topology import forehead_guide_corners


def forehead_boundary_chain(edges, start, target):
    adjacent = defaultdict(set)
    for first, second in edges:
        adjacent[first].add(second)
        adjacent[second].add(first)
    queue, previous = deque([start]), {start: None}
    while queue and target not in previous:
        point = queue.popleft()
        for neighbor in sorted(adjacent[point]):
            if neighbor not in previous:
                previous[neighbor] = point
                queue.append(neighbor)
    if target not in previous:
        raise ValueError('Guide target is on a different open boundary')
    chain = [target]
    while chain[-1] != start:
        chain.append(previous[chain[-1]])
    return list(reversed(chain))


def forehead_boundary_route_path(builder, name, target, protected, boundary, station_cost=.3):
    points = np.asarray(builder.points, float)
    paths = builder.paths
    start = paths[name][0]
    blocked = set(protected) | set(boundary)
    blocked |= {i for other, path in paths.items() if other != name for i in path}
    blocked -= {start, target}
    adjacent, shared = defaultdict(set), defaultdict(set)
    for face, owner in zip(builder.faces, builder.owners):
        if builder.regions[owner] != 1:
            continue
        for point in face:
            shared[point].update(face)
        for first, second in zip(face, face[1:] + face[:1]):
            adjacent[first].add(second)
            adjacent[second].add(first)
    angle = np.arctan2(points[:, 0], -points[:, 1])
    reference = points[paths[name]]
    reference_angle = angle[paths[name]]
    if np.any(np.diff(reference_angle) <= 0) or angle[target] <= 0:
        raise ValueError('Unrecognized upper-guide sweep for boundary routing')
    scale = np.linalg.norm(points[paths['red'][-1]] - points[paths['green'][-1]]) / 2
    if scale <= 1e-9:
        raise ValueError('Collapsed boundary routing domain')
    queue, distance, previous = [(0., -1, start)], {(-1, start): 0.}, {}
    finish = None
    while queue:
        cost, prior, point = heapq.heappop(queue)
        state = (prior, point)
        if cost != distance[state]:
            continue
        if point == target:
            finish = state
            break
        for neighbor in sorted(adjacent[point] - blocked):
            if neighbor == prior or points[neighbor, 0] < -1e-8:
                continue
            fraction = np.clip(angle[neighbor] / angle[target], 0, 1)
            ideal = np.interp(angle[neighbor], reference_angle, reference[:, 2])
            ideal += (points[target, 2] - reference[-1, 2]) * fraction ** 3
            length = np.linalg.norm(points[neighbor] - points[point])
            corner = prior != -1 and neighbor in shared[prior]
            score = cost + length * (1 + ((points[neighbor, 2] - ideal) / (scale / 2)) ** 2)
            score += scale * (corner + station_cost) + 5 * scale * max(angle[point] - angle[neighbor], 0)
            following = (point, neighbor)
            if score < distance.get(following, float('inf')):
                distance[following] = score
                previous[following] = state
                heapq.heappush(queue, (score, point, neighbor))
    if finish is None:
        raise ValueError('No disjoint skin-edge route through the boundary corridor')
    route, state = [], finish
    while state in previous:
        route.append(state[1])
        state = previous[state]
    route.append(start)
    route.reverse()
    if len(route) != len(set(route)):
        raise ValueError('Boundary guide route contains a cycle')
    return route


def forehead_remove_route_elbows(builder, name, protected):
    for _ in range(24):
        corners = forehead_guide_corners(builder.faces, builder.paths[name])
        if not corners:
            return
        corner = corners[0]
        if corner['opposite'] not in protected:
            builder.refine_star(corner['opposite'])
        elif corner['vertex'] not in protected:
            # Split this closed star first. The remaining elbow then has a new,
            # interior opposite support instead of the immutable rim vertex.
            builder.refine_star(corner['vertex'])
        else:
            raise ValueError('Guide elbow cannot be removed without changing a protected star')
    raise ValueError('Boundary guide elbow refinement did not terminate within its budget')


def forehead_rebind_boundary_guide(builder, name, target, functional_protected, ear_root_star):
    points = np.asarray(builder.points, float)
    previous_endpoint = builder.paths[name][-1]
    edges = [edge for edge, count in builder.original_edges.items() if count == 1]
    boundary = {i for edge in edges for i in edge}
    if previous_endpoint == target:
        return {'changed': False, 'name': name, 'endpoint': target}
    chain = forehead_boundary_chain(edges, previous_endpoint, target)
    if len(chain) > 12:
        raise ValueError('Boundary routing would exceed the local corridor')
    selected = {tuple(sorted(edge)) for edge in zip(chain, chain[1:])}
    strip = [i for i, face in enumerate(builder.faces)
             if any(tuple(sorted(edge)) in selected for edge in zip(face, face[1:] + face[:1]))]
    if any(builder.regions[builder.owners[i]] != 1 for i in strip):
        raise ValueError('Boundary corridor crosses non-skin geometry')
    protected = set(functional_protected) | boundary | set(ear_root_star)
    pivots = sorted({v for i in strip for v in builder.faces[i]} - protected)
    if len(pivots) > 8:
        raise ValueError('Boundary support candidate set exceeds the local search budget')
    choices = []
    for count in range(len(pivots) + 1):
        for subset in combinations(pivots, count):
            trial = deepcopy(builder)
            try:
                for pivot in subset:
                    trial.refine_star(pivot)
                trial.paths[name] = forehead_boundary_route_path(trial, name, target, protected, boundary)
                forehead_remove_route_elbows(trial, name, protected)
                trial.finish()
            except ValueError:
                continue
            length = np.linalg.norm(np.diff(np.asarray(trial.points)[trial.paths[name]], axis=0), axis=1).sum()
            choices.append((len(trial.faces), float(length), len(trial.paths[name]), subset, trial))
        if choices:
            break
    if not choices:
        raise ValueError('No bounded conforming support corridor admits the guide')
    _, _, _, chosen_pivots, chosen = min(choices, key=lambda row: row[:3])
    builder.points, builder.chart = chosen.points, chosen.chart
    builder.faces, builder.owners = chosen.faces, chosen.owners
    builder.paths, builder.lineage = chosen.paths, chosen.lineage
    builder.operations, builder.surfaces = chosen.operations, chosen.surfaces
    builder.edge_parent = chosen.edge_parent
    if not np.array_equal(np.asarray(builder.points)[list(boundary)], points[list(boundary)]):
        raise ValueError('Boundary routing moved the original open rim')
    return {'changed': True, 'name': name, 'previous_endpoint': previous_endpoint, 'endpoint': target,
            'corridor_boundary': chain, 'support_pivots': list(chosen_pivots),
            'boundary_coordinates_preserved': True,
            'scope': 'Current graph and fitted ear geometry; no saved vertex identifiers',
            'acceptance': 'Routing only; quad quality, fields, poses and appearance still required'}


def reroute_forehead_boundary_guides(builder, functional_protected, ear_root_star):
    """Use Faye's own frontal guide gaps, never reference coordinates or saved IDs."""
    points = np.asarray(builder.points, float)
    paths = builder.paths
    red, orange, green = [paths[name][-1] for name in ('red', 'orange', 'green')]
    boundary = {i for edge, count in builder.original_edges.items() if count == 1 for i in edge}
    if red not in boundary or orange not in boundary:
        raise ValueError('Expected original open-rim upper guide endpoints')
    desired = np.array([np.linalg.norm(points[paths[a][0]]-points[paths[b][0]])
                        for a, b in (('red','orange'),('orange','green'))])
    if desired.min() <= 1e-9:
        raise ValueError('Collapsed frontal guide spacing')
    def cost(first, second):
        gaps = np.array([np.linalg.norm(points[first]-points[second]),
                         np.linalg.norm(points[second]-points[green])])
        movement = np.array([np.linalg.norm(points[first]-points[red]),
                             np.linalg.norm(points[second]-points[orange])])
        return float(np.sum((gaps/desired-1)**2)+.01*np.sum((movement/desired)**2))
    candidates = [i for i in boundary if points[i,0]>0 and points[i,2]>points[green,2]
                  and i not in functional_protected]
    ranked = sorted((cost(a,b),a,b) for a in candidates for b in candidates
                    if points[a,2]>points[b,2])
    if not ranked:
        raise ValueError('No ordered boundary targets for upper guides')
    if cost(red,orange) <= .05 or ranked[0][0] >= cost(red,orange):
        return {'changed': False, 'reason': 'Existing endpoint gaps need no boundary reassignment'}
    if len(ranked)>1 and abs(ranked[0][0]-ranked[1][0])<1e-10:
        raise ValueError('Ambiguous boundary guide assignment')
    _, red_target, orange_target = ranked[0]
    # Move the lower route first so it does not fence off the upper route's target.
    lower = forehead_rebind_boundary_guide(builder,'orange',orange_target,functional_protected,ear_root_star)
    upper = forehead_rebind_boundary_guide(builder,'red',red_target,functional_protected,ear_root_star)
    return {'changed': True, 'red': upper, 'orange': lower,
            'frontal_gap_targets_mm': (desired*1000).tolist(),
            'boundary_coordinates_preserved': True,
            'acceptance': 'Routing only; fitting and native mesh/field/pose/visual checks remain'}

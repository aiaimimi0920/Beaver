"""Semantic landmarks and a single-pass nasal/cheek source replacement domain."""
from collections import Counter, defaultdict
import numpy as np
from nasal_source_geometry import (canonical_boundary, clipped_polygon, edge_incidence,
                                   patch_boundary, polygon_area, segment_distance)


def connected_components(adjacency):
    remaining = set(adjacency)
    result = []
    while remaining:
        first = remaining.pop()
        found,stack = {first},[first]
        while stack:
            for neighbor in adjacency[stack.pop()]:
                if neighbor not in found:
                    found.add(neighbor); remaining.discard(neighbor); stack.append(neighbor)
        result.append(found)
    return result


def current_source_features(vertices, faces, regions, ordinals):
    """Use live source role names and topology, not historical file indices."""
    vertices = np.asarray(vertices,dtype=float)
    faces = [tuple(map(int,f)) for f in faces]
    membership = [set() for _ in vertices]
    skin_neighbors = defaultdict(set)
    skin_edges = Counter()
    for face,region in zip(faces,regions):
        for i in face:
            membership[i].add(int(region))
        if region == 1:
            for a,b in zip(face,face[1:]+face[:1]):
                skin_neighbors[a].add(b); skin_neighbors[b].add(a)
                skin_edges[tuple(sorted((a,b)))] += 1
    centers = []
    for side in (-1,1):
        name = f'Continuous shallow iris disk {side}'
        if name not in ordinals:
            raise ValueError('Nasal flow requires a named current iris source')
        incidence = Counter(i for face,r in zip(faces,regions) if r == ordinals[name] for i in face)
        if len(incidence) < 5:
            raise ValueError('Incomplete current iris source')
        ordered = incidence.most_common()
        if ordered[0][1] <= 2*ordered[1][1]:
            raise ValueError('Iris fan center is not unique')
        index = ordered[0][0]
        if side*vertices[index,0] <= 0:
            raise ValueError('Iris source side does not match its role')
        centers.append(vertices[index].copy())
    eye_height = float(np.mean(centers,axis=0)[2])
    boundary_neighbors = defaultdict(set)
    for (a,b),count in skin_edges.items():
        if count == 1:
            boundary_neighbors[a].add(b); boundary_neighbors[b].add(a)
    components = connected_components(boundary_neighbors)
    eyes,first_lids = set(),set()
    for center in centers:
        hits = [part for part in components if 8 <= len(part) <= 128
                and np.linalg.norm(vertices[list(part)][:,[0,2]].mean(axis=0)-center[[0,2]]) < .015]
        if len(hits) != 1 or any(len(boundary_neighbors[i]) != 2 for i in hits[0]):
            raise ValueError('Eye aperture has no unique current source loop')
        aperture = hits[0]
        eyes |= aperture
        first_lids |= set().union(*(skin_neighbors[i] for i in aperture))-aperture
    oral_roles = [value for name,value in ordinals.items()
                  if name.startswith('Oral cavity inner lips and back wall')]
    if len(oral_roles) != 1:
        raise ValueError('Missing unique oral source role')
    oral = {i for i,roles in enumerate(membership) if {1,oral_roles[0]} <= roles}
    if len(oral) < 4:
        raise ValueError('No shared source oral aperture')
    first_lips = set().union(*(skin_neighbors[i] for i in oral))-oral
    axis = {i for i,roles in enumerate(membership) if 1 in roles and abs(vertices[i,0]) < 1e-8}
    mouth_height = float(vertices[list(oral),2].mean())
    nose_candidates = [i for i in axis if mouth_height+.005 < vertices[i,2] < eye_height-.005]
    nose_candidates.sort(key=lambda i:vertices[i,1])
    if len(nose_candidates) < 2 or vertices[nose_candidates[1],1]-vertices[nose_candidates[0],1] < 1e-5:
        raise ValueError('Current source nasal peak is ambiguous')
    nose = vertices[nose_candidates[0]].copy()
    attachments = {i for i,roles in enumerate(membership)
                   if 1 in roles and any(r not in (1,2) for r in roles)}
    protected = eyes|first_lids|oral|first_lips|attachments|axis
    return {'eye_centers':centers,'eye_height':eye_height,'nose_peak':nose,
            'apertures':eyes,'first_lids':first_lids,'oral':oral,'first_lips':first_lips,
            'axis':axis,'attachments':attachments,'protected':protected,
            'skin_neighbors':skin_neighbors,'membership':membership}


def resolve_outer_anchors(vertices, regions, faces, authored, assembly_offset):
    """Bind current build-face semantic samples through the explicit assembly."""
    vertices = np.asarray(vertices,dtype=float)
    skin = {i for f,r in zip(faces,regions) if r == 1 for i in f}
    expected = [(1.614,.87),(1.608,.87),(1.6045,.87),(1.599,.87),(1.590,.87)]
    if [(row['height'],row['lateral']) for row in authored] != expected:
        raise ValueError('Nasal guide source station contract changed')
    resolved = []
    for row in authored:
        point = np.asarray(row['position'],dtype=np.float32).astype(float)
        point[2] = np.float32(point[2]+assembly_offset)
        distances = np.linalg.norm(vertices-point,axis=1)
        hits = [i for i in np.flatnonzero(distances <= 2e-6) if i in skin]
        if len(hits) != 1:
            raise ValueError('Nasal source anchor is missing or ambiguous')
        index = hits[0]
        second = min(float(distances[i]) for i in skin if i != index)
        if second < .0005 or vertices[index,0] <= 0:
            raise ValueError('Nasal anchor has inadequate semantic separation')
        resolved.append(index)
    if len(set(resolved)) != 5:
        raise ValueError('Nasal source guide anchors collapsed')
    return resolved


def select_source_domain(vertices, faces, regions, seed_cells, core_boundary, features):
    """One intersection pass, one notch pass, one narrow-gap pass; no tuning."""
    vertices = np.asarray(vertices,dtype=float)
    faces = [tuple(map(int,f)) for f in faces]
    locked = features['protected']
    bounds_low = np.asarray(seed_cells).reshape(-1,2).min(axis=0)
    bounds_high = np.asarray(seed_cells).reshape(-1,2).max(axis=0)
    seed = set()
    for index,(face,region) in enumerate(zip(faces,regions)):
        if region != 1 or len(face) != 4 or max(vertices[list(face),0]) <= 0:
            continue
        xy = vertices[list(face)][:,[0,2]]
        if np.any(xy.max(axis=0) < bounds_low) or np.any(xy.min(axis=0) > bounds_high):
            continue
        if sum(polygon_area(clipped_polygon(xy,cell)) for cell in seed_cells) > 1e-12:
            seed.add(index)
    if not seed:
        raise ValueError('Empty nasal source intersection domain')
    incidence = edge_incidence(faces)
    def eligible(index):
        face = faces[index]
        return (regions[index] == 1 and len(face) == 4
                and min(vertices[list(face),0]) > 1e-9 and not set(face)&locked)
    boundary = patch_boundary(faces,seed)
    notches = set()
    for index,face in enumerate(faces):
        if index in seed or not eligible(index):
            continue
        edges = [tuple(sorted((a,b))) for a,b in zip(face,face[1:]+face[:1])]
        hits = [k for k,e in enumerate(edges) if e in boundary]
        if len(hits) == 2 and (hits[0]-hits[1])%4 in (1,3):
            notches.add(index)
    selected = seed|notches
    boundary = patch_boundary(faces,selected)
    narrow,triggers = set(),[]
    inner = np.asarray(core_boundary,dtype=float)[:,[0,2]]
    for edge in boundary:
        a,b = vertices[list(edge)][:,[0,2]]
        distance = min(segment_distance(a,b,c,d) for c,d in zip(inner,np.roll(inner,-1,axis=0)))
        if distance >= .0015:
            continue
        adjacent = [i for i in incidence[edge] if i not in selected and eligible(i)]
        if len(adjacent) > 1:
            raise ValueError('Ambiguous narrow-domain extension')
        if adjacent:
            narrow.add(adjacent[0]); triggers.append(float(distance))
    selected |= narrow
    cycle = canonical_boundary(vertices,faces,selected)
    used = {i for j in selected for i in faces[j]}
    if (used-set(cycle))&locked or (used&locked)-features['axis']:
        raise ValueError('Nasal domain touches a functional protected feature')
    return selected,cycle,{'seed_faces':len(seed),'notch_faces':len(notches),
                           'narrow_faces':len(narrow),'final_faces':len(selected),
                           'boundary_count':len(cycle),'narrow_gaps_m':sorted(triggers)}

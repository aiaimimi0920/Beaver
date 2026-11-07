"""Whole-guide support neighborhoods without an arbitrary frozen jaw height."""
from collections import defaultdict
import numpy as np
from forehead_flow_rule import forehead_chart
from orbital_support_rule import orbital_quad_quality


def forehead_chart_cell_orientation(chart, face):
    quad = chart[np.asarray(face, int)][:, [0, 2]]
    edge = np.roll(quad, -1, axis=0) - quad
    cross = edge[:, 0] * np.roll(edge[:, 1], -1) - edge[:, 1] * np.roll(edge[:, 0], -1)
    return int(np.sign(cross.sum())) if cross.min() * cross.max() > 1e-18 else 0


def forehead_chart_cell_is_convex(chart, face):
    return forehead_chart_cell_orientation(chart, face) != 0


def close_forehead_quality_domain(points, faces, moving, protected, affected, rounds=8):
    """Free eligible determinants of immutable bad corners in incident cells.

    Moving a quad's opposite corner cannot repair an angle determined by three
    frozen vertices. Recompute incident faces after every expansion; otherwise
    adding one support can expose the same obstruction in its next neighbor.
    """
    moving, original = set(moving), set(moving)
    for _ in range(rounds):
        current = set(affected) | {i for i, face in enumerate(faces) if moving.intersection(face)}
        rows = sorted(current)
        if any(len(faces[i]) != 4 for i in rows):
            raise ValueError('Forehead quality closure reached a non-quad')
        quads = np.asarray([faces[i] for i in rows], int)
        angles, _ = orbital_quad_quality(points, quads)
        required = []
        for quad, values in zip(quads, angles):
            for corner in np.flatnonzero((values < 25.003) | (values > 149.997)):
                triple = {int(quad[(corner-1) % 4]), int(quad[corner]), int(quad[(corner+1) % 4])}
                if not triple.intersection(moving):
                    required.append(triple)
            frozen_edges = [(int(a), int(b)) for a, b in zip(quad, np.roll(quad, -1))
                            if a not in moving and b not in moving]
            if len(frozen_edges) > 1:
                lengths = [np.linalg.norm(points[a]-points[b]) for a, b in frozen_edges]
                if min(lengths) <= 1e-12 or max(lengths)/min(lengths) > 6.998:
                    required.append(set(frozen_edges[int(np.argmin(lengths))])
                                    | set(frozen_edges[int(np.argmax(lengths))]))
        added = set()
        for nodes in required:
            eligible = {i for i in nodes if i not in protected and points[i, 0] > 1e-8}
            if not eligible:
                raise ValueError('A protected fixed corner/edge pair cannot meet forehead quality bounds')
            added.update(eligible)
        if not added:
            return moving, current, sorted(moving-original)
        moving.update(added)
    raise ValueError('Forehead quality support closure exceeded its bounded neighborhood')


def prepare_forehead_fit_domain(old, original_faces, regions, proposal, rings=2):
    if not isinstance(rings, int) or not 0 <= rings <= 2:
        raise ValueError('Forehead support expansion is limited to two rings')
    old = np.asarray(old, float)
    points = np.asarray(proposal['vertices'], float)
    faces = proposal['faces']
    topology = proposal['topology']
    owners = topology['face_owners']
    adjacent = defaultdict(set)
    protected = set(proposal['protected'])
    chart = forehead_chart(points)
    source_chart = forehead_chart(old)
    front_half = [region == 1 and len(face) == 4 and min(old[face, 0]) >= -1e-8
                  for face, region in zip(original_faces, regions)]
    orientations = [forehead_chart_cell_orientation(source_chart, face) if eligible else 0
                    for face, eligible in zip(original_faces, front_half)]
    starts = {path[0] for path in proposal['paths'].values()}
    anchor_signs = {sign for face, sign in zip(original_faces, orientations)
                    if starts.intersection(face) and sign}
    if len(anchor_signs) != 1:
        raise ValueError('Guide source does not identify a unique frontal parameter sheet')
    front_sign = anchor_signs.pop()
    opposite_cells = {i for i, sign in enumerate(orientations) if sign and sign != front_sign}
    frozen_cells = []
    for index, (face, owner) in enumerate(zip(faces, owners)):
        if len(face) != 4 or regions[owner] != 1:
            protected.update(face)
        elif owner in opposite_cells:
            if index in proposal['affected_new']:
                raise ValueError('Initial affected patch crosses another parameter sheet')
            protected.update(face)
        elif not forehead_chart_cell_is_convex(chart, face):
            if index in proposal['affected_new']:
                raise ValueError('Initial affected patch already has a folded chart cell')
            # An unchanged ear/jaw return may not be represented by this chart.
            # Preserve its real geometry rather than moving it through an
            # invalid parameterization or weakening the convexity constraint.
            protected.update(face)
            frozen_cells.append(index)
        for first, second in zip(face, face[1:] + face[:1]):
            adjacent[first].add(second)
            adjacent[second].add(first)
    if any(i >= len(old) or not np.array_equal(points[i], old[i]) for i in protected):
        raise ValueError('Initialization changed a protected/non-skin neighbor')
    guides = {i for path in proposal['paths'].values() for i in path}
    moving = (set(proposal['moving']) | guides | set(range(len(old), len(points)))) - protected
    moving = {i for i in moving if points[i, 0] > 1e-8}
    for _ in range(rings):
        moving |= {j for i in moving for j in adjacent[i]
                   if j not in protected and points[j, 0] > 1e-8}
    moving, new_faces, closure = close_forehead_quality_domain(
        points, faces, moving, protected, proposal['affected_new'])
    old_faces = set(proposal['affected_old'])
    old_faces.update(i for i, face in enumerate(original_faces) if moving.intersection(face))
    donor_faces = [list(face) for face, sign in zip(original_faces, orientations) if sign == front_sign]
    proposal.update(moving=sorted(moving), support=sorted(moving-guides),
                    protected=sorted(protected), affected_old=sorted(old_faces),
                    affected_new=sorted(new_faces), source_faces=donor_faces,
                    source_chart=source_chart)
    proposal['support_domain'] = {
        'rings': rings, 'whole_guide_continuations': True,
        'protected_and_non_skin_neighbors_fixed': True,
        'nonconvex_chart_faces_frozen': frozen_cells,
        'opposite_chart_sheet_faces_frozen': sorted(opposite_cells),
        'quality_closure_added_vertices': closure,
        'posed_field_and_dense_shape_guards_required': True,
    }
    return proposal

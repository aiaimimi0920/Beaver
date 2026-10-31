"""Reflect a current source-retile proposal with verified owner correspondence."""
import numpy as np


def mirror_forehead_retile(vertices, faces, regions, proposal, owner_mirror, old_open_delta):
    old = np.asarray(vertices, float)
    original = [list(map(int, q)) for q in faces]
    positive = np.asarray(proposal['vertices'], float)
    topology = proposal['topology']
    n, old_count = len(old), len(faces)
    owners = {1: 1, **owner_mirror}
    if any(owners.get(b, b) != a for a, b in owners.items()):
        raise ValueError('Forehead mirror owners are not involutive')
    membership = [set() for _ in old]
    for q, region in zip(original, regions):
        for i in q:
            membership[i].add(int(region))
    needed = {i for fi in proposal['affected_old'] for i in original[fi]}
    needed |= {i for row in proposal['lineage'].values() for i in row['donors']}
    needed |= {i for p in proposal['paths'].values() for i in p if i < n}
    needed |= set(topology['roles']['original_binding']['ear_root_star'])
    mirror = {}
    for i in needed:
        expected = {owners.get(r, r) for r in membership[i]}
        candidates = np.flatnonzero(np.linalg.norm(old-old[i]*[-1, 1, 1], axis=1) <= 1e-7)
        candidates = [int(j) for j in candidates if membership[j] == expected]
        if len(candidates) != 1:
            raise ValueError('Ambiguous current forehead mirror vertex')
        mirror[i] = candidates[0]
    for offset, i in enumerate(range(n, len(positive))):
        mirror[i] = len(positive)+offset

    def cycle(q):
        q = list(q)
        return min(tuple(q[k:]+q[:k]) for k in range(len(q)))

    lookup = {cycle(q): i for i, q in enumerate(original)}
    if len(lookup) != len(original):
        raise ValueError('Duplicate original forehead source face')
    face_mirror = {}
    for i in proposal['affected_old']:
        partner = lookup.get(cycle([mirror[j] for j in reversed(original[i])]))
        if partner is None or regions[partner] != regions[i] or partner in proposal['affected_old']:
            raise ValueError('Forehead patch lacks an independent mirrored skin face')
        face_mirror[i] = partner
    points = np.vstack((positive, positive[n:]*[-1, 1, 1]))
    lineage = dict(proposal['lineage'])
    for i, row in proposal['lineage'].items():
        partner = mirror[i]
        if i == partner or partner in lineage:
            raise ValueError('Forehead mirrored lineage overlaps')
        donors = [mirror[j] for j in row['donors']]
        lineage[partner] = {'donors': donors, 'weights': list(row['weights'])}
        points[partner] = np.asarray(np.asarray(row['weights']) @ old[donors], np.float32)
    final_faces = [list(q) for q in proposal['faces']]
    final_owners = list(topology['face_owners'])
    for i in topology['retiled_old_faces']:
        partner = face_mirror[i]
        final_faces[partner] = [mirror[j] for j in reversed(final_faces[i])]
        final_owners[partner] = face_mirror[topology['face_owners'][i]]
    for i in range(old_count, len(proposal['faces'])):
        final_faces.append([mirror[j] for j in reversed(proposal['faces'][i])])
        final_owners.append(face_mirror[topology['face_owners'][i]])
    final_lookup = {cycle(q): i for i, q in enumerate(final_faces)}
    if len(final_lookup) != len(final_faces):
        raise ValueError('Mirrored forehead retile has duplicate faces')
    negative_new = [final_lookup[cycle([mirror[j] for j in reversed(proposal['faces'][i])])]
                    for i in proposal['affected_new']]
    old_ids = sorted(set(proposal['affected_old']) | set(face_mirror.values()))
    new_ids = sorted(set(proposal['affected_new']) | set(negative_new))
    protected = set(proposal['protected'])
    if set(lineage) & protected or not np.array_equal(points[sorted(protected)], old[sorted(protected)]):
        raise ValueError('Mirrored forehead edit crosses a functional boundary')
    edge_parents = {tuple(row['edge']): row['parent'] for row in topology['edge_lineage']}
    # Only mirror changed/new edges. All unrelated original edge records remain.
    old_edges = {tuple(sorted((a, b))) for q in original for a, b in zip(q, q[1:]+q[:1])}
    positive_edges = {tuple(sorted((a, b))) for i in proposal['affected_new']
                      for q in [proposal['faces'][i]] for a, b in zip(q, q[1:]+q[:1])}
    for edge in positive_edges:
        if edge in old_edges:
            continue
        parent = edge_parents[edge]
        edge_parents[tuple(sorted(mirror[i] for i in edge))] = (
            sorted(mirror[i] for i in parent) if parent is not None else None)
    current_edges = {tuple(sorted((a, b))) for q in final_faces for a, b in zip(q, q[1:]+q[:1])}
    edge_parents = {edge: edge_parents.get(edge, list(edge) if edge in old_edges else None)
                    for edge in current_edges}
    new_open = np.zeros_like(points, dtype=np.float32)
    new_open[:n] = old_open_delta
    for i, row in lineage.items():
        new_open[i] = np.asarray(row['weights']) @ np.asarray(old_open_delta)[row['donors']]
    proposal.update(open_delta=new_open, vertices=points, faces=final_faces, lineage=lineage,
        affected_old=old_ids, affected_new=new_ids, protected=sorted(protected),
        face_owners=final_owners,
        edge_lineage=[{'edge': list(e), 'parent': p} for e, p in edge_parents.items()],
        guide_paths={'positive': proposal['paths'],
                     'negative': {name: [mirror[i] for i in p] for name, p in proposal['paths'].items()}},
        positive_mirror=mirror)
    proposal['metrics'].update(added_vertices=len(points)-n,
                               added_quads=len(final_faces)-old_count,
                               bilateral_lineage_vertices=len(lineage))
    return proposal

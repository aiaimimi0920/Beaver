"""Apply one source-relative chin proposal to both frozen native mesh halves."""
import numpy as np
from chin_source_roles import bind_chin_source_roles
from chin_source_fields import (chin_snapshot_point_fields, chin_apply_point_fields,
                               chin_validate_face_fields)
from chin_source_contract import chin_validate_source_proposal
from chin_source_shape import ChinShapeRefinementContext, chin_refine_selected_export
from nasal_source_geometry import AuthorQuadSurface
from nasal_source_mesh import claim_nasal_source_face


def stage_chin_source_retile(data, authored, recipe, proposer):
    import bmesh
    from ear_topology import ear_loop_values, assign_ear_loop
    from ear_surface import polygon_diagonals
    bm = bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bm.verts.index_update()
        bm.faces.index_update()
        parents, old_faces = list(bm.verts), list(bm.faces)
        vertices = np.asarray([tuple(v.co) for v in parents])
        faces = [tuple(v.index for v in face.verts) for face in old_faces]
        region = bm.faces.layers.int.get('NormalRegion')
        crease = bm.edges.layers.float.get('crease_edge')
        group = bm.edges.layers.int.get('FacialCreaseGroup')
        motion = bm.verts.layers.float_vector.get('MouthOpenDelta')
        if any(layer is None for layer in (region, crease, group, motion)):
            raise ValueError('Lower-mouth source ownership or motion field is missing')
        regions = [face[region] for face in old_faces]
        halves = bind_chin_source_roles(vertices, faces, regions, authored, recipe, -.021)
        positive, negative = halves
        mirror = {positive['roles'][name]: negative['roles'][name]
                  for name in positive['roles']}
        for first, second in mirror.items():
            if np.linalg.norm(vertices[first] * [-1, 1, 1] - vertices[second]) > 1e-7:
                raise ValueError('Lower-mouth source halves are not symmetric counterparts')
        patch = [faces[i] for i in positive['face_ids']]
        surface = AuthorQuadSurface(vertices, patch, [1] * len(patch))
        delta = np.asarray([tuple(v[motion]) for v in parents])
        proposal = proposer(vertices, faces, positive['roles'], delta,
                            author_surface=surface, author_faces=patch,
                            polygon_diagonals=polygon_diagonals,
                            progress=lambda stage, _points, info:
                            print('Lower-mouth source stage:', stage, info, flush=True))
        if list(proposal['source_face_ids']) != positive['face_ids']:
            raise ValueError('Lower-mouth proposer selected a different current disk')
        expected_moved = {positive['roles'][name] for name in recipe['movable_roles']}
        if set(map(int, proposal['lineage'])) != expected_moved:
            raise ValueError('Lower-mouth proposer changed semantic movable ownership')
        shape_context = ChinShapeRefinementContext(vertices, faces, delta, proposal, surface,
                                                   polygon_diagonals, positive['roles'], n=32)
        proposal, shape_proof = chin_refine_selected_export(
            shape_context, proposal, progress=lambda row: print(
                'Lower-mouth shape refinement:', row['iteration'], row['success'],
                row['min_constraint'], flush=True))
        if set(map(int, proposal['lineage'])) != expected_moved:
            raise ValueError('Lower-mouth shape refinement changed movable ownership')
        points, final_faces = vertices.copy(), list(faces)
        lineage = {}
        for index, record in proposal['lineage'].items():
            index = int(index)
            ids, weights = list(map(int, record['donors'])), list(map(float, record['weights']))
            if index not in mirror or any(i not in mirror for i in ids):
                raise ValueError('Lower-mouth lineage escaped the captured semantic domain')
            lineage[index] = {'donors': ids, 'weights': weights}
            lineage[mirror[index]] = {'donors': [mirror[i] for i in reversed(ids)],
                                      'weights': list(reversed(weights))}
        for index, record in lineage.items():
            points[index] = np.asarray(np.asarray(record['weights']) @
                                      vertices[record['donors']], dtype=np.float32)
        solved = np.asarray(proposal['new_vertices'], float)
        if any(np.linalg.norm(solved[int(i)] - points[int(i)]) > 2e-7
               for i in proposal['lineage']):
            raise ValueError('Lower-mouth geometry and field lineage disagree')
        for positive_id, negative_id, quad in zip(positive['face_ids'], negative['face_ids'],
                                                proposal['replacement_faces']):
            final_faces[positive_id] = tuple(map(int, quad))
            final_faces[negative_id] = tuple(mirror[int(i)] for i in reversed(quad))
        selected_ids = set(positive['face_ids']) | set(negative['face_ids'])
        proof = chin_validate_source_proposal(vertices, faces, points, final_faces,
                                             selected_ids, lineage)
        selected = {old_faces[i] for i in selected_ids}
        chin_validate_face_fields(bm, selected)
        fields, deforms = chin_snapshot_point_fields(bm, parents)
        uv = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values()) + list(bm.loops.layers.float_color.values())
        old_loops, values = ear_loop_values(selected, {v: i for i, v in enumerate(parents)},
                                            uv, colors)
        edge_fields = {tuple(sorted(v.index for v in edge.verts)):
                       (float(edge[crease]), int(edge[group])) for edge in bm.edges}
        weights = {i: {k: w for k, w in zip(row['donors'], row['weights']) if w > 0}
                   for i, row in lineage.items()}
        chin_apply_point_fields(parents, weights, fields, deforms)
        for index in lineage:
            parents[index].co = points[index]
        stored_delta = np.asarray([tuple(v[motion]) for v in parents], dtype=np.float32)
        posed = (points.astype(np.float32) + stored_delta).astype(float)
        if any(not polygon_diagonals(points, final_faces[i], posed) for i in selected_ids):
            raise ValueError('Lower-mouth source has no valid neutral and MouthOpen diagonal')
        old_edges = {edge for face in selected for edge in face.edges}
        reused, added = set(), []
        example = old_faces[min(selected_ids)]
        for index in sorted(selected_ids):
            ordered = [parents[i] for i in final_faces[index]]
            face = bm.faces.get(ordered)
            if face is None:
                face = bm.faces.new(ordered, example)
            else:
                claim_nasal_source_face(face, ordered, selected, reused)
            face.smooth = example.smooth
            for loop in face.loops:
                i = loop.vert.index
                assign_ear_loop(loop, weights.get(i, {i: 1.}), old_loops, values, uv, colors)
            added.append(face)
        for face in selected - reused:
            bm.faces.remove(face)
        for edge in old_edges:
            if edge.is_valid and not edge.link_faces:
                if abs(edge[crease]) > 1e-8 or edge[group]:
                    raise ValueError('Lower-mouth retile removes an authored crease')
                bm.edges.remove(edge)
        for edge in bm.edges:
            key = tuple(sorted(v.index for v in edge.verts))
            if key in edge_fields:
                if (float(edge[crease]), int(edge[group])) != edge_fields[key]:
                    raise ValueError('Lower-mouth retile changed retained edge ownership')
            else:
                edge[crease], edge[group] = 0., 0
        if any(len(edge.link_faces) != 2 for face in added for edge in face.edges):
            raise ValueError('Lower-mouth native retile is not manifold')
        bm.normal_update()
        bm.to_mesh(data)
        data.update()
        return {'contract': proof, 'proposal': proposal, 'shape_refinement': shape_proof,
                'source_lineage': lineage, 'stage': 'native-source-retile'}
    finally:
        bm.free()

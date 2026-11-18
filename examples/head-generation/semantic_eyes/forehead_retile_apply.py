"""Apply a verified retile only to the caller's detached native mesh."""
import numpy as np
from chin_source_fields import chin_snapshot_point_fields, chin_validate_face_fields
from nasal_source_mesh import claim_nasal_source_face
from forehead_retile_fields import (stage_forehead_point_fields, apply_forehead_point_fields,
    snapshot_forehead_uv_pins, assign_forehead_uv_pins, snapshot_forehead_edges,
    restore_forehead_edges, forehead_named_field)
from forehead_retile_contract import validate_forehead_retile


def apply_forehead_retile_data(data, proposal):
    import bmesh
    from mathutils import Vector
    from ear_topology import ear_loop_values, assign_ear_loop
    if data.shape_keys is not None:
        raise ValueError('Forehead source retile must precede runtime shape-key creation')
    bm = bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bm.verts.index_update()
        bm.faces.index_update()
        parents, original = list(bm.verts), list(bm.faces)
        old = np.asarray([tuple(v.co) for v in parents], float)
        faces = [list(v.index for v in face.verts) for face in original]
        region = bm.faces.layers.int.get('NormalRegion')
        motion = bm.verts.layers.float_vector.get('MouthOpenDelta')
        if region is None or motion is None:
            raise ValueError('Missing forehead skin or motion ownership')
        regions = [face[region] for face in original]
        contract = validate_forehead_retile(old, faces, regions, proposal)
        old_open = np.asarray([tuple(v[motion]) for v in parents], np.float32)
        fields, deforms = chin_snapshot_point_fields(bm, parents)
        if not deforms:
            raise ValueError('Forehead source lacks authored deform membership')
        staged, staged_deforms, weights = stage_forehead_point_fields(
            len(parents), proposal['lineage'], fields, deforms)
        selected = {original[i] for i in proposal['affected_old']}
        chin_validate_face_fields(bm, selected)
        donor_ids = {i for row in weights.values() for i in row}
        donor_ids |= {i for fi in proposal['affected_old'] for i in faces[fi]}
        donor_faces = {face for i in donor_ids for face in parents[i].link_faces if face[region] == 1}
        uv = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
        indices = {v: i for i, v in enumerate(parents)}
        old_loops, loop_values = ear_loop_values(donor_faces, indices, uv, colors)
        pins = snapshot_forehead_uv_pins(donor_faces, indices, uv)
        edge_values, crease, group = snapshot_forehead_edges(bm)
        old_edges = {edge for face in selected for edge in face.edges}
        original_open = {tuple(sorted(v.index for v in edge.verts))
                         for edge in bm.edges if len(edge.link_faces) == 1}
        points = np.asarray(proposal['vertices'], np.float32).astype(float)
        allocated = list(parents)
        for i in range(len(parents), len(points)):
            example = parents[max(weights[i], key=weights[i].get)]
            allocated.append(bm.verts.new(Vector(points[i]), example))
        apply_forehead_point_fields(allocated, staged, staged_deforms)
        for i in proposal['lineage']:
            allocated[i].co = points[i]
        bm.verts.index_update()
        if any(vertex.index != i for i, vertex in enumerate(allocated)):
            raise ValueError('Native forehead allocation changed source point ordering')
        reused, added = set(), []
        for i in proposal['affected_new']:
            q = proposal['faces'][i]
            ordered = [allocated[j] for j in q]
            face = bm.faces.get(ordered)
            owner = original[proposal['face_owners'][i]]
            if face is None:
                face = bm.faces.new(ordered, owner)
            else:
                claim_nasal_source_face(face, ordered, selected, reused)
            face.smooth = owner.smooth
            for loop in face.loops:
                row = weights.get(loop.vert.index, {loop.vert.index: 1.})
                assign_ear_loop(loop, row, old_loops, loop_values, uv, colors)
                assign_forehead_uv_pins(loop, row, pins, uv)
            added.append(face)
        for face in selected-reused:
            bm.faces.remove(face)
        final_edges = {tuple(row['edge']) for row in proposal['edge_lineage']}
        for edge in old_edges:
            if edge.is_valid and not edge.link_faces:
                key = tuple(sorted(v.index for v in edge.verts))
                if key in final_edges:
                    raise ValueError('Forehead retile lost a required original edge')
                bm.edges.remove(edge)
        edge_proof = restore_forehead_edges(bm, proposal['edge_lineage'], edge_values, crease, group)
        if len(bm.verts)-len(bm.edges)+len(bm.faces) != contract['euler']:
            raise ValueError('Native forehead Euler characteristic changed')
        open_edges = {tuple(sorted(v.index for v in edge.verts))
                      for edge in bm.edges if len(edge.link_faces) == 1}
        if len(open_edges) != contract['open_edges'] or any(
                len(edge.link_faces) not in (1, 2) for face in added for edge in face.edges):
            raise ValueError('Native forehead retile is not conforming')
        if open_edges != original_open:
            raise ValueError('Native forehead open edge identities changed')
        if any(tuple(allocated[i].co) != tuple(old[i]) for i in proposal['protected']):
            raise ValueError('Native forehead moved a protected original point')
        native_points = np.asarray([tuple(v.co) for v in allocated], np.float32)
        if not np.array_equal(native_points, points):
            raise ValueError('Native forehead positions differ from frozen-donor proposal')
        new_open = np.asarray([tuple(v[motion]) for v in allocated], np.float32)
        expected_open_field = np.zeros_like(new_open)
        expected_open_field[:len(parents)] = old_open
        for i, values in staged.items():
            expected_open_field[i] = forehead_named_field(values, motion.name)
        if not np.array_equal(new_open, expected_open_field):
            raise ValueError('Native forehead MouthOpen transport differs from proposal')
        proposal['open_delta'] = new_open.copy()
        bm.normal_update()
        bm.to_mesh(data)
        data.update()
        return {'contract': contract, 'edge_transport': edge_proof,
                'point_layers': [layer.name for _, layer, _ in fields],
                'uv_layers': [layer.name for layer in uv],
                'old_open_delta': old_open, 'new_open_delta': new_open,
                'source_fields_frozen_before_allocation': True}
    finally:
        bm.free()

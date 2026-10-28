"""Frozen native-field transport for five-rail spacing and bridge repair."""
from collections import Counter
import numpy as np
from nasal_spacing_rule import propose_nasal_bridge_spacing
from nasal_source_domain import current_source_features
from nasal_spacing_shape import guard_nasal_spacing_shape
from chin_source_fields import (chin_snapshot_point_fields, chin_apply_point_fields,
                               chin_validate_face_fields)


def stage_nasal_bridge_spacing(data, ordinals, inner, records):
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
        parents, original = list(bm.verts), list(bm.faces)
        vertices = np.asarray([tuple(v.co) for v in parents])
        faces = [tuple(v.index for v in face.verts) for face in original]
        region = bm.faces.layers.int.get('NormalRegion')
        crease = bm.edges.layers.float.get('crease_edge')
        group = bm.edges.layers.int.get('FacialCreaseGroup')
        motion = bm.verts.layers.float_vector.get('MouthOpenDelta')
        if any(layer is None for layer in (region, crease, group, motion)):
            raise ValueError('Nasal spacing source fields are missing')
        regions = [face[region] for face in original]
        old_delta = np.asarray([tuple(v[motion]) for v in parents], dtype=np.float32)
        mapping = {int(k): int(v) for k, v in inner['source_point_mapping'].items()}
        paths = inner['final_full_paths']
        anchors = [mapping[int(record['anchor'])] for record in records[:5]]
        offsets = {path.index(anchor) for path, anchor in zip(paths, anchors)}
        if len(paths) != 5 or len(anchors) != 5 or len(offsets) != 1:
            raise ValueError('Nasal spacing handoff anchors are inconsistent')
        ear_bases = {value for name, value in ordinals.items()
                     if name.startswith('Ear hollow pinna ')}
        if len(ear_bases) != 2:
            raise ValueError('Nasal spacing needs two explicit current ear owners')
        ear_regions = {base + offset for base in ear_bases for offset in (0, 100, 110)}
        spec = propose_nasal_bridge_spacing(vertices, faces, regions, paths, ear_regions,
                                             outer_anchor_offset=next(iter(offsets)))
        points = np.asarray(spec['vertices'], dtype=np.float32).astype(float)
        final_faces = [tuple(face) for face in spec['faces']]
        lineage = spec['lineage']
        affected, edited = set(spec['affected_faces']), spec['edited_faces']
        features = current_source_features(vertices, faces, regions, ordinals)
        peak = {i for i, membership in enumerate(features['membership'])
                if 1 in membership and np.linalg.norm(vertices[i] - features['nose_peak']) < 1e-9}
        protected = features['apertures'] | features['first_lids'] | features['oral']
        protected |= features['first_lips'] | features['attachments'] | peak
        if len(peak) != 1 or set(lineage) & protected:
            raise ValueError('Nasal spacing moved a peak, aperture, support or attachment')
        if any(set(faces[i]) & protected for i in edited):
            raise ValueError('Nasal bridge retile changes a functional attachment fan')
        if len(points) != len(vertices) or len(final_faces) != len(faces) or len(edited) != 4:
            raise ValueError('Nasal spacing changed its source topology budget')
        if any(tuple(final_faces[i]) != face for i, face in enumerate(faces) if i not in edited):
            raise ValueError('Nasal spacing changed an unowned face')
        if any(not np.array_equal(points[i], vertices[i]) for i in range(len(points)) if i not in lineage):
            raise ValueError('Nasal spacing moved a vertex outside its donor transaction')
        skin_cycles = set()
        for face, owner in zip(faces, regions):
            if owner == 1:
                for ordered in (face, face[::-1]):
                    skin_cycles.update(ordered[k:] + ordered[:k] for k in range(len(ordered)))
        edge_fields = {tuple(sorted(v.index for v in edge.verts)):
                       (float(edge[crease]), int(edge[group])) for edge in bm.edges}
        weights = {}
        for index, row in lineage.items():
            ids, shares = tuple(row['donors']), np.asarray(row['weights'], float)
            if (len(ids) not in (2, 4) or len(set(ids)) != len(ids) or len(shares) != len(ids)
                    or not np.isfinite(shares).all() or shares.min() < 0
                    or abs(shares.sum() - 1) > 1e-8):
                raise ValueError('Invalid nasal spacing donor weights')
            if len(ids) == 2 and tuple(sorted(ids)) not in edge_fields:
                raise ValueError('Nasal spacing edge donor is not a current source edge')
            if len(ids) == 4 and (ids not in skin_cycles
                    or abs(shares[0] * shares[2] - shares[1] * shares[3]) > 1e-8):
                raise ValueError('Nasal spacing quad donor is not current bilinear skin')
            expected = np.asarray(shares @ vertices[list(ids)], dtype=np.float32).astype(float)
            if not np.array_equal(expected, points[index]):
                raise ValueError('Stored nasal spacing point disagrees with its frozen donor')
            weights[index] = {i: float(w) for i, w in zip(ids, shares) if w > 0}
        old_edges = Counter(tuple(sorted((a, b))) for face in faces
                            for a, b in zip(face, face[1:] + face[:1]))
        new_edges = Counter(tuple(sorted((a, b))) for face in final_faces
                            for a, b in zip(face, face[1:] + face[:1]))
        if len(old_edges) != len(new_edges) or {e for e, n in old_edges.items() if n == 1} != {
                e for e, n in new_edges.items() if n == 1}:
            raise ValueError('Nasal spacing changed Euler characteristic or open boundaries')
        if len(set(map(frozenset, final_faces))) != len(final_faces):
            raise ValueError('Nasal spacing introduced duplicate faces')
        changed_faces = {original[i] for i in edited}
        chin_validate_face_fields(bm, changed_faces)
        fields, deforms = chin_snapshot_point_fields(bm, parents)
        donor_ids = set(weights) | {i for row in weights.values() for i in row}
        donor_ids |= {i for fi in affected for i in faces[fi]}
        donor_faces = {face for i in donor_ids for face in parents[i].link_faces if face[region] == 1}
        uv = list(bm.loops.layers.uv.values())
        colors = list(bm.loops.layers.color.values()) + list(bm.loops.layers.float_color.values())
        old_loops, values = ear_loop_values(donor_faces, {v: i for i, v in enumerate(parents)}, uv, colors)
        chin_apply_point_fields(parents, weights, fields, deforms)
        delta = np.asarray([tuple(v[motion]) for v in parents], dtype=np.float32)
        posed = (points.astype(np.float32) + delta).astype(float)
        for index in affected:
            if regions[index] != 1 or not polygon_diagonals(points, final_faces[index], posed):
                raise ValueError('Nasal spacing has an invalid neutral or MouthOpen source quad')
            a, b = vertices[list(faces[index])], points[list(final_faces[index])]
            old_normal = np.cross(a, np.roll(a, -1, axis=0)).sum(axis=0)
            new_normal = np.cross(b, np.roll(b, -1, axis=0)).sum(axis=0)
            if old_normal @ new_normal <= 0:
                raise ValueError('Nasal spacing reversed source face orientation')
        shape_proof = guard_nasal_spacing_shape(vertices, faces, points, final_faces, affected,
                                                old_delta, delta, polygon_diagonals)
        for index in lineage:
            parents[index].co = points[index]
        replacement = {}
        removed_edges = {edge for face in changed_faces for edge in face.edges}
        for index, quad in edited.items():
            ordered = [parents[i] for i in quad]
            if bm.faces.get(ordered) is not None:
                raise ValueError('Nasal bridge replacement already exists outside its old fan')
            face = bm.faces.new(ordered, original[index])
            face.smooth = original[index].smooth
            replacement[index] = face
        for face in changed_faces:
            bm.faces.remove(face)
        for edge in removed_edges:
            if edge.is_valid and not edge.link_faces:
                if abs(edge[crease]) > 1e-8 or edge[group]:
                    raise ValueError('Nasal bridge flip removes an authored crease')
                bm.edges.remove(edge)
        for index in affected:
            face = replacement.get(index, original[index])
            for loop in face.loops:
                i = loop.vert.index
                assign_ear_loop(loop, weights.get(i, {i: 1.}), old_loops, values, uv, colors)
            if any(len(edge.link_faces) != 2 for edge in face.edges):
                raise ValueError('Nasal spacing native edit is not manifold')
        for edge in bm.edges:
            key = tuple(sorted(v.index for v in edge.verts))
            if key in edge_fields:
                if (float(edge[crease]), int(edge[group])) != edge_fields[key]:
                    raise ValueError('Nasal spacing changed retained edge ownership')
            else:
                edge[crease], edge[group] = 0., 0
        bm.normal_update()
        bm.to_mesh(data)
        data.update()
        return {'source_lineage': lineage, 'affected_old_cycles': [faces[i] for i in sorted(affected)],
                'affected_final_cycles': [final_faces[i] for i in sorted(affected)],
                'edited_faces': edited, 'final_full_paths': paths, 'roles': spec['roles'],
                'metrics': spec['metrics'], 'axis_tangent': spec['axis_tangent'], 'fade': spec['fade'],
                'sampled_shape_guard': shape_proof,
                'scope': 'Five-rail metric spacing and upper bridge; inherited rear branch ambiguity remains',
                'acceptance': 'Pending actual export, field, expression and visual verification'}
    finally:
        bm.free()

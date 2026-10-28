"""Frozen native-field transport for five-rail spacing and bridge repair."""
from collections import Counter
import numpy as np
from nasal_spacing_rule import propose_nasal_bridge_spacing
from nasal_rear_rule import propose_nasal_rear_spacing
from orbital_support_native import prepare_orbital_support
from nasal_return_provenance import verify_nasal_return_provenance
from nasal_source_domain import current_source_features
from nasal_spacing_shape import guard_nasal_spacing_shape
from chin_source_fields import (chin_snapshot_point_fields, chin_apply_point_fields,
                               chin_validate_face_fields)


def stage_nasal_bridge_spacing(data, ordinals, inner, records, generated_return=None, baseline=None, orbital_calibration=None, assembly_offset=None):
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
        features = current_source_features(vertices, faces, regions, ordinals)
        provenance = None
        if orbital_calibration is not None:
            if generated_return is not None or baseline is not None or not np.isfinite(float(assembly_offset)):
                raise ValueError('Orbital transaction has conflicting source modes or frame')
            if float(assembly_offset) != -.021:
                raise ValueError('Current native blink generator requires its explicit -0.021 assembly frame')
            from semantic_eye_runtime import closure_delta
            spec = prepare_orbital_support(vertices, faces, regions, features, paths,
                lambda point: closure_delta(point[0], point[2] - assembly_offset, 'skin', orbital_calibration))
            edit_budget = 0
        elif generated_return is None:
            spec = propose_nasal_bridge_spacing(vertices, faces, regions, paths, ear_regions,
                                                 outer_anchor_offset=next(iter(offsets)))
            edit_budget = 4
        else:
            from scipy.spatial import cKDTree
            distance, matches = cKDTree(vertices).query(vertices * [-1, 1, 1], k=2)
            mirror = matches[:, 0]
            spec = propose_nasal_rear_spacing(vertices, faces, regions,
                [list(reversed(path)) for path in paths], mirror,
                ear_regions=ear_regions, protected_vertices=features['protected'])
            required = set(spec['roles']['moved']) | set(spec['roles']['donors']) | set(spec['roles']['rim_path'])
            required |= {i for fi in spec['affected_faces'] for i in faces[fi]}
            required |= {int(mirror[i]) for i in list(required)}
            left, right = (ordinals['Ear hollow pinna -1'], ordinals['Ear hollow pinna 1'])
            owners = {a + offset: b + offset for a, b in ((left, right), (right, left))
                      for offset in (0, 100, 110)}
            for i in required:
                partner = int(mirror[i])
                if distance[i, 0] > 2e-6 or distance[i, 1] <= 2e-6 or int(mirror[partner]) != i:
                    raise ValueError('Rear neighborhood has ambiguous mirror correspondence')
                if {owners.get(r, r) for r in features['membership'][i]} != features['membership'][partner]:
                    raise ValueError('Rear mirrored source owners disagree')
            cycle = lambda q: min(tuple(q[k:]) + tuple(q[:k]) for k in range(len(q)))
            face_owners = {cycle(q): r for q, r in zip(faces, regions)}
            for fi in spec['affected_faces']:
                mirrored = [int(mirror[i]) for i in reversed(faces[fi])]
                if face_owners.get(cycle(mirrored)) != regions[fi]:
                    raise ValueError('Rear mirrored support topology disagrees')
            parent, rim = map(int, spec['roles']['flow_with_return'][-2:])
            provenance = verify_nasal_return_provenance(vertices, faces, regions, generated_return,
                [(parent, rim), (int(mirror[parent]), int(mirror[rim]))])
            spec['faces'], spec['edited_faces'] = faces, {}
            spec['metrics'] = {k: spec[k] for k in ('positive_move_mm', 'old_turns', 'new_turns',
                'old_rim_turns', 'new_rim_turns', 'max_turn_increase_deg', 'objective')}
            edit_budget = 0
        points = np.asarray(spec['vertices'], dtype=np.float32).astype(float)
        final_faces = [tuple(face) for face in spec['faces']]
        lineage = spec['lineage']
        affected, edited = set(spec['affected_faces']), spec['edited_faces']
        peak = {i for i, membership in enumerate(features['membership'])
                if 1 in membership and np.linalg.norm(vertices[i] - features['nose_peak']) < 1e-9}
        protected = features['apertures'] | features['first_lids'] | features['oral']
        protected |= features['first_lips'] | features['attachments'] | peak
        protected |= spec.get('additional_protected', set())
        if len(peak) != 1 or set(lineage) & protected:
            raise ValueError('Nasal spacing moved a peak, aperture, support or attachment')
        if any(set(faces[i]) & protected for i in edited):
            raise ValueError('Nasal bridge retile changes a functional attachment fan')
        if len(points) != len(vertices) or len(final_faces) != len(faces) or len(edited) != edit_budget:
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
        if changed_faces:
            chin_validate_face_fields(bm, changed_faces)
        else:
            for kind in ('int', 'float', 'string', 'float_vector'):
                accessor = getattr(bm.loops.layers, kind, None)
                if accessor is not None and list(accessor.values()):
                    raise ValueError('Unsupported rear spacing corner field')
        fields, deforms = chin_snapshot_point_fields(bm, parents)
        if generated_return is not None or orbital_calibration is not None:
            if not deforms or any(not any(values[index] for _, values in deforms) for index in weights):
                raise ValueError('Rear spacing has no authored semantic membership')
            for _, values in deforms:
                for index, donors in weights.items():
                    if any(values[donor] != values[index] for donor, share in donors.items() if share > 0):
                        raise ValueError('Rear spacing donor semantic membership is inconsistent')
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
            if provenance is not None and any(set(final_faces[index]) == set(q) for pair in provenance['pairs'] for q in pair['support_cycles']):
                moved = posed[list(final_faces[index])]
                posed_normal = np.cross(moved, np.roll(moved, -1, axis=0)).sum(axis=0)
                if new_normal[1] <= 0 or posed_normal[1] <= 0:
                    raise ValueError('Generated posterior strip no longer faces backward')
        shape_proof = guard_nasal_spacing_shape(vertices, faces, points, final_faces, affected,
                                                old_delta, delta, polygon_diagonals)
        blink_proof = None
        if orbital_calibration is not None:
            blink_proof = guard_nasal_spacing_shape(vertices, faces, points, final_faces, affected,
                spec['old_blink_delta'], spec['new_blink_delta'], polygon_diagonals,
                selector_delta=delta, pose_name='blink.L+blink.R')
        cumulative_proof = None
        if generated_return is not None:
            if baseline is None or len(baseline['vertices']) != len(vertices):
                raise ValueError('Rear spacing requires a frozen pre-spacing shape baseline')
            canonical = lambda q: min(tuple(q[k:]) + tuple(q[:k]) for k in range(len(q)))
            old_cycles = {canonical(q) for q in baseline['faces']}
            if any(canonical(faces[i]) not in old_cycles for i in affected):
                raise ValueError('Cumulative rear source support changed topology')
            cumulative_proof = guard_nasal_spacing_shape(baseline['vertices'], faces, points,
                final_faces, affected, baseline['mouth_delta'], delta, polygon_diagonals)
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
            if any(len(edge.link_faces) != (old_edges.get(tuple(sorted(v.index for v in edge.verts)), 2)
                   if generated_return is not None else 2) for edge in face.edges):
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
                'metrics': spec['metrics'], 'axis_tangent': spec.get('axis_tangent'), 'fade': spec.get('fade'),
                'sampled_shape_guard': shape_proof, 'cumulative_shape_guard': cumulative_proof,
                'generated_return_provenance': provenance, 'generated_blink_guard': spec.get('generated_blink_guard'),
                'generated_blink_shape_guard': blink_proof,
                'scope': spec.get('scope', 'Five-rail metric spacing and upper bridge; inherited rear branch ambiguity remains'),
                'acceptance': 'Pending actual export, field, expression and visual verification'}
    finally:
        bm.free()


def capture_nasal_spacing_baseline(data):
    motion = data.attributes.get('MouthOpenDelta')
    if motion is None or motion.domain != 'POINT' or motion.data_type != 'FLOAT_VECTOR':
        raise ValueError('Spacing baseline lacks authored MouthOpenDelta')
    return {'vertices': np.asarray([tuple(v.co) for v in data.vertices], dtype=np.float32),
            'faces': [tuple(face.vertices) for face in data.polygons],
            'mouth_delta': np.asarray([tuple(row.vector) for row in motion.data], dtype=np.float32)}

"""Build a conforming source boundary strip after cage fairing, before ear fitting.

API reference: https://docs.blender.org/api/4.3/bmesh.ops.html#bmesh.ops.subdivide_edges
"""

def refine_outline_after_fairing(shell, front_count, config, height, origin, rejected_source_faces=()):
    """Refine only source patches that pass actual triangulated quality checks.

    Subdivision replaces vertex CustomData and invalidates Python BMVert handles.
    Reacquire a unique live vertex for each old coordinate, then restore its exact
    value. Trial meshes are private until every affected triangle passes. Unsafe
    patches are excluded deterministically, without lowering quality thresholds.
    """
    import bmesh
    from math import acos, degrees, isfinite
    from mathutils import Vector
    from ear_topology import refined_outline
    data = shell.data
    vertices = [tuple(v.co) for v in data.vertices]
    faces = [tuple(p.vertices) for p in data.polygons]
    chain = list(shell.get('source_outline_chain', []))
    anchors = [i for i in chain if config['ear_root_min_height'] <= (vertices[i][2] - origin) / height <= config['ear_root_max_height']]
    protected = set(anchors[:2] + anchors[-1:])
    points, _, prediction = refined_outline(vertices, faces, chain, protected=protected)
    pairs = [(a, b) for a, b in zip(chain, chain[1:]) if a not in protected and b not in protected]
    assert len(points) - len(vertices) == len(pairs)
    bm = bmesh.new()
    check = None
    baseline = None
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.faces.ensure_lookup_table()
        bm.normal_update()
        region = bm.faces.layers.int.new('OutlineSourceRegion')
        identity = bm.faces.layers.int.new('OutlineOriginalFace')
        for i, face in enumerate(bm.faces):
            face[region] = 1 if i < front_count else 2
            face[identity] = i + 1
        baseline = bm.copy()
        bmesh.ops.triangulate(baseline, faces=list(baseline.faces), quad_method='BEAUTY', ngon_method='BEAUTY')
        baseline.normal_update()
        base_normals = {}
        base_angles = {}

        def minimum_angle(face):
            ps = [v.co for v in face.verts]
            angles = []
            for i in range(3):
                a = ps[(i + 1) % 3] - ps[i]
                b = ps[(i + 2) % 3] - ps[i]
                if min(a.length, b.length) < 1e-09:
                    return 0.0
                angles.append(degrees(acos(max(-1.0, min(1.0, a.dot(b) / (a.length * b.length))))))
            return min(angles)
        base_identity = baseline.faces.layers.int['OutlineOriginalFace']
        for face in baseline.faces:
            key = face[base_identity]
            base_normals.setdefault(key, []).append(face.normal.copy())
            base_angles[key] = min(base_angles.get(key, 180.0), minimum_angle(face))
        original = [v.co.copy() for v in bm.verts]
        edges = set()
        affected = set()
        pair_refs = []
        targets = []
        skipped = []
        for (a, b), target in zip(pairs, points[len(vertices):]):
            va, vb = (bm.verts[a], bm.verts[b])
            edge = bm.edges.get((va, vb))
            assert edge is not None and len(edge.link_faces) == 1, 'Expected source boundary edge'
            owner = edge.link_faces[0]
            assert owner[region] == 1, 'Outline must belong to the front source surface'
            local_edges = {edge}
            if len(owner.verts) == 4:
                opposite = [e for e in owner.edges if va not in e.verts and vb not in e.verts]
                assert len(opposite) == 1
                local_edges.add(opposite[0])
            touched = {f[identity] for e in local_edges for f in e.link_faces}
            if touched.intersection(rejected_source_faces):
                skipped.append((a, b))
                continue
            edges.update(local_edges)
            pair_refs.append((a, b))
            targets.append(target)
        assert pair_refs, 'No outline refinement passed source triangle quality checks'
        for edge in edges:
            affected.update((f[identity] for f in edge.link_faces))
        bmesh.ops.subdivide_edges(bm, edges=sorted(edges, key=lambda e: tuple(sorted((v.index for v in e.verts)))), cuts=1, smooth=0.0, use_grid_fill=True, use_single_edge=False)
        remaining = set(bm.verts)
        restored = []
        max_roundoff = 0.0
        for i, point in enumerate(original):
            matches = [v for v in remaining if (v.co - point).length < 2e-07]
            assert len(matches) == 1, f'Original source vertex missing or ambiguous: {i}'
            vertex = matches[0]
            remaining.remove(vertex)
            max_roundoff = max(max_roundoff, (vertex.co - point).length)
            vertex.co = point
            restored.append(vertex)
        assert all((tuple(v.co) == tuple(p) for v, p in zip(restored, original))), 'Original support not exactly restored'
        new = list(remaining)
        moved = []
        for (a, b), target in zip(pair_refs, targets):
            midpoint = (original[a] + original[b]) / 2
            found = [v for v in new if (v.co - midpoint).length < 2e-07]
            assert len(found) == 1, 'Ambiguous source boundary midpoint'
            moved.append((found[0], found[0].co.copy(), Vector(target)))
        assert all((f[region] in (1, 2) and f[identity] in base_normals for f in bm.faces)), 'Source face identity lost'
        accepted = None
        failures = []
        for fraction in (1.0, 0.5, 0.25, 0.0):
            for v, start, target in moved:
                v.co = start.lerp(target, fraction)
            bm.normal_update()
            check = bm.copy()
            bmesh.ops.triangulate(check, faces=list(check.faces), quad_method='BEAUTY', ngon_method='BEAUTY')
            check.normal_update()
            ci = check.faces.layers.int['OutlineOriginalFace']
            cr = check.faces.layers.int['OutlineSourceRegion']
            minimum = 180.0
            normal_dot = 1.0
            valid = True
            for face in check.faces:
                if face[ci] not in affected:
                    continue
                angle = minimum_angle(face)
                dot = max((face.normal.dot(n) for n in base_normals[face[ci]]))
                minimum = min(minimum, angle)
                normal_dot = min(normal_dot, dot)
                if face.calc_area() <= 1e-12 or not all((isfinite(c) for v in face.verts for c in v.co)) or (face[cr] == 1 and face.normal.y >= -1e-08) or (dot < 0.7071067) or (angle + 1e-05 < min(base_angles[face[ci]], 8.0)):
                    failures.append({'fraction': fraction, 'source': face[ci], 'angle': angle, 'base_angle': base_angles[face[ci]], 'dot': dot, 'normal': list(face.normal), 'area': face.calc_area(), 'points': [list(v.co) for v in face.verts], 'source_points': [vertices[i] for i in faces[face[ci] - 1]]})
                    valid = False
            check.free()
            check = None
            if valid:
                accepted = {'fraction': fraction, 'minimum_affected_angle': minimum, 'minimum_original_surface_normal_dot': normal_dot}
                break
        if accepted is None:
            rejected = set(rejected_source_faces) | {f['source'] for f in failures}
            assert rejected != set(rejected_source_faces), f'No progress selecting safe outline segments: {failures}'
            return refine_outline_after_fairing(shell, front_count, config, height, origin, tuple(sorted(rejected)))
        transitions = [f for f in bm.faces if f[identity] in affected and len(f.verts) > 4]
        if transitions:
            bmesh.ops.triangulate(bm, faces=transitions, quad_method='BEAUTY', ngon_method='BEAUTY')
        assert all((len(f.verts) <= 4 for f in bm.faces if f[identity] in affected)), 'Unresolved local source n-gon'
        bm.to_mesh(data)
        data.update()
        report = {**prediction, **accepted, 'maximum_curve_deviation_m': max(((target - start).length for _, start, target in moved)) * accepted['fraction'], 'stage': 'conforming source strip after cage fairing', 'new_vertices': len(new), 'skipped_edges': skipped, 'rejected_source_faces': list(rejected_source_faces), 'faces_before': len(faces), 'faces_after': len(data.polygons), 'existing_vertices_unchanged': True, 'source_faces_unchanged': False, 'operator_roundoff_restored_m': max_roundoff, 'quads_after': sum((len(p.vertices) == 4 for p in data.polygons)), 'triangles_after': sum((len(p.vertices) == 3 for p in data.polygons)), 'affected_source_faces': sorted(affected), 'reference_normals': {str(k): [list(n) for n in values] for k, values in base_normals.items()}}
        return report
    finally:
        if check is not None:
            check.free()
        if baseline is not None:
            baseline.free()
        bm.free()

def validate_final_outline_triangles(data, reference):
    """Check source coverage and separate outline changes from later fitting.

    Unaffected side normals are diagnostics: ear/jaw fitting intentionally changes
    them after the reference snapshot. New posterior returns have their own
    orientation rule and must not inherit an original source-face identity.
    """
    from mathutils import Vector
    from math import isfinite
    identity = data.attributes.get('OutlineOriginalFace')
    region = data.attributes.get('OutlineSourceRegion')
    assert identity is not None and region is not None, 'Lost source strip provenance'
    affected = set(reference['affected_source_faces'])
    normals = reference['reference_normals']
    normal_region = data.attributes.get('NormalRegion')
    generated = data.attributes.get('OutlineGeneratedReturn')
    assert normal_region is not None, 'Lost final normal regions'
    worst = 1.0
    count = 0
    returns = 0
    coverage = set()
    source_count = 0
    unaffected_side_dots = []
    for face in data.polygons:
        key = identity.data[face.index].value
        is_return = generated is not None and generated.data[face.index].value == 1
        if normal_region.data[face.index].value in (1, 2) and (not is_return):
            assert key > 0 and str(key) in normals, 'Lost source face provenance'
            assert region.data[face.index].value == normal_region.data[face.index].value, 'Source normal region mismatch'
            coverage.add(key)
        if not is_return and key <= 0:
            continue
        assert len(face.vertices) == 3, 'Final outline check requires actual triangles'
        points = [data.vertices[i].co for i in face.vertices]
        normal = (points[1] - points[0]).cross(points[2] - points[0])
        assert normal.length > 2e-12 and all((isfinite(c) for p in points for c in p)), 'Invalid final source triangle'
        normal.normalize()
        if is_return:
            assert key == 0 and region.data[face.index].value == 0 and (normal.y > 0), 'Invalid generated return triangle'
            returns += 1
            continue
        if sum((p.x for p in points)) < 0:
            normal.x = -normal.x
        dot = max((normal.dot(Vector(n)) for n in normals[str(key)]))
        source_count += 1
        if key in affected:
            assert dot > 0.7071067, f'Final outline triangle changed original surface orientation: face={face.index}, source={key}, dot={dot}'
            worst = min(worst, dot)
            count += 1
        elif region.data[face.index].value == 2:
            unaffected_side_dots.append({'face': face.index, 'source': key, 'normal_dot': dot})
    assert count > 0, 'No final source strip triangles checked'
    assert coverage == {int(k) for k in normals}, 'Final source coverage incomplete'
    return {'triangles': count, 'finite_nondegenerate_source_triangles': source_count, 'unaffected_side_before_fit_diagnostics': unaffected_side_dots, 'generated_return_triangles': returns, 'source_face_coverage': len(coverage), 'minimum_source_normal_dot': worst, 'scope': 'Direction constraint only on outline-affected patches; all source triangles finite/nondegenerate with complete provenance; generated returns face backward. Unaffected side normal changes are pre-fit diagnostics, not a no-deformation or intersection proof.'}

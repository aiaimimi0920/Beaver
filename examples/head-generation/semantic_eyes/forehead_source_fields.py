"""Read original native field ownership before any shared quad allocation."""
from chin_source_fields import chin_snapshot_point_fields
from forehead_retile_fields import snapshot_forehead_edges


def capture_forehead_source_fields(data):
    """Return mathematical ownership inputs, not a mutable native mesh handle.

    Neutrality includes corner UV/color/pin continuity and identical face fields.
    A zero crease alone never authorizes a retopology transition. Semantic point
    classes include deform membership; ordinary float fields remain interpolated
    by the existing native transport transaction.
    """
    import bmesh
    bm = bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.verts.index_update()
        bm.faces.ensure_lookup_table()
        bm.faces.index_update()
        values, _, _ = snapshot_forehead_edges(bm)
        region = bm.faces.layers.int.get('NormalRegion')
        if region is None:
            raise ValueError('Forehead source has no semantic face ownership')
        fields, deforms = chin_snapshot_point_fields(bm, list(bm.verts))
        discrete = {layer.name: list(rows) for kind, layer, rows in fields
                    if kind in ('int', 'string') and layer.name != '.select_vert'}
        for number, (_, rows) in enumerate(deforms):
            key = 'deform_membership_' + str(number)
            if key in discrete:
                raise ValueError('Deform semantic key collides with an authored point field')
            discrete[key] = [
                tuple(sorted(row.items())) for row in rows]
        if not deforms:
            raise ValueError('Forehead source has no authored deform membership')
        uv_layers = list(bm.loops.layers.uv.values())
        color_layers = list(bm.loops.layers.color.values())
        color_layers += list(bm.loops.layers.float_color.values())
        face_layers = []
        for kind in ('float', 'int', 'string', 'float_vector', 'color', 'float_color'):
            accessor = getattr(bm.faces.layers, kind, None)
            if accessor is not None:
                face_layers.extend(accessor.values())

        def loop_signature(loop):
            return ([tuple(loop[layer].uv) for layer in uv_layers],
                    [bool(loop[layer].pin_uv) for layer in uv_layers],
                    [tuple(loop[layer]) for layer in color_layers])

        neutral = set()
        for edge in bm.edges:
            key = tuple(sorted(vertex.index for vertex in edge.verts))
            linked = list(edge.link_faces)
            if values[key] != (0., 0, False, True) or len(linked) != 2:
                continue
            first, second = linked
            if (first[region] != 1 or second[region] != 1
                    or first.material_index != second.material_index
                    or first.smooth != second.smooth
                    or any(first[layer] != second[layer] for layer in face_layers)):
                continue
            continuous = True
            for vertex in edge.verts:
                signatures = [loop_signature(loop) for face in linked
                              for loop in face.loops if loop.vert == vertex]
                if not signatures or any(row != signatures[0] for row in signatures[1:]):
                    continuous = False
                    break
            if continuous:
                neutral.add(key)
        return {'edge_fields': dict(values), 'neutral_parent_edges': neutral,
                'point_discrete': discrete,
                'original_vertex_count': len(bm.verts),
                'original_face_count': len(bm.faces),
                'frozen_before_allocation': True}
    finally:
        bm.free()

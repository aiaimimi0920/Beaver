"""Apply final-source quad ear layouts while preserving pinned interfaces and UVs."""


def ear_loop_values(faces, local_by_vertex, uv_layers, color_layers):
    loops, values = {}, {}
    for face in faces:
        for loop in face.loops:
            local = local_by_vertex[loop.vert]
            current = {layer:tuple(loop[layer].uv) for layer in uv_layers}
            current.update({layer:tuple(loop[layer]) for layer in color_layers})
            if local in values:
                if any(max(abs(a-b) for a,b in zip(current[k],values[local][k]))>1e-7 for k in current):
                    raise ValueError('Ear has an unsupported internal UV/color seam')
            else:
                loops[local], values[local] = None, current
    return loops, values


def assign_ear_loop(loop, weights, old_loops, values, uv_layers, color_layers):
    parent = max(weights,key=weights.get)
    # Only immutable corner snapshots survive BMesh topology allocation.
    for layer in uv_layers + color_layers:
        size = len(values[parent][layer])
        value = tuple(sum(weight*values[i][layer][c] for i,weight in weights.items()) for c in range(size))
        if layer in uv_layers:
            loop[layer].uv = value
        else:
            loop[layer] = value


def apply_final_ear(bm, old_faces, original_vertices, result, base_region, crease):
    from mathutils import Vector
    regions = bm.faces.layers.int['NormalRegion']
    old_set = set(old_faces)
    fixed = set(result['fixed_vertex_ids'])
    old_positions = [tuple(v.co) for v in original_vertices]
    for i,vertex in enumerate(original_vertices):
        if i not in fixed and any(face not in old_set for face in vertex.link_faces):
            raise ValueError('Unpinned ear vertex is shared with another source patch')
    weights = inset_vertex_weights(result)
    for kind in ('int','float','string','float_vector'):
        accessor = getattr(bm.loops.layers,kind,None)
        if accessor is not None and list(accessor.values()):
            raise ValueError('Unsupported non-UV/color ear corner attributes: '+kind)
    uv_layers = list(bm.loops.layers.uv.values())
    color_layers = list(bm.loops.layers.color.values()) + list(bm.loops.layers.float_color.values())
    old_loops, values = ear_loop_values(old_faces,{v:i for i,v in enumerate(original_vertices)},uv_layers,color_layers)
    vertices = list(original_vertices)
    for i,point in enumerate(result['vertices']):
        if i < len(original_vertices):
            vertices[i].co = point
        else:
            parent = max(weights[i],key=weights[i].get)
            vertices.append(bm.verts.new(Vector(point),original_vertices[parent]))
    for i in fixed:
        if tuple(vertices[i].co) != old_positions[i]:
            raise ValueError('Final root or outline pin moved during ear retopology')
    old_quads = {frozenset(face.verts):face for face in old_faces if len(face.verts)==4}
    examples = {face[regions]:face for face in old_faces}
    keep = set()
    added = []
    reverse = old_positions[0][0] < 0
    for local,zone in zip(result['quads'],result['ear_face_zones']):
        ids = list(reversed(local)) if reverse else list(local)
        points = [vertices[i] for i in ids]
        region = base_region
        if zone not in ('main','smooth_cap'):
            region += 100
            if crease and zone=='inner_return':
                region += 10
        face = old_quads.get(frozenset(points))
        if face is not None:
            keep.add(face)
        else:
            face = bm.faces.new(points,examples.get(region,old_faces[0]))
        face[regions] = region
        face.smooth = True
        index_by_vertex = {vertices[i]:i for i in ids}
        for loop in face.loops:
            assign_ear_loop(loop,weights[index_by_vertex[loop.vert]],old_loops,values,uv_layers,color_layers)
        added.append(face)
    old_edges = {edge for face in old_faces for edge in face.edges}
    for face in old_faces:
        if face not in keep:
            bm.faces.remove(face)
    for edge in old_edges:
        if edge.is_valid and not edge.link_faces:
            bm.edges.remove(edge)
    return {'quads':len(added),'vertices':len(vertices),'fixed_vertices':len(fixed),
            'main_faces':result['ear_main_faces'],'return_faces':result['ear_return_band_faces'],
            'min_both_diagonal_dot':result['min_diagonal_pair_dot'],
            'uv_lineage':'original authored corners retained; new inset vertices use their source-quad weights'}


def retopologize_final_ears(data, reports, ordinals, original_vertex_count):
    import bmesh
    from face import C
    bm = bmesh.new()
    bm.from_mesh(data)
    bm.verts.ensure_lookup_table(); bm.faces.ensure_lookup_table()
    bm.verts.index_update(); bm.faces.index_update()
    regions = bm.faces.layers.int.get('NormalRegion')
    if regions is None:
        raise ValueError('Missing source region labels')
    specs = []
    root_ids = (0,1,5,9,15,21,27)
    for report in reports:
        base = ordinals[report['name']]
        faces = [face for face in bm.faces if face[regions] in (base,base+100,base+110)][:46]
        candidates = {v for face in faces for v in face.verts}
        seeds = {}
        positions = list(report['root_positions']) + [report['lower_bridge_root_position']]
        if len(positions)!=8:
            raise ValueError('Unsupported source ear root chain')
        for local,point in zip(root_ids+(38,),positions):
            target = (point[0],point[1],point[2]-.021)
            nearest = sorted((sum((v.co[j]-target[j])**2 for j in range(3)),v.index) for v in candidates)
            if nearest[0][0]>4e-12 or (len(nearest)>1 and nearest[1][0]<1e-16):
                raise ValueError('Ambiguous or missing final ear root anchor')
            seeds[local]=nearest[0][1]
        reverse = positions[0][0]<0
        mapping = map_final_ear([tuple(v.index for v in face.verts) for face in faces],EAR_QUAD_EXPECTED_FACES,seeds,reverse)
        specs.append((base,faces,[bm.verts[i] for i in mapping]))
    output = []
    raise RuntimeError('Initializer source is consumed by the opt-in flow adapter; not a standalone retopology path')

"""Conforming source skin/attachment patch with explicit interpolation lineage."""


def flow_point_fields(bm, target, parents, weights):
    """Interpolate numeric point fields; refuse ambiguous discrete semantics."""
    weights = {i:w for i,w in weights.items() if w > 1e-12}
    if abs(sum(weights.values())-1)>1e-8:
        raise ValueError('Invalid source point interpolation weights')
    for kind in ('float','float_vector','color','float_color','shape','int','string'):
        accessor = getattr(bm.verts.layers,kind,None)
        if accessor is None:
            continue
        for layer in accessor.values():
            values = {i:parents[i][layer] for i in weights}
            first = next(iter(values.values()))
            if kind in ('int','string'):
                if any(value != first for value in values.values()):
                    raise ValueError('Source patch crosses a discrete point-field boundary')
                target[layer] = first
            elif kind == 'float':
                target[layer] = sum(weights[i]*value for i,value in values.items())
            else:
                target[layer] = tuple(sum(weights[i]*values[i][k] for i in weights)
                                      for k in range(len(first)))
    for layer in bm.verts.layers.deform.values():
        target[layer].clear()
        keys = set().union(*(set(parents[i][layer].keys()) for i in weights))
        for key in keys:
            target[layer][key] = sum(weights[i]*parents[i][layer].get(key,0) for i in weights)


def flow_new_point(bm, parents, weights):
    from mathutils import Vector
    position = sum((parents[i].co*weight for i,weight in weights.items()),Vector((0,0,0)))
    target = bm.verts.new(position,parents[max(weights,key=weights.get)])
    flow_point_fields(bm,target,parents,weights)
    return target


def flow_ordered_quad(face, first, second):
    points = list(face.verts)
    for reverse in (False,True):
        ordered = list(reversed(points)) if reverse else points
        for shift in range(4):
            cycle = ordered[shift:]+ordered[:shift]
            if cycle[:2] == [first,second]:
                return cycle,reverse
    raise ValueError('Source patch has no requested directed boundary edge')


def flow_reconnect_skin(bm, original, generated, layout):
    regions = bm.faces.layers.int['NormalRegion']
    R,L,n = original[39],original[27],generated[41]
    lower = [face for face in R.link_faces if L in face.verts and face[regions] in (1,2)]
    if len(lower) != 1 or len(lower[0].verts) != 4:
        raise ValueError('Upper attachment has no unique skin quad')
    lower = lower[0]
    order,reverse = flow_ordered_quad(lower,R,L)
    _,_,I,J = order
    upper = [face for face in R.link_faces if face != lower and J in face.verts
             and face[regions] == lower[regions]]
    if len(upper) != 1 or len(upper[0].verts) != 4:
        raise ValueError('Upper attachment lacks the adjacent semantic skin cell')
    upper = upper[0]
    up,up_reverse = flow_ordered_quad(upper,R,J)
    if up_reverse != reverse:
        raise ValueError('Adjacent source skin cells have inconsistent winding')
    _,_,K,U = up
    shared = bm.edges.get((R,J)); rim = bm.edges.get((R,U)); seam = bm.edges.get((R,L))
    if shared is None or rim is None or seam is None or len(rim.link_faces) != 1:
        raise ValueError('Upper source transition is not the expected free-rim patch')
    crease = bm.edges.layers.float.get('crease_edge')
    group = bm.edges.layers.int.get('FacialCreaseGroup')
    if (crease and abs(shared[crease])>1e-8) or (group and shared[group] != 0):
        raise ValueError('Cannot remove a protected source skin edge')
    old_edges = {edge for face in (lower,upper) for edge in face.edges}
    parents = [R,L,I,J,K,U]
    index = {vertex:i for i,vertex in enumerate(parents)}
    uv_layers = list(bm.loops.layers.uv.values())
    color_layers = list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
    loops,values = ear_loop_values((lower,upper),index,uv_layers,color_layers)
    u = layout['upper_attachment_flow']['edge_parameters'][1]
    lineage = {vertex:{index[vertex]:1.} for vertex in parents}
    lineage[n] = {0:1-u,1:u}
    s = flow_new_point(bm,parents,{0:.5,5:.5})
    lineage[s] = {0:.5,5:.5}
    A,B = [I,J,K],[L,n,R,s,U]
    inside = []
    for uu,tt in ((.35,.35),(.5,.35),(.65,.45)):
        source_weights = flow_ruled_weights(A,B,uu,tt)
        weights = {}
        for vertex,weight in source_weights.items():
            for parent,share in lineage[vertex].items():
                weights[parent] = weights.get(parent,0)+weight*share
        vertex = flow_new_point(bm,parents,weights)
        lineage[vertex] = weights
        inside.append(vertex)
    left,middle,right = inside
    a0,a1,a2 = A; b0,b1,b2,b3,b4 = B
    quads = [(a0,a1,middle,left),(middle,b2,b1,left),(b1,b0,a0,left),
             (a1,a2,right,middle),(middle,right,b3,b2),(right,a2,b4,b3)]
    for points in quads:
        face = bm.faces.new(list(reversed(points)) if reverse else points,lower)
        face.smooth = True
        for loop in face.loops:
            assign_ear_loop(loop,lineage[loop.vert],loops,values,uv_layers,color_layers)
    # Preserve crease/group on both subdivided source interfaces.
    for old,chain in ((seam,(R,n,L)),(rim,(R,s,U))):
        for a,b in zip(chain,chain[1:]):
            edge = bm.edges.get((a,b))
            if edge is None:
                raise ValueError('Missing conforming source child edge')
            if crease: edge[crease] = old[crease]
            if group: edge[group] = old[group]
    bm.faces.remove(lower); bm.faces.remove(upper)
    for edge in old_edges:
        if edge.is_valid and not edge.link_faces:
            bm.edges.remove(edge)
    for a,b in ((R,n),(n,L)):
        if len(bm.edges.get((a,b)).link_faces) != 2:
            raise ValueError('Upper attachment source seam is not manifold')
    return {'skin_quads_before':2,'skin_quads_after':6,
            'attachment_positions':[tuple(v.co) for v in (L,n,R)],
            'new_common_normal_positions':[tuple(v.co) for v in (n,R)],
            'free_boundary_positions':[tuple(v.co) for v in (R,s,U)],
            'scope':'Source topology; native area, crossing, pose and visual gates required'}


def source_flow_normal_ids(data, reports):
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    tree = KDTree(len(data.vertices))
    for vertex in data.vertices:
        tree.insert(vertex.co,vertex.index)
    tree.balance()
    ids = set()
    for report in reports:
        for point in report['new_common_normal_positions']:
            matches = tree.find_range(Vector(point),2e-6)
            if len(matches) != 1:
                raise ValueError('Ambiguous source attachment normal vertex')
            ids.add(matches[0][1])
    if len(ids) != 2*len(reports):
        raise ValueError('Incomplete source attachment normal ownership')
    return ids


def validate_lower_flow_interface(bm, faces, vertices):
    edge = bm.edges.get((vertices[0],vertices[29]))
    if edge is None or len(edge.link_faces) != 2 or any(face not in faces for face in edge.link_faces):
        raise ValueError('Lower ear source interface is not internal to the template')
    crease = bm.edges.layers.float.get('crease_edge')
    group = bm.edges.layers.int.get('FacialCreaseGroup')
    if (crease and abs(edge[crease])>1e-8) or (group and edge[group] != 0):
        raise ValueError('Cannot reconnect a protected lower ear crease')
    if len({face.material_index for face in edge.link_faces}) != 1:
        raise ValueError('Cannot reconnect a lower ear material boundary')
    # The complete original template is also checked for per-corner UV/color
    # continuity by ear_loop_values before any original face is removed.

"""Attach a narrow open posterior return without moving the original ear root."""

def return_weight(height):
    t = max(0.0, min(1.0, (height-.11)/.125 if height <= .235 else (.37-height)/.135))
    return t*t*(3-2*t)


def lower_return_weight(height):
    t = max(0.0, min(1.0, (height-.01)/.045 if height <= .055 else (.115-height)/.06))
    return t*t*(3-2*t)


def add_final_return(data):
    import bmesh
    from math import sin, pi
    from face import C,H,Y0,Z0
    bm = bmesh.new()
    bm.from_mesh(data)
    bm.verts.ensure_lookup_table()
    original = [(v, v.co.copy()) for v in bm.verts]
    regions = bm.faces.layers.int.get('NormalRegion')
    assert regions is not None
    skin = {v for f in bm.faces if f[regions] in (1,2) for v in f.verts}
    pending = list(skin)
    while pending:
        for edge in pending.pop().link_edges:
            for v in edge.verts:
                if v not in skin:
                    skin.add(v)
                    pending.append(v)
    selected = []
    for edge in list(bm.edges):
        if not edge.is_boundary or any(v not in skin for v in edge.verts):
            continue
        a,b = edge.verts
        height = ((a.co.z+b.co.z)/2+.021-Y0)/H
        if .01 < height < .38 and min((Z0+v.co.y)/H for v in edge.verts) > .18:
            selected.append(edge)
    assert selected, 'No permitted posterior ear-under boundary'
    ids = {v for edge in selected for v in edge.verts}
    maps = [{v:v for v in ids}]
    new_faces = []
    sources = []
    for edge in selected:
        source = edge.link_faces[0]
        loop = next(loop for loop in source.loops if loop.edge == edge)
        sources.append((source,loop.vert,loop.link_loop_next.vert))
    for fraction in (.5,1.0):
        row = {}
        for v in ids:
            height = (v.co.z+.021-Y0)/H
            upper = return_weight(height)
            lower = lower_return_weight(height)
            width = C['ear_post_return_width_m']*upper + C['ear_post_lower_return_width_m']*lower
            weight = max(upper,lower)
            if weight < 1e-8:
                row[v] = v
                continue
            point = v.co.copy()
            sign = 1 if point.x>0 else -1
            point.x -= sign*min(width,abs(point.x)*.5)*fraction
            point.y += C['ear_post_return_depth_m']*upper*sin(pi*fraction/2)
            if (point-v.co).length < 1e-7:
                row[v] = v
                continue
            new = bm.verts.new(point, v)
            row[v] = new
        previous = maps[-1]
        for source,a,b in sources:
            corners = list(dict.fromkeys((previous[b],previous[a],row[a],row[b])))
            if len(corners)<3:
                continue
            face = bm.faces.new(corners, source)
            face.smooth = True
            for new_loop in face.loops:
                origin = a if new_loop.vert in (previous[a],row[a]) else b
                old_loop = next(old_loop for old_loop in source.loops if old_loop.vert == origin)
                for layer in bm.loops.layers.uv.values():
                    new_loop[layer].uv = old_loop[layer].uv
                for kind in ('color','float_color','float','int','float_vector'):
                    collection = getattr(bm.loops.layers, kind, None)
                    if collection is not None:
                        for layer in collection.values():
                            new_loop[layer] = old_loop[layer]
            new_faces.append(face)
        maps.append(row)
    bm.normal_update()
    assert all(face.calc_area()>1e-12 for face in new_faces), 'Collapsed posterior return'
    assert all(face.normal.y>0 for face in new_faces), 'Posterior return faces must face backward'
    assert all(len(edge.link_faces)<=2 for face in new_faces for edge in face.edges), 'Non-manifold return'
    assert all((v.co-point).length<1e-9 for v,point in original), 'Original root or skin moved'
    result = {'added_faces':len(new_faces),'original_vertices_unchanged':True,'selected_free_edges':len(selected),'open_back':True}
    bm.to_mesh(data)
    bm.free()
    data.update()
    return result

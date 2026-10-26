"""Replace paired source-return endpoint triangles by bounded thin quad caps."""



def check_stored_cap_neighbors(before, after, polygons, report):
    import numpy as np
    before,after=np.asarray(before),np.asarray(after)
    for index in report['adjacent_faces']:
        face=polygons[index]
        patterns=((0,1,2),(0,2,3),(0,1,3),(1,2,3)) if len(face)==4 else ((0,1,2),)
        def measured(vertices):
            points=vertices[list(face)]
            normals=np.array([np.cross(points[b]-points[a],points[c]-points[a]) for a,b,c in patterns])
            areas=np.linalg.norm(normals,axis=1)
            return normals/np.maximum(areas[:,None],1e-30),areas
        old,old_area=measured(before);new,new_area=measured(after)
        if (np.min(np.sum(old*new,axis=1))<=.7 or np.min(new_area/old_area)<=.2
                or (len(face)==4 and min(new[0]@new[1],new[2]@new[3])<.49)):
            raise ValueError('Stored return cap damaged an adjacent source face')
    ids=report['boundary_global_ids']
    if np.linalg.norm(after[ids]-before[ids],axis=1).max()>.0005:
        raise ValueError('Stored return cap exceeded its displacement budget')


def retopologize_source_return_caps(bm, original_vertex_count, ear_bases):
    import numpy as np
    regions = bm.faces.layers.int['NormalRegion']
    selected_regions = {1,2} | set(ear_bases)
    selected_regions |= {r+offset for r in ear_bases for offset in (100,110)}
    bm.verts.index_update(); bm.faces.index_update()
    original_ids = set(range(original_vertex_count))
    remaining = {face for face in bm.faces if face[regions] in selected_regions and len(face.verts)==3}
    pairs = []
    while remaining:
        first = min(remaining,key=lambda face:face.index)
        matches = []
        for second in remaining-{first}:
            shared = set(first.verts)&set(second.verts)
            union = set(first.verts)|set(second.verts)
            new_ids = {v.index for v in union}-original_ids
            if (second[regions]==first[regions] and len(shared)==2 and len(union)==4
                    and len(new_ids)==2 and len({v.index for v in shared}&new_ids)==1):
                matches.append(second)
        if len(matches)!=1:
            raise ValueError('Source return triangles do not form a unique endpoint pair')
        second=matches[0]; pairs.append((first,second)); remaining-={first,second}
    uv_layers=list(bm.loops.layers.uv.values())
    color_layers=list(bm.loops.layers.color.values())+list(bm.loops.layers.float_color.values())
    reports=[]
    for first,second in pairs:
        bm.verts.ensure_lookup_table();bm.verts.index_update();bm.faces.index_update()
        vertices=list(bm.verts);faces=list(bm.faces)
        geometry=[tuple(v.co) for v in vertices]
        polygons=[tuple(v.index for v in f.verts) for f in faces]
        result=solve_return_cap_with_seed(geometry,polygons,(first.index,second.index),original_ids,ear_quad_least_squares)
        ids=result['boundary_global_ids']
        loops,values=ear_loop_values((first,second),{vertices[i]:i for i in ids},uv_layers,color_layers)
        for i,point in zip(ids,result['vertices']):
            vertices[i].co=point
        check_stored_cap_neighbors(geometry,[tuple(v.co) for v in vertices],polygons,result)
        stored=np.array([tuple(vertices[i].co) for i in ids])
        normal,angle,area=ear_quad_measures((stored-stored[0])*1000,[[0,1,2,3]])
        if normal.min()<.49 or angle.min()<result['old_min_angle']-1e-5 or area.min()<=1e-8:
            raise ValueError('Stored source return cap lost its geometry bounds')
        face=bm.faces.new([vertices[i] for i in ids],first)
        face.smooth=True
        for loop in face.loops:
            assign_ear_loop(loop,{loop.vert.index:1.},loops,values,uv_layers,color_layers)
        edges=set(first.edges)|set(second.edges)
        bm.faces.remove(first);bm.faces.remove(second)
        for edge in edges:
            if edge.is_valid and not edge.link_faces:bm.edges.remove(edge)
        reports.append({k:v for k,v in result.items() if k not in ('vertices','quads','failed_initializations')})
    return reports

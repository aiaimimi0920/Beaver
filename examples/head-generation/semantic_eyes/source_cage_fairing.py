def fair_source_cage(shell, maximum_move=.002):
    """Place coarse source vertices more smoothly without adding topology."""
    from mathutils import Vector
    data=shell.data;old=[v.co.copy() for v in data.vertices]
    old_normals=[p.normal.copy() for p in data.polygons]
    faces=[tuple(p.vertices) for p in data.polygons]
    neighbors=[set() for _ in old];uses={};owned=[[] for _ in old]
    for face_id,face in enumerate(faces):
        for a,b in zip(face,face[1:]+face[:1]):
            neighbors[a].add(b);neighbors[b].add(a)
            key=tuple(sorted((a,b)));uses[key]=uses.get(key,0)+1
        for i in face:owned[i].append(face_id)
    centers=[sum((old[i] for i in f),Vector())/len(f) for f in faces]
    sharp=[set() for _ in old];crease=data.attributes.get('crease_edge')
    for edge in data.edges:
        a,b=edge.vertices
        if uses[tuple(sorted((a,b)))]==1 or (crease and crease.data[edge.index].value>=.5):
            sharp[a].add(b);sharp[b].add(a)
    feature_groups=data.attributes.get('FacialCreaseGroup')
    pinned=set()
    if feature_groups:
        for edge in data.edges:
            if feature_groups.data[edge.index].value==3:pinned.update(edge.vertices)
    for _ in range(2):pinned.update(j for i in list(pinned) for j in neighbors[i])
    result=[]
    for i,p in enumerate(old):
        if i in pinned:
            target=p
        elif len(sharp[i])==2:
            target=(6*p+sum((old[j] for j in sharp[i]),Vector()))/8
        elif sharp[i] or len(neighbors[i])<3:
            target=p
        else:
            n=len(neighbors[i]);f=sum((centers[j] for j in owned[i]),Vector())/len(owned[i])
            r=sum(((p+old[j])/2 for j in neighbors[i]),Vector())/n
            target=(f+2*r+(n-3)*p)/n
        if abs(p.x) < 1e-8:
            outer=[j for j in neighbors[i] if uses[tuple(sorted((i,j)))]==1 and abs(old[j].x)>1e-8]
            if len(outer)==1 and i not in pinned:
                mirrored_neighbor=old[outer[0]].copy();mirrored_neighbor.x=0.0
                target=(6*p+2*mirrored_neighbor)/8
            target=target.copy();target.x=0.0
        delta=target-p
        if delta.length>maximum_move:delta*=maximum_move/delta.length
        result.append(p+delta)
    for vertex,point in zip(data.vertices,result):vertex.co=point
    data.update()
    valid=all(poly.area>1e-12 and poly.normal.dot(normal)>.1 for poly,normal in zip(data.polygons,old_normals))
    if not valid:
        for vertex,point in zip(data.vertices,old):vertex.co=point
        data.update();raise ValueError('Source cage fairing would invert or collapse a face')
    return {'topology_unchanged':True,'vertices':len(old),'maximum_move_m':max((a-b).length for a,b in zip(old,result)), 'operation':'bounded source vertex placement, not subdivision or decimation'}


def sample_source_cage(shell, baseline_fairing, maximum_move=.001):
    """Bounded native limit-position samples; retain coarse topology and shared rims."""
    import bpy
    from math import isfinite
    from mathutils import Vector
    assert not isinstance(maximum_move,bool) and isfinite(maximum_move) and maximum_move>0
    data = shell.data
    data.update()
    old = [v.co.copy() for v in data.vertices]
    normals = [p.normal.copy() for p in data.polygons]
    delta = data.attributes.get('MouthOpenDelta')
    assert delta is not None and delta.domain=='POINT' and delta.data_type=='FLOAT_VECTOR', 'Missing or invalid paired mouth pose field'
    assert len(delta.data)==len(old), 'Mouth pose count mismatch'
    old_delta = [v.vector.copy() for v in delta.data]
    base_obj=None;base_data=None
    try:
        base_obj=shell.copy();base_data=data.copy();base_obj.data=base_data
        baseline_report=baseline_fairing(base_obj)
        baseline=[v.co.copy() for v in base_data.vertices]
        normals=[p.normal.copy() for p in base_data.polygons]
        assert len(baseline)==len(old) and [tuple(p.vertices) for p in base_data.polygons]==[tuple(p.vertices) for p in data.polygons], 'Baseline changed source topology'
    finally:
        if base_obj is not None:bpy.data.objects.remove(base_obj,do_unlink=True)
        if base_data is not None and base_data.users==0:bpy.data.meshes.remove(base_data)
    data.calc_loop_triangles()
    triangles=[tuple(t.vertices) for t in data.loop_triangles]
    old_posed=[p+d for p,d in zip(baseline,old_delta)]
    def cross_at(points,triangle):
        a,b,c=(points[i] for i in triangle)
        return (b-a).cross(c-a)
    old_neutral_normals=[cross_at(baseline,t) for t in triangles]
    old_posed_normals=[cross_at(old_posed,t) for t in triangles]
    assert all(n.length>2e-12 for n in old_neutral_normals), 'Degenerate original neutral cage'
    assert all(n.length>2e-12 for n in old_posed_normals), 'Degenerate original posed cage'
    assert all(p.x >= -1e-7 for p in old), 'Expected authored half cage'
    assert all(m.type == 'MIRROR' for m in shell.modifiers), 'Unexpected pre-sampling modifier'
    neighbors = [set() for _ in old]
    uses = {}
    for poly in data.polygons:
        ids = list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            neighbors[a].add(b);neighbors[b].add(a)
            key=tuple(sorted((a,b)));uses[key]=uses.get(key,0)+1
    pins=set();groups=data.attributes.get('FacialCreaseGroup')
    crease=data.attributes.get('crease_edge')
    for edge in data.edges:
        a,b=edge.vertices
        external=uses[tuple(sorted((a,b)))]==1 and max(abs(old[a].x),abs(old[b].x))>1e-7
        hard=crease and crease.data[edge.index].value>=.9999
        aperture=groups and groups.data[edge.index].value==3
        if external or hard or aperture:pins.update((a,b))
    aperture_links={}
    if groups:
        for edge in data.edges:
            if groups.data[edge.index].value==3:
                a,b=edge.vertices
                aperture_links.setdefault(a,set()).add(b);aperture_links.setdefault(b,set()).add(a)
    unseen=set(aperture_links);eye_support=set()
    while unseen:
        component={unseen.pop()};front=set(component)
        while front:
            front={j for i in front for j in aperture_links[i] if j not in component}
            component.update(front);unseen.difference_update(front)
        if all(abs(old[i].x)>1e-7 for i in component):eye_support.update(component)
    for _ in range(2):eye_support.update(j for i in list(eye_support) for j in neighbors[i])
    pins.update(eye_support)
    weights={i:0. for i in pins};front=set(pins)
    for value in (.25,.5,.75):
        front={j for i in front for j in neighbors[i] if j not in weights}
        weights.update({i:value for i in front})
    temp=None;temp_data=None;labels={}
    try:
        temp=shell.copy();temp_data=data.copy();temp.data=temp_data
        bpy.context.scene.collection.objects.link(temp)
        sub=temp.modifiers.new('Bounded native sampling reference','SUBSURF')
        sub.subdivision_type='CATMULL_CLARK';sub.levels=1;sub.render_levels=1
        sub.use_limit_surface=True;sub.quality=3;sub.use_creases=True
        for vertex in temp.data.vertices:
            group=temp.vertex_groups.new(name=f'__sample_source_{vertex.index}')
            group.add([vertex.index],1.0,'REPLACE');labels[group.index]=vertex.index
        def evaluate():
            temp.data.update();bpy.context.view_layer.update()
            graph=bpy.context.evaluated_depsgraph_get();graph.update()
            obj=temp.evaluated_get(graph);mesh=obj.to_mesh(preserve_all_data_layers=True,depsgraph=graph)
            try:
                best=[-1.]*len(old);points=[[] for _ in old]
                for vertex in mesh.vertices:
                    for group in vertex.groups:
                        i=labels.get(group.group)
                        if i is None or vertex.co.x*old[i].x < -1e-10:continue
                        if group.weight>best[i]+1e-7:best[i]=group.weight;points[i]=[]
                        if abs(group.weight-best[i])<1e-7:points[i].append(vertex.co.copy())
                selected=[]
                for weight,items in zip(best,points):
                    assert weight>=1-1e-6 and items, 'Missing original-vertex correspondence'
                    assert all((p-items[0]).length<1e-7 for p in items), 'Ambiguous original-vertex correspondence'
                    selected.append(items[0])
                return selected,len(mesh.vertices),len(mesh.polygons)
            finally:obj.to_mesh_clear()
        neutral,reference_vertices,reference_faces=evaluate()
        for vertex in temp.data.vertices:vertex.co=old[vertex.index]+old_delta[vertex.index]
        posed,posed_vertices,posed_faces=evaluate()
        assert (reference_vertices,reference_faces)==(posed_vertices,posed_faces), 'Pose changed reference topology'
    finally:
        if temp is not None:bpy.data.objects.remove(temp,do_unlink=True)
        if temp_data is not None and temp_data.users==0:bpy.data.meshes.remove(temp_data)
    assert isfinite(maximum_move) and maximum_move>0
    factors=[]
    for i,(point,target) in enumerate(zip(baseline,neutral)):
        distance=max((target-point).length,(posed[i]-old_posed[i]).length)
        factor=weights.get(i,1.)*min(1.,maximum_move/max(distance,1e-12))
        factors.append(factor)
    try:
        for i,vertex in enumerate(data.vertices):
            factor=factors[i]
            if not factor:
                vertex.co=baseline[i];delta.data[i].vector=old_delta[i]
                continue
            target=baseline[i].lerp(neutral[i],factor)
            if abs(old[i].x)<1e-7:target.x=0.
            vertex.co=target
            posed_target=old_posed[i].lerp(posed[i],factor)
            if abs(old[i].x)<1e-7:posed_target.x=0.
            delta.data[i].vector=posed_target-vertex.co
        data.update()
        assert all(isfinite(c) for v in data.vertices for c in v.co), 'Nonfinite sampled source'
        assert all(isfinite(c) for d in delta.data for c in d.vector), 'Nonfinite sampled pose'
        assert all(p.area>1e-12 and p.normal.dot(n)>.1 for p,n in zip(data.polygons,normals)), 'Sampled source inverted/collapsed a face'
        actual_neutral=[v.co.copy() for v in data.vertices]
        for triangle,old_normal in zip(triangles,old_neutral_normals):
            new_normal=cross_at(actual_neutral,triangle)
            assert new_normal.length>2e-12 and new_normal.normalized().dot(old_normal.normalized())>.1, 'Sampled neutral cage inverted/collapsed a triangle'
        moved=max((v.co-p).length for v,p in zip(data.vertices,baseline))
        assert moved<=maximum_move+1e-6, 'Source sampling exceeded neutral bound'
        actual_posed=[v.co+d.vector for v,d in zip(data.vertices,delta.data)]
        posed_move=max((p-q).length for p,q in zip(actual_posed,old_posed))
        assert posed_move<=maximum_move+1e-6, 'Source sampling exceeded posed bound'
        for triangle,old_normal in zip(triangles,old_posed_normals):
            new_normal=cross_at(actual_posed,triangle)
            assert new_normal.length>2e-12 and new_normal.normalized().dot(old_normal.normalized())>.1, 'Sampled posed cage inverted/collapsed a triangle'
        polygon_options=[polygon_diagonals(actual_neutral,tuple(poly.vertices),actual_posed) for poly in data.polygons]
        assert all(polygon_options), 'Final paired source polygons invalid: '+str([i for i,v in enumerate(polygon_options) if not v])
    except Exception:
        for i,vertex in enumerate(data.vertices):vertex.co=old[i];delta.data[i].vector=old_delta[i]
        data.update();raise
    return {'operation':'bounded native limit-position sampling at original coarse vertices',
            'topology_unchanged':True,'source_vertices':len(old),'source_polygons':len(data.polygons),
            'reference_level':1,'use_limit_surface':True,'quality':3,
            'reference_vertices':reference_vertices,'reference_polygons':reference_faces,
            'pinned_vertices':len(pins),'eye_support_pinned_vertices':len(eye_support),'maximum_move_m':max((v.co-p).length for v,p in zip(data.vertices,old)),'maximum_increment_from_baseline_m':moved,'maximum_posed_increment_from_baseline_m':posed_move,'baseline_fairing':baseline_report,'pins_reference':'retained bounded-fairing positions',
            'maximum_unconstrained_move_m':max((a-b).length for a,b in zip(old,neutral)),
            'paired_polygon_validity':True,'paired_pose_same_factors':True,'visual_acceptance':'pending'}

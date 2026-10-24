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

"""Bounded local plane fairing for accidental lower attachment folds."""
def plane_target(point,center,normal):
    d=sum((p-c)*n for p,c,n in zip(point,center,normal))
    return tuple(p-d*n for p,n in zip(point,normal))

def fair_attachment_folds(data,reports):
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    data.update();old=[v.co.copy() for v in data.vertices]
    tree=KDTree(len(old))
    for i,p in enumerate(old):tree.insert(p,i)
    tree.balance();anchors=[];seeds=[]
    for report in reports:
        mapping={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]];ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)));assert d<.000002;ids.append(i)
            mapping[key]=ids
        anchors.append(mapping);seeds.extend(mapping['root_positions'][:3])
    edges={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):edges.setdefault(tuple(sorted((a,b))),[]).append(poly.index)
    normals=[p.normal.copy() for p in data.polygons]
    moved=set();history=[]
    for iteration in range(12):
        before=[v.co.copy() for v in data.vertices];proposals={};selected_edges=0
        for edge,faces in edges.items():
            if len(faces)!=2:continue
            a,b=[data.polygons[j] for j in faces]
            if a.normal.dot(b.normal)>=.70710678:continue
            center=(old[edge[0]]+old[edge[1]])*.5
            distance=min((center-old[i]).length for i in seeds)
            if distance>.020:continue
            if center.z>min(old[i].z for i in seeds)+.012:continue
            taper=max(0.,min(1.,(.020-distance)/.012))
            taper=taper*taper*(3-2*taper)
            normal=(a.normal+b.normal).normalized()
            if normal.length<.9:continue
            ids=set(a.vertices)|set(b.vertices);center=sum((before[i] for i in ids),Vector())/len(ids)
            for i in ids:proposals.setdefault(i,[]).append(before[i].lerp(Vector(plane_target(before[i],center,normal)),taper))
            selected_edges+=1
        if iteration==0:assert selected_edges>=2, 'Expected both lower attachment folds'
        history.append(selected_edges)
        if not proposals:break
        targets={}
        for i,points in proposals.items():
            target=before[i]+(sum(points,Vector())/len(points)-before[i])*.35
            delta=target-old[i]
            if delta.length>.002:target=old[i]+delta.normalized()*.002
            targets[i]=target
        weights={i:1. for i in targets}
        for attempt in range(16):
            for i,q in targets.items():data.vertices[i].co=before[i].lerp(q,weights[i])
            data.update();invalid=[p for p,n in zip(data.polygons,normals) if any(i in targets for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
            if not invalid:break
            for i in {i for p in invalid for i in p.vertices if i in targets}:weights[i]*=.5
            if attempt==14:weights={i:0. for i in targets}
        assert not invalid,'Lower attachment fairing inverted a face'
        moved.update(targets)
    assert all((v.co-old[i]).length<1e-9 for i,v in enumerate(data.vertices) if i not in moved)
    for report,mapping in zip(reports,anchors):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'edge_count_per_iteration':history,'selected_vertices':len(moved),'maximum_displacement_m':max(((data.vertices[i].co-old[i]).length for i in moved),default=0),'topology_unchanged':True}

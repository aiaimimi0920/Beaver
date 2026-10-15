"""Local skin and welded lower-ear attachment fairing."""

def relax_local_skin(data,reports):
    """Bounded 3D fairing of interior skin only; pin ear roots and free boundary."""
    from face import H,Y0,Z0
    from mathutils import Vector
    data.update()
    original=[v.co.copy() for v in data.vertices]
    regions=data.attributes['NormalRegion']
    neighbors={};uses={};memberships={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for i in ids:memberships.setdefault(i,set()).add(regions.data[poly.index].value)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            neighbors.setdefault(a,set()).add(b);neighbors.setdefault(b,set()).add(a)
            edge=tuple(sorted((a,b)));uses[edge]=uses.get(edge,0)+1
    boundary={i for edge,count in uses.items() if count==1 for i in edge}
    eligible={i for i,p in enumerate(original) if i not in boundary and bool(memberships.get(i)) and memberships[i].issubset({1,2}) and .12<(p.z+.021-Y0)/H<.38 and (Z0+p.y)/H>.05 and abs(p.x)>.035}
    from mathutils.kdtree import KDTree
    tree=KDTree(len(original))
    for i,p in enumerate(original):tree.insert(p,i)
    tree.balance();anchors=[];root_ids=set()
    for report in reports:
        mapping={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
                assert d<.000002;ids.append(i)
            mapping[key]=ids;root_ids.update(ids)
        anchors.append(mapping)
    moving_roots={i for i in root_ids if .12<(original[i].z+.021-Y0)/H<.38 and (Z0+original[i].y)/H>.05}
    eligible.update(moving_roots)
    lower_seeds={i for mapping in anchors for i in mapping['lower_bridge_root_position']}
    patch=set(lower_seeds);front=set(lower_seeds)
    for _ in range(2):
        front={j for i in front for j in neighbors.get(i,())}
        patch.update(front)
    patch={i for i in patch if .20<(original[i].z+.021-Y0)/H<.34 and (Z0+original[i].y)/H>.18}
    eligible.update(patch)

    cotangent={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for k in range(1,len(ids)-1):
            tri=[ids[0],ids[k],ids[k+1]]
            for j in range(3):
                left,right,opposite=tri[j],tri[(j+1)%3],tri[(j+2)%3]
                u=original[left]-original[opposite];v=original[right]-original[opposite]
                weight=max(0.,min(100.,u.dot(v)/max(u.cross(v).length,1e-12)))
                cotangent.setdefault(left,{})[right]=cotangent.get(left,{}).get(right,0.)+weight
                cotangent.setdefault(right,{})[left]=cotangent.get(right,{}).get(left,0.)+weight
    normals=[p.normal.copy() for p in data.polygons]
    for iteration in range(24):
        before=[v.co.copy() for v in data.vertices];proposal={}
        for i in eligible:
            linked=neighbors[i]
            weights_local=cotangent.get(i,{})
            total=sum(weights_local.values())
            target=sum((before[j]*w for j,w in weights_local.items()),Vector())/total if total>1e-12 else sum((before[j] for j in linked),Vector())/len(linked)
            point=before[i]+(target-before[i])*.3
            delta=point-original[i]
            depth=(Z0+original[i].y)/H
            height=(original[i].z+.021-Y0)/H
            extra=max(0.,min(1.,(depth-.12)/.08,(height-.12)/.05,(.38-height)/.05))
            extra=extra*extra*(3-2*extra)
            limit=.003 if i in patch else ((.001 if i in moving_roots else .002)+.002*extra)
            if delta.length>limit:point=original[i]+delta.normalized()*limit
            proposal[i]=point
        weights={i:1. for i in eligible}
        for attempt in range(16):
            for i in eligible:data.vertices[i].co=before[i].lerp(proposal[i],weights[i])
            data.update()
            invalid=[p for p,n in zip(data.polygons,normals) if any(i in eligible for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
            if not invalid:break
            for i in {i for p in invalid for i in p.vertices if i in eligible}:weights[i]*=.5
            if attempt==14:weights={i:0. for i in eligible}
    assert all((v.co-original[i]).length<1e-9 for i,v in enumerate(data.vertices) if i not in eligible)
    for report,mapping in zip(reports,anchors):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'selected_vertices':len(eligible),'maximum_displacement_m':max(((data.vertices[i].co-original[i]).length for i in eligible),default=0),'moving_lower_attachment_patch':len(patch),'moving_shared_roots':len(moving_roots),'other_boundary_fixed':True,'iterations':24}

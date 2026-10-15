"""Keep the calibrated lower-ear frontal envelope during local fairing."""
PROFILE=[(-.40,.1994),(-.375,.2285),(-.35,.2527),(-.325,.2714),(-.30,.2846),(-.275,.2967),(-.25,.3074)]

def target_width(h):
    for (a,x),(b,y) in zip(PROFILE,PROFILE[1:]):
        if a<=h<=b:return x+(y-x)*(h-a)/(b-a)
    return None

def fit_front_width(data,reports):
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    from face import H,Y0,Z0
    data.update();old=[v.co.copy() for v in data.vertices]
    height=max(p.z for p in old)-min(p.z for p in old)
    middle=(max(p.z for p in old)+min(p.z for p in old))*.5
    region=data.attributes['NormalRegion'];skin=set();neighbors={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        if region.data[poly.index].value in (1,2):skin.update(ids)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            neighbors.setdefault(a,set()).add(b);neighbors.setdefault(b,set()).add(a)
    pending=list(skin)
    while pending:
        for j in neighbors.get(pending.pop(),()):
            if j not in skin:skin.add(j);pending.append(j)
    edges={tuple(sorted((i,j))) for i in skin for j in neighbors[i]}
    tree=KDTree(len(old))
    for i,p in enumerate(old):tree.insert(p,i)
    tree.balance();anchors=[]
    for report in reports:
        mapping={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
                assert d<.000002;ids.append(i)
            mapping[key]=ids
        anchors.append(mapping)
    targets={}
    for i in skin:
        p=old[i];h=(p.z-middle)/height;w=target_width(h)
        if w is None or abs(p.x)<.035 or (Z0+p.y)/H<=.04:continue
        widths=[]
        for a,b in edges:
            a,b=old[a],old[b]
            if (a.z-p.z)*(b.z-p.z)<=0 and abs(a.z-b.z)>1e-10:
                widths.append(abs(a.x+(b.x-a.x)*(p.z-a.z)/(b.z-a.z)))
        current=max(widths,default=abs(p.x))
        delta=max(-.003,min(.003,w*height-current))
        lateral=max(0.,min(1.,abs(p.x)/max(current,1e-8)))
        depth_fade=max(0.,min(1.,((Z0+p.y)/H-.04)/.08))
        inner_fade=max(0.,min(1.,(abs(p.x)-.035)/.015))
        fade=max(0.,min(1.,(h+.40)/.015,(-.25-h)/.015))
        weight=lateral*depth_fade*depth_fade*(3-2*depth_fade)*inner_fade*inner_fade*(3-2*inner_fade)*fade*fade*(3-2*fade)
        target=p.copy();target.x+=(1 if p.x>0 else -1)*delta*weight
        if (target-p).length>1e-9:targets[i]=target
    normals=[p.normal.copy() for p in data.polygons];weights={i:1. for i in targets}
    for attempt in range(16):
        for i,p in targets.items():data.vertices[i].co=old[i].lerp(p,weights[i])
        data.update();invalid=[p for p,n in zip(data.polygons,normals) if any(i in targets for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
        if not invalid:break
        for i in {i for p in invalid for i in p.vertices if i in targets}:weights[i]*=.5
        if attempt==14:weights={i:0. for i in targets}
    assert not invalid,'Frontal ear-under envelope inverted a face'
    assert all((v.co-old[i]).length<1e-9 for i,v in enumerate(data.vertices) if i not in targets)
    for report,mapping in zip(reports,anchors):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'selected_vertices':len(targets),'maximum_displacement_m':max(((data.vertices[i].co-old[i]).length for i in targets),default=0),'depth_and_height_fixed':True,'normalized_height_range':[-.40,-.25]}

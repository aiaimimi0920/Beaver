"""Moving quadratic skin-section fit, preserving other geometry and welded roots."""

def solve(a,b):
    a=[list(row)+[rhs] for row,rhs in zip(a,b)];n=len(b)
    for i in range(n):
        pivot=max(range(i,n),key=lambda j:abs(a[j][i]))
        if abs(a[pivot][i])<1e-12:return None
        a[i],a[pivot]=a[pivot],a[i];d=a[i][i];a[i]=[x/d for x in a[i]]
        for j in range(n):
            if j!=i:
                factor=a[j][i];a[j]=[x-factor*y for x,y in zip(a[j],a[i])]
    return [row[-1] for row in a]

def quadratic_intercept(samples):
    # Coordinates are normalized by the local support radius before fitting.
    a=[[0.]*6 for _ in range(6)];b=[0.]*6
    for u,v,x,w in samples:
        f=[1.,u,v,u*u,u*v,v*v]
        for i in range(6):
            b[i]+=w*f[i]*x
            for j in range(6):a[i][j]+=w*f[i]*f[j]
    for i in range(6):a[i][i]+=1e-9
    answer=solve(a,b);return answer[0] if answer is not None else None

def fair_skin_sections(data,reports):
    from face import H,Y0,Z0
    from math import exp
    data.update();old=[v.co.copy() for v in data.vertices]
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    tree=KDTree(len(old))
    for i,p in enumerate(old):tree.insert(p,i)
    tree.balance();anchors=[]
    for report in reports:
        mapping={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]];ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)));assert d<.000002;ids.append(i)
            mapping[key]=ids
        anchors.append(mapping)
    regions=data.attributes['NormalRegion'];membership={};uses={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for i in ids:membership.setdefault(i,set()).add(regions.data[poly.index].value)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            edge=tuple(sorted((a,b)));uses[edge]=uses.get(edge,0)+1
    boundary={i for edge,n in uses.items() if n==1 for i in edge}
    skin={i for i,p in enumerate(old) if membership.get(i) and bool(membership[i]&{1,2}) and abs(p.x)>.035}
    def smooth(t):
        t=max(0.,min(1.,t));return t*t*(3-2*t)
    targets={};radius=.018
    for i in skin-boundary:
        p=old[i];h=(p.z+.021-Y0)/H;back=(Z0+p.y)/H
        if not .13<h<.39 or back<=.04:continue
        samples=[]
        for j in skin:
            q=old[j]
            if p.x*q.x<0:continue
            u=(q.y-p.y)/radius;v=(q.z-p.z)/radius
            if u*u+v*v>1:continue
            samples.append((u,v,abs(q.x),exp(-3*(u*u+v*v))))
        if len(samples)<12:continue
        x=quadratic_intercept(samples)
        if x is None:continue
        weight=smooth((h-.13)/.05)*smooth((.39-h)/.05)*smooth((back-.04)/.04)
        delta=max(-.0015,min(.0015,x-abs(p.x)))*weight
        q=p.copy();q.x+=(1 if p.x>0 else -1)*delta
        if (q-p).length>1e-9:targets[i]=q
    normals=[p.normal.copy() for p in data.polygons];weights={i:1. for i in targets}
    for attempt in range(16):
        for i,q in targets.items():data.vertices[i].co=old[i].lerp(q,weights[i])
        data.update();invalid=[p for p,n in zip(data.polygons,normals) if any(i in targets for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
        if not invalid:break
        for i in {i for p in invalid for i in p.vertices if i in targets}:weights[i]*=.5
        if attempt==14:weights={i:0. for i in targets}
    assert not invalid,'Local quadratic skin fit inverted a face'
    assert all((v.co-old[i]).length<1e-9 for i,v in enumerate(data.vertices) if i not in targets)
    for report,mapping in zip(reports,anchors):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'selected_vertices':len(targets),'maximum_displacement_m':max(((data.vertices[i].co-old[i]).length for i in targets),default=0),'depth_and_height_fixed':True,'support_radius_m':radius}

"""Semantic ear-under shoulder depth surface; keep welded topology and other parts fixed."""
ROWS=[[-0.4, 0.1994, [0.1385, 0.1303, 0.1218, 0.1125, 0.0977, 0.0803]], [-0.375, 0.2285, [0.1436, 0.1351, 0.1254, 0.117, 0.1039, 0.0847]], [-0.35, 0.2527, [0.1492, 0.1399, 0.1287, 0.1201, 0.1049, 0.0844]], [-0.325, 0.2714, [0.1537, 0.1441, 0.1315, 0.1223, 0.1048, 0.081]], [-0.3, 0.2846, [0.1574, 0.1471, 0.1337, 0.1246, 0.1065, 0.0831]], [-0.275, 0.2967, [0.1577, 0.1467, 0.1315, 0.1224, 0.1057, 0.0805]], [-0.25, 0.3074, [0.1546, 0.1434, 0.1282, 0.1184, 0.1014, 0.0716]], [-0.225, 0.317, [0.1496, 0.138, 0.1226, 0.1123, 0.0961, 0.0622]], [-0.2, 0.327, [0.1431, 0.1307, 0.115, 0.1048, 0.089, 0.0298]], [-0.175, 0.337, [0.1338, 0.1203, 0.1052, 0.0955, 0.0757, 0.0084]], [-0.15, 0.347, [0.1152, 0.1073, 0.0939, 0.0848, 0.063, -0.0411]]]
RATIOS=[0.65, 0.75, 0.85, 0.9, 0.95, 0.98]

def smooth(t):
    t=max(0.,min(1.,t));return t*t*(3-2*t)

def interpolate(x,knots,values):
    if x<=knots[0]:return values[0]
    if x>=knots[-1]:return values[-1]
    for a,b,va,vb in zip(knots,knots[1:],values,values[1:]):
        if a<=x<=b:return va+(vb-va)*(x-a)/(b-a)

def shoulder_depth(h,x):
    width=interpolate(h,[r[0] for r in ROWS],[r[1] for r in ROWS])
    ratio=x/width
    depths=[interpolate(ratio,RATIOS,r[2]) for r in ROWS]
    return interpolate(h,[r[0] for r in ROWS],depths),ratio

def fit_ear_under_shoulder(data,reports):
    from face import H,Y0,Z0
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    data.update();old=[v.co.copy() for v in data.vertices]
    height=max(p.z for p in old)-min(p.z for p in old)
    middle=(max(p.z for p in old)+min(p.z for p in old))*.5
    middepth=(max(p.y for p in old)+min(p.y for p in old))*.5
    regions=data.attributes['NormalRegion'];membership={}
    for poly in data.polygons:
        for i in poly.vertices:membership.setdefault(i,set()).add(regions.data[poly.index].value)
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
    targets={}
    for i,p in enumerate(old):
        h=(p.z-middle)/height;back=(Z0+p.y)/H
        if not -.40<h<-.15 or back<=.04 or abs(p.x)<=.035 or not membership.get(i,set()).issubset({1,2}):continue
        target,ratio=shoulder_depth(h,abs(p.x)/height)
        front=-(p.y-middepth)/height
        weight=smooth((h+.40)/.05)*smooth((-.15-h)/.05)*smooth((ratio-.65)/.2)*smooth((1.06-ratio)/.08)*smooth((front+.05)/.08)
        delta=max(-.005,min(.005,(target-front)*height))
        if delta>0:delta=min(delta,(back-.04)*H*.5)
        q=p.copy();q.y-=delta*weight
        if (q-p).length>1e-9:targets[i]=q
    normals=[p.normal.copy() for p in data.polygons];weights={i:1. for i in targets}
    for attempt in range(16):
        for i,q in targets.items():data.vertices[i].co=old[i].lerp(q,weights[i])
        data.update();invalid=[p for p,n in zip(data.polygons,normals) if any(i in targets for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
        if not invalid:break
        for i in {i for p in invalid for i in p.vertices if i in targets}:weights[i]*=.5
        if attempt==14:weights={i:0. for i in targets}
    assert not invalid,'Ear-under shoulder inverted a face'
    assert all((v.co-old[i]).length<1e-9 for i,v in enumerate(data.vertices) if i not in targets)
    for report,mapping in zip(reports,anchors):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'selected_vertices':len(targets),'maximum_displacement_m':max(((data.vertices[i].co-old[i]).length for i in targets),default=0),'width_and_height_fixed':True}

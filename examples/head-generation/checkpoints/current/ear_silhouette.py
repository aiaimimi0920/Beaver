"""Bounded depth-only ear silhouette calibration; shared facial roots are pinned."""
def bump(t):
    t=abs(t)
    return (1-t*t)**3 if t<1 else 0.0

def silhouette_depth(height):
    radius=.015 if height>1.607 else .027
    return .0025*bump((height-1.607)/radius)-.0008*bump((height-1.641)/.014)

def fit_ear_silhouette(data,reports):
    data.update()
    old=[v.co.copy() for v in data.vertices]
    normals=[p.normal.copy() for p in data.polygons]
    regions=data.attributes['NormalRegion']
    ear_ids=set();other_ids=set()
    for p in data.polygons:
        target=ear_ids if regions.data[p.index].value in (47,48,147,148,157,158) else other_ids
        target.update(p.vertices)
    pinned=ear_ids & other_ids
    changed=set()
    for i in ear_ids-pinned:
        v=data.vertices[i];roots=reports[0 if v.co.x<0 else 1]['root_positions']
        z=v.co.z+.021
        j=next((j for j in range(len(roots)-1) if z<=roots[j+1][2]),len(roots)-2)
        a,b=roots[j:j+2];t=max(0,min(1,(z-a[2])/(b[2]-a[2])))
        root_x=abs(a[0]+t*(b[0]-a[0]))
        w=max(0,min(1,(abs(v.co.x)-root_x)/.008));w=w*w*(3-2*w)
        delta=silhouette_depth(v.co.z)*w
        if abs(delta)>1e-12:v.co.y+=delta;changed.add(i)
    data.update()
    invalid=[p.index for p,n in zip(data.polygons,normals) if changed.intersection(p.vertices) and (p.area<1e-12 or p.normal.dot(n)<=0)]
    assert not invalid,('Ear silhouette inverted face',invalid)
    assert all((data.vertices[i].co-old[i]).length<1e-10 for i in pinned)
    return {'changed_vertices':len(changed),'max_displacement_m':max(((data.vertices[i].co-old[i]).length for i in changed),default=0),'shared_root_vertices_pinned':len(pinned),'depth_only':True,'topology_unchanged':True}


def upper_join_offset(point,anchor):
    x,y,z=point;ax,ay,az=anchor
    root=-.0048*bump((abs(x)-abs(ax))/.026)*bump((y-ay)/.017)*bump((z-az)/.018)
    crown=.0022*bump((abs(x)-abs(ax))/.020)*bump(((y-ay)-.009)/.008)*bump((z-az)/.015)
    return root+crown

def fit_upper_join(data,reports):
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    data.update();old=[v.co.copy() for v in data.vertices];normals=[p.normal.copy() for p in data.polygons]
    tree=KDTree(len(old))
    for i,p in enumerate(old):tree.insert(p,i)
    tree.balance();maps=[];anchors=[]
    for report in reports:
        mapping={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]];ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)));assert d<.000002;ids.append(i)
            mapping[key]=ids
        maps.append(mapping);anchors.append(old[mapping['root_positions'][-1]])
    changed=set()
    for v in data.vertices:
        a=anchors[0 if v.co.x<0 else 1];d=upper_join_offset(v.co,a)
        if abs(d)>1e-12:v.co.z+=d;changed.add(v.index)
    data.update()
    bad=[p.index for p,n in zip(data.polygons,normals) if changed.intersection(p.vertices) and (p.area<1e-12 or p.normal.dot(n)<=0)]
    assert not bad,('Upper-ear junction inverted face',[(i,[list(old[j]) for j in data.polygons[i].vertices],[list(data.vertices[j].co) for j in data.polygons[i].vertices]) for i in bad])
    for report,mapping in zip(reports,maps):
        for key,ids in mapping.items():
            points=[[data.vertices[i].co.x,data.vertices[i].co.y,data.vertices[i].co.z+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    return {'changed_vertices':len(changed),'maximum_displacement_m':max(((data.vertices[i].co-old[i]).length for i in changed),default=0),'shared_root_moved_together':True,'topology_unchanged':True}

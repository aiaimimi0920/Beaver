"""Bounded, topology-preserving fairing of the lower open ear-root rim."""

def fair_chain(points, neighbors, selected, iterations=12, strength=0.45, limit=0.004, blend_rings=0):
    from math import sqrt
    original=[tuple(p) for p in points]
    result=list(original)
    for _ in range(iterations):
        old=list(result)
        for i in selected:
            if len(neighbors.get(i,[])) != 2:
                continue
            a,b=neighbors[i]
            da=sqrt(sum((old[a][k]-old[i][k])**2 for k in range(3)))
            db=sqrt(sum((old[b][k]-old[i][k])**2 for k in range(3)))
            if da+db < 1e-10: continue
            target=tuple((old[a][k]*db+old[b][k]*da)/(da+db) for k in range(3))
            proposed=tuple(old[i][k]+strength*(target[k]-old[i][k]) for k in range(3))
            d=sqrt(sum((proposed[k]-original[i][k])**2 for k in range(3)))
            factor=min(1,limit/max(d,1e-12))
            result[i]=tuple(original[i][k]+factor*(proposed[k]-original[i][k]) for k in range(3))
    if blend_rings:
        selected=set(selected)
        distance={i:1 for i in selected if any(j not in selected for j in neighbors.get(i,[]))}
        for step in range(2,blend_rings+1):
            for i in selected-set(distance):
                if any(distance.get(j)==step-1 for j in neighbors.get(i,[])):
                    distance[i]=step
        for i in selected:
            t=min(1.0,distance.get(i,blend_rings)/blend_rings)
            weight=t*t*(3-2*t)
            result[i]=tuple(original[i][k]+weight*(result[i][k]-original[i][k]) for k in range(3))
    return result

def fair_lower_ear_boundaries(data,reports):
    from mathutils import Vector
    from face import C
    from mathutils.kdtree import KDTree
    data.update()
    tree=KDTree(len(data.vertices))
    for v in data.vertices:tree.insert(v.co,v.index)
    tree.balance()
    locked=set()
    for report in reports:
        for p in report['root_positions']:
            _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
            assert d<.000002
            locked.add(i)
    uses={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            edge=tuple(sorted((a,b)));uses[edge]=uses.get(edge,0)+1
    neighbors={}
    for (a,b),count in uses.items():
        if count==1:
            neighbors.setdefault(a,[]).append(b);neighbors.setdefault(b,[]).append(a)
    selected=set();seeds=[]
    for report in reports:
        p=report['lower_bridge_root_position']
        _,seed,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
        assert d<.000002 and len(neighbors.get(seed,[]))==2
        seeds.append(seed);front={seed};region={seed}
        for _ in range(C["ear_lower_fairing_neighbor_rings"]):
            front={j for i in front for j in neighbors.get(i,[]) if j not in locked}
            region.update(front)
        selected.update(i for i in region if i not in locked)
    original=[tuple(v.co) for v in data.vertices]
    old_normals=[p.normal.copy() for p in data.polygons]
    result=fair_chain(original,neighbors,selected,blend_rings=C["ear_lower_fairing_blend_rings"])
    for i in selected:data.vertices[i].co=result[i]
    data.update()
    for p,n in zip(data.polygons,old_normals):
        if any(i in selected for i in p.vertices):
            assert p.area>1e-12 and p.normal.dot(n)>0, 'Boundary fairing inverted or collapsed a face'
    assert all(tuple(data.vertices[i].co)==original[i] for i in locked)
    for report,i in zip(reports,seeds):
        p=data.vertices[i].co
        report['lower_bridge_root_position']=[p.x,p.y,p.z+.021]
    maximum=max((Vector(result[i])-Vector(original[i])).length for i in selected)
    assert maximum<=.004001
    return {'selected_vertices':len(selected),'maximum_displacement_m':maximum,'shared_root_vertices_locked':len(locked),'topology_unchanged':True}


def support_offset(v, q, amplitude):
    height=max(0.0,1.0-abs((v-.26)/.08))
    lateral=max(0.0,min(1.0,(q-.90)/.10))
    return amplitude*height*height*(3-2*height)*lateral*lateral*(3-2*lateral)

def fit_final_ear_support(data,reports):
    """Final, compact deformation; never feed this back through global smoothing."""
    from face import C,H,Y0,width
    from mathutils import Vector
    def shift(p):
        x,y,z=p; original_height=z+.021
        v=(original_height-Y0)/H
        q=abs(x)/max(width(original_height),1e-8)
        return (x,y+support_offset(v,q,C['ear_root_posterior_support_m']),z)
    data.update()
    before=[tuple(v.co) for v in data.vertices]
    old_normals=[p.normal.copy() for p in data.polygons]
    changed=set()
    for vertex,p in zip(data.vertices,before):
        target=shift(p)
        if target!=p:
            vertex.co=target; changed.add(vertex.index)
    data.update()
    for poly,normal in zip(data.polygons,old_normals):
        if any(i in changed for i in poly.vertices):
            assert poly.area>1e-12 and poly.normal.dot(normal)>0,'Ear support inverted a face'
    for report in reports:
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            shifted=[]
            for p in points:
                x,y,z=shift((p[0],p[1],p[2]-.021));shifted.append([x,y,z+.021])
            report[key]=shifted if key=='root_positions' else shifted[0]
    maximum=max([(Vector(data.vertices[i].co)-Vector(before[i])).length for i in changed] or [0])
    assert maximum<=C['ear_root_posterior_support_m']+1e-7
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    return {'changed_vertices':len(changed),'maximum_displacement_m':maximum,'final_stage':True,'compact_height_range':[.18,.34],'minimum_lateral_fraction':.9}


def rear_curve_offset(v, back_depth, amplitude):
    # An asymmetric arc returns gradually through the lower ear attachment.
    # A short symmetric falloff creates an inward kink immediately below the ear.
    t=max(0.0,min(1.0,(v-.035)/.075)) if v<=.11 else max(0.0,min(1.0,(.38-v)/.27))
    arch=t*t*(3-2*t)
    depth=max(0.0,min(1.0,(back_depth-.18)/.05))
    return amplitude*arch*depth*depth*(3-2*depth)

def fit_ear_to_jaw_curve(data,reports):
    """Round the annotated posterior skin silhouette; other components stay fixed."""
    from face import C,H,Y0,Z0
    regions=data.attributes['NormalRegion']
    skin={i for p in data.polygons if regions.data[p.index].value in (1,2) for i in p.vertices}
    # Include the connected ear membrane, excluding disconnected mouth and eye parts.
    neighbors={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            neighbors.setdefault(a,set()).add(b);neighbors.setdefault(b,set()).add(a)
    pending=list(skin)
    while pending:
        for j in neighbors.get(pending.pop(),()):
            if j not in skin:skin.add(j);pending.append(j)
    data.update();before=[tuple(v.co) for v in data.vertices]
    normals=[p.normal.copy() for p in data.polygons];changed=set();maximum=0
    for i in skin:
        x,y,z=before[i]
        height=(z+.021-Y0)/H
        back_depth=(Z0+y)/H
        d=rear_curve_offset(height,back_depth,C['ear_to_jaw_rear_curve_m'])
        if d>0:
            data.vertices[i].co.y+=d;changed.add(i);maximum=max(maximum,d)
    data.update()
    for p,n in zip(data.polygons,normals):
        if any(i in changed for i in p.vertices):
            assert p.area>1e-12 and p.normal.dot(n)>0,'Rear contour inverted or collapsed a face'
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    for report in reports:
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            shifted=[]
            for p in points:
                d=rear_curve_offset((p[2]-Y0)/H,(Z0+p[1])/H,C['ear_to_jaw_rear_curve_m'])
                shifted.append([p[0],p[1]+d,p[2]])
            report[key]=shifted if key=='root_positions' else shifted[0]
    return {'changed_vertices':len(changed),'maximum_displacement_m':maximum,'height_range':[.035,.38],'posterior_depth_start':.18,'skin_only':True,'front_and_other_parts_unchanged':True}


def fair_ear_jaw_rim(data,reports):
    """Remove the residual rear-view S bend with bounded 3D boundary fairing."""
    from face import C,H,Y0,Z0
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    data.update()
    before=[tuple(v.co) for v in data.vertices]
    normals=[p.normal.copy() for p in data.polygons]
    uses={};all_neighbors={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            edge=tuple(sorted((a,b)));uses[edge]=uses.get(edge,0)+1
            all_neighbors.setdefault(a,set()).add(b);all_neighbors.setdefault(b,set()).add(a)
    regions=data.attributes['NormalRegion']
    skin={i for p in data.polygons if regions.data[p.index].value in (1,2) for i in p.vertices}
    pending=list(skin)
    while pending:
        for j in all_neighbors.get(pending.pop(),()):
            if j not in skin:skin.add(j);pending.append(j)
    neighbors={}
    for (a,b),count in uses.items():
        if count==1 and a in skin and b in skin:
            neighbors.setdefault(a,[]).append(b);neighbors.setdefault(b,[]).append(a)
    selected={i for i in neighbors if .06<(before[i][2]+.021-Y0)/H<.38 and (Z0+before[i][1])/H>.18}
    tree=KDTree(len(before))
    for i,p in enumerate(before):tree.insert(Vector(p),i)
    tree.balance()
    anchors=[]
    for report in reports:
        entries={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            ids=[]
            for p in points:
                _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
                assert d<.000002;ids.append(i)
            entries[key]=ids
        anchors.append(entries)
    result=fair_chain(before,neighbors,selected,iterations=C['ear_jaw_rim_fairing_iterations'],limit=C['ear_jaw_rim_fairing_limit_m'],blend_rings=3)
    # Blend the moved rim through the permitted adjacent skin patch to avoid stretched strips.
    boundary=set(neighbors)
    eligible={i for i in skin-boundary if .06<(before[i][2]+.021-Y0)/H<.38 and (Z0+before[i][1])/H>.18}
    result,interior=blend_rim_displacements(before,result,all_neighbors,selected,eligible)
    changed=selected|interior
    for i in changed:data.vertices[i].co=result[i]
    data.update()
    for poly,n in zip(data.polygons,normals):
        if any(i in changed for i in poly.vertices):
            assert poly.area>1e-12 and poly.normal.dot(n)>0,'Ear-jaw rim fairing inverted a face'
    for report,entries in zip(reports,anchors):
        for key,ids in entries.items():
            points=[[result[i][0],result[i][1],result[i][2]+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    return {'interior_blend_vertices':len(interior),'selected_vertices':len(selected),'maximum_displacement_m':max((Vector(result[i])-Vector(before[i])).length for i in selected),'scope':'connected skin free boundary only','height_range':[.06,.38],'topology_unchanged':True}


def blend_rim_displacements(before,result,neighbors,boundary,eligible):
    """Harmonic skin extension with fixed rim and zero displacement outside scope."""
    from math import sqrt
    delta={i:tuple(result[i][k]-before[i][k] for k in range(3)) for i in boundary}
    weights={}
    for i in eligible:
        linked=list(neighbors.get(i,()))
        weights[i]=[(j,1/max(1e-7,sqrt(sum((before[j][k]-before[i][k])**2 for k in range(3))))) for j in linked]
        delta[i]=(0.,0.,0.)
    for _ in range(240):
        old=dict(delta);largest=0
        for i in eligible:
            linked=weights[i];total=sum(w for _,w in linked)
            if not total:continue
            d=tuple(sum(old.get(j,(0.,0.,0.))[k]*w for j,w in linked)/total for k in range(3))
            largest=max(largest,max(abs(d[k]-old[i][k]) for k in range(3)));delta[i]=d
        if largest<1e-9:break
    output=list(result)
    for i in eligible:output[i]=tuple(before[i][k]+delta[i][k] for k in range(3))
    return output,set(eligible)


def opening_lateral_offset(v, amplitude):
    t=max(0.,min(1.,(v-.04)/.07 if v<=.11 else (.25-v)/.14))
    return amplitude*t*t*(3-2*t)

def fit_opening_lateral_arc(data,reports):
    """Round the lower posterior opening in projection without moving ear roots."""
    from face import C,H,Y0,Z0
    from mathutils import Vector
    data.update();before=[tuple(v.co) for v in data.vertices]
    normals=[p.normal.copy() for p in data.polygons]
    uses={};neighbors={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            key=tuple(sorted((a,b)));uses[key]=uses.get(key,0)+1
            neighbors.setdefault(a,set()).add(b);neighbors.setdefault(b,set()).add(a)
    regions=data.attributes['NormalRegion']
    skin={i for p in data.polygons if regions.data[p.index].value in (1,2) for i in p.vertices}
    pending=list(skin)
    while pending:
        for j in neighbors.get(pending.pop(),()):
            if j not in skin:skin.add(j);pending.append(j)
    anchors=[]
    for report in reports:
        entries={}
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            ids=[]
            for p in points:
                target=Vector((p[0],p[1],p[2]-.021))
                idx=min(range(len(before)),key=lambda i:(Vector(before[i])-target).length_squared)
                assert (Vector(before[idx])-target).length<.000002
                ids.append(idx)
            entries[key]=ids
        anchors.append(entries)
    boundary={i for edge,n in uses.items() if n==1 for i in edge if i in skin}
    def allowed(i):
        x,y,z=before[i]
        return abs(x)>.003 and .04<(z+.021-Y0)/H<.25 and (Z0+y)/H>.18
    chosen={i for i in boundary if allowed(i)}
    eligible={i for i in skin-boundary if allowed(i)}
    result=list(before)
    for i in chosen:
        x,y,z=before[i];v=(z+.021-Y0)/H
        d=opening_lateral_offset(v,C['ear_jaw_opening_lateral_m'])
        result[i]=(x+(d if x>0 else -d),y,z)
    result,interior=blend_rim_displacements(before,result,neighbors,chosen,eligible)
    changed=chosen|interior
    for i in changed:data.vertices[i].co=result[i]
    data.update()
    for poly,n in zip(data.polygons,normals):
        if any(i in changed for i in poly.vertices):
            assert poly.area>1e-12 and poly.normal.dot(n)>0,'Opening arc inverted a face'
    for report,entries in zip(reports,anchors):
        for key,ids in entries.items():
            points=[[result[i][0],result[i][1],result[i][2]+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    maximum=max([(Vector(result[i])-Vector(before[i])).length for i in changed]or[0])
    assert maximum<=C['ear_jaw_opening_lateral_m']+1e-7
    assert all(tuple(data.vertices[i].co)==q for i,q in enumerate(before) if i not in changed)
    return {'boundary_vertices':len(chosen),'interior_vertices':len(interior),'maximum_displacement_m':maximum,'height_range':[.04,.25],'topology_unchanged':True}

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
    for report in reports:
        ids=[]
        for p in report['lower_bridge_triangle_positions']:
            _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
            assert d<.000002,'Authored bridge vertex missing after welding'
            ids.append(i)
        assert len(set(ids))==3
        report['lower_bridge_vertex_ids']=ids
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
    assert maximum<=abs(C['ear_root_posterior_support_m'])+1e-7
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    return {'changed_vertices':len(changed),'maximum_displacement_m':maximum,'final_stage':True,'compact_height_range':[.18,.34],'minimum_lateral_fraction':.9}


def rear_curve_offset(v, back_depth, amplitude):
    # An asymmetric arc returns gradually through the lower ear attachment.
    # A short symmetric falloff creates an inward kink immediately below the ear.
    t=max(0.0,min(1.0,(v-.025)/.085)) if v<=.11 else max(0.0,min(1.0,(.27-v)/.16))
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
    return {'changed_vertices':len(changed),'maximum_displacement_m':maximum,'height_range':[.025,.27],'posterior_depth_start':.18,'skin_only':True,'front_and_other_parts_unchanged':True}


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
    preserved=fair_chain(before,neighbors,selected,iterations=80,limit=.006,blend_rings=3)
    result=[(preserved[i][0],point[1],preserved[i][2]) for i,point in enumerate(result)]
    # Blend the moved rim through the permitted adjacent skin patch to avoid stretched strips.
    boundary=set(neighbors)
    eligible={i for i in skin-boundary if .06<(before[i][2]+.021-Y0)/H<.38 and (Z0+before[i][1])/H>.05}
    result,interior=blend_rim_displacements(before,result,all_neighbors,selected,eligible)
    result=fair_interior_depth(result,all_neighbors,eligible)
    changed=selected|interior
    safe=list(result)
    proposed=constrain_lower_envelope(before,result,changed)
    envelope_factors={i:1.0 for i in changed}
    rejected_faces=[]
    for attempt in range(25):
        result=[tuple(a[k]+envelope_factors.get(i,0)*(b[k]-a[k]) for k in range(3)) for i,(a,b) in enumerate(zip(safe,proposed))]
        for i in changed:data.vertices[i].co=result[i]
        data.update()
        invalid=[poly.index for poly,n in zip(data.polygons,normals) if any(i in changed for i in poly.vertices) and (poly.area<=1e-12 or poly.normal.dot(n)<=0)]
        if not invalid:break
        rejected_faces.append({'iteration':attempt,'faces':invalid})
        affected={i for index in invalid for i in data.polygons[index].vertices if i in changed}
        for i in affected:envelope_factors[i]*=.5
        if attempt==23:envelope_factors={i:0.0 for i in changed}
    opening_safe=list(result)
    opening_proposed=calibrate_opening_sweep(before,result,selected,C)
    opening_proposed,_=blend_rim_displacements(result,opening_proposed,all_neighbors,selected,eligible)
    # Preserve each small bridge coherently before independent vertex backtracking.
    for i in changed:data.vertices[i].co=opening_proposed[i]
    data.update()
    initial_invalid=[p.index for p,n in zip(data.polygons,normals) if any(i in changed for i in p.vertices) and (p.area<=1e-12 or p.normal.dot(n)<=0)]
    bridge_roots={i for report in reports for i in report['lower_bridge_vertex_ids']}
    from bridge_pairs import paired_bridge_indices
    candidates={p.index:list(p.vertices) for p in data.polygons if len(p.vertices)==3 and 40<=regions.data[p.index].value<50 and bridge_roots.intersection(p.vertices)}
    initial_bridge_indices=paired_bridge_indices(before,candidates,[i for i in initial_invalid if i in candidates])
    initial_bridge_groups=[candidates[i] for i in initial_bridge_indices]
    assert len(initial_bridge_groups)==2, 'Expected two initial constrained bridges'
    bridge_ids_initial={i for g in initial_bridge_groups for i in g}
    coherent=coherent_bridge_targets(before,opening_proposed,initial_bridge_groups)
    opening_proposed,_=blend_rim_displacements(opening_proposed,coherent,all_neighbors,bridge_ids_initial,eligible-bridge_ids_initial)
    for i in changed:data.vertices[i].co=opening_safe[i]
    data.update()
    opening_factors={i:1.0 for i in changed}
    opening_rejected=[]
    for attempt in range(25):
        result=[tuple(a[k]+opening_factors.get(i,0)*(b[k]-a[k]) for k in range(3)) for i,(a,b) in enumerate(zip(opening_safe,opening_proposed))]
        for i in changed:data.vertices[i].co=result[i]
        data.update()
        invalid=[poly.index for poly,n in zip(data.polygons,normals) if any(i in changed for i in poly.vertices) and (poly.area<=1e-12 or poly.normal.dot(n)<=0)]
        if not invalid:break
        opening_rejected.append({'iteration':attempt,'faces':invalid})
        affected={i for index in invalid for i in data.polygons[index].vertices if i in changed}
        for i in affected:opening_factors[i]*=.5
        if attempt==23:opening_factors={i:0.0 for i in changed}
    opening_factor=min(opening_factors.values(),default=0.)
    bridge_roots={i for report in reports for i in report['lower_bridge_vertex_ids']}
    limiting_faces={i for step in opening_rejected for i in step['faces']} | set(initial_invalid) | set(initial_bridge_indices)
    bridge_groups=[list(data.polygons[i].vertices) for i in sorted(limiting_faces) if len(data.polygons[i].vertices)==3 and 40<=regions.data[i].value<50 and bridge_roots.intersection(data.polygons[i].vertices)]
    assert len(bridge_groups)==2,('Expected symmetric constrained ear-root faces',bridge_groups,sorted(bridge_roots),[(i,list(data.polygons[i].vertices),regions.data[i].value) for i in sorted(limiting_faces)])
    bridge_ids={i for group in bridge_groups for i in group}
    assert bridge_ids.issubset(changed),'Bridge outside authorized local support'
    bridge_safe=list(result)
    bridge_proposed=coherent_bridge_targets(before,result,bridge_groups)
    bridge_proposed,_=blend_rim_displacements(result,bridge_proposed,all_neighbors,bridge_ids,eligible-bridge_ids)
    bridge_factor=1.
    for attempt in range(13):
        result=[tuple(a[k]+bridge_factor*(b[k]-a[k]) for k in range(3)) if i in changed else a for i,(a,b) in enumerate(zip(bridge_safe,bridge_proposed))]
        for i in changed:data.vertices[i].co=result[i]
        data.update()
        invalid=[poly.index for poly,n in zip(data.polygons,normals) if any(i in changed for i in poly.vertices) and (poly.area<=1e-12 or poly.normal.dot(n)<=0)]
        if not invalid:break
        bridge_factor=bridge_factor*.5 if attempt<11 else 0.
    envelope_factor=min(envelope_factors.values(),default=0.0)
    for poly,n in zip(data.polygons,normals):
        if any(i in changed for i in poly.vertices):
            assert poly.area>1e-12 and poly.normal.dot(n)>0,'Ear-jaw rim fairing inverted a face'
    for report,entries in zip(reports,anchors):
        for key,ids in entries.items():
            points=[[result[i][0],result[i][1],result[i][2]+.021] for i in ids]
            report[key]=points if key=='root_positions' else points[0]
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    return {'opening_initial_invalid':initial_invalid,'bridge_preconditioned':True,'bridge_shape_factor':bridge_factor,'bridge_groups':bridge_groups,'opening_constraint_faces':[{'index':i,'vertices':list(data.polygons[i].vertices),'original':[before[v] for v in data.polygons[i].vertices],'final':[result[v] for v in data.polygons[i].vertices],'region':regions.data[i].value} for i in sorted({i for step in opening_rejected for i in step['faces']})],'opening_rejected_faces':opening_rejected,'opening_full_weight_vertices':sum(f==1 for f in opening_factors.values()),'opening_factor':opening_factor,'envelope_factor':envelope_factor,'envelope_full_weight_vertices':sum(f==1 for f in envelope_factors.values()),'rejected_envelope_faces':rejected_faces,'interior_blend_vertices':len(interior),'selected_vertices':len(selected),'maximum_displacement_m':max((Vector(result[i])-Vector(before[i])).length for i in selected),'scope':'connected skin free boundary only','height_range':[.06,.38],'topology_unchanged':True}


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


def under_ear_lateral_offset(height, back_depth, lateral, amplitude):
    t=max(0.,min(1.,(height-.13)/.09 if height<=.22 else (.36-height)/.14))
    z=max(0.,min(1.,(back_depth-.05)/.07))
    x=max(0.,min(1.,(lateral-.025)/.035))
    return amplitude*t*t*(3-2*t)*z*z*(3-2*z)*x*x*(3-2*x)


def fit_ear_under_width(data,reports):
    from face import C,H,Y0,Z0
    regions=data.attributes['NormalRegion']
    skin={i for p in data.polygons if regions.data[p.index].value in (1,2) for i in p.vertices}
    neighbors={}
    for poly in data.polygons:
        ids=list(poly.vertices)
        for a,b in zip(ids,ids[1:]+ids[:1]):
            neighbors.setdefault(a,set()).add(b);neighbors.setdefault(b,set()).add(a)
    pending=list(skin)
    while pending:
        for j in neighbors.get(pending.pop(),()):
            if j not in skin:skin.add(j);pending.append(j)
    def shift(p):
        x,y,z=p
        d=under_ear_lateral_offset((z+.021-Y0)/H,(Z0+y)/H,abs(x),C['ear_under_lateral_width_m'])
        return (x+(d if x>=0 else -d),y,z)
    data.update();before=[tuple(v.co) for v in data.vertices]
    normals=[p.normal.copy() for p in data.polygons];changed=set()
    for i in skin:
        target=shift(before[i])
        if target!=before[i]:data.vertices[i].co=target;changed.add(i)
    data.update()
    for p,n in zip(data.polygons,normals):
        if any(i in changed for i in p.vertices):
            assert p.area>1e-12 and p.normal.dot(n)>0,'Under-ear width inverted a face'
    assert all(tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed)
    for report in reports:
        for key in ['root_positions','lower_bridge_root_position']:
            points=report[key] if key=='root_positions' else [report[key]]
            shifted=[]
            for x,y,z in points:
                a,b,c=shift((x,y,z-.021));shifted.append([a,b,c+.021])
            report[key]=shifted if key=='root_positions' else shifted[0]
    return {'changed_vertices':len(changed),'height_support':[.13,.36],'maximum_width_m':C['ear_under_lateral_width_m'],'posterior_only':True}


def fair_interior_depth(points,neighbors,eligible,iterations=60,strength=.25,limit=.003):
    """Smooth actual depth only, with every boundary and other coordinate fixed."""
    from math import sqrt
    original=[tuple(p) for p in points];result=list(original)
    weights={i:[(j,1/max(1e-7,sqrt(sum((original[i][k]-original[j][k])**2 for k in range(3))))) for j in neighbors.get(i,())] for i in eligible}
    for _ in range(iterations):
        old=list(result)
        for i in eligible:
            linked=weights[i];total=sum(w for _,w in linked)
            if not total:continue
            target=sum(old[j][1]*w for j,w in linked)/total
            depth=old[i][1]+strength*(target-old[i][1])
            depth=max(original[i][1]-limit,min(original[i][1]+limit,depth))
            result[i]=(original[i][0],depth,original[i][2])
    return result


def constrain_lower_envelope(before,points,selected,limit=.003):
    """Bounded semantic silhouette envelope; normalized aggregate support, no mesh dependency."""
    from math import cos,sin,radians,sqrt
    profiles = [[-0.4, [0.1994, 0.2001, 0.1961, 0.1876, 0.1761, 0.1574, 0.1281]], [-0.375, [0.2285, 0.2334, 0.2311, 0.223, 0.2072, 0.1821, 0.1446]], [-0.35, [0.2527, 0.26, 0.2601, 0.2511, 0.2323, 0.2021, 0.1581]], [-0.325, [0.2714, 0.2817, 0.2841, 0.2732, 0.2542, 0.2197, 0.1702]], [-0.3, [0.2846, 0.2986, 0.3047, 0.2947, 0.275, 0.2365, 0.1819]], [-0.275, [0.2967, 0.315, 0.3242, 0.3133, 0.2918, 0.2504, 0.192]], [-0.25, [0.3074, 0.331, 0.3422, 0.3319, 0.3086, 0.2644, 0.2021]], [-0.225, [0.3472, 0.3914, 0.409, 0.3986, 0.3611, 0.299, 0.2166]], [-0.2, [0.3761, 0.4262, 0.4472, 0.4378, 0.3985, 0.3321, 0.2441]], [-0.175, [0.3911, 0.4445, 0.4676, 0.4589, 0.4188, 0.3502, 0.2596]], [-0.15, [0.4062, 0.4629, 0.488, 0.4799, 0.4391, 0.3684, 0.2751]], [-0.125, [0.4209, 0.4797, 0.5057, 0.4973, 0.455, 0.382, 0.2864]], [-0.1, [0.4357, 0.4962, 0.523, 0.5141, 0.4702, 0.3942, 0.2928]]]
    height=max(p[2] for p in before)-min(p[2] for p in before)
    midheight=(max(p[2] for p in before)+min(p[2] for p in before))*.5
    middepth=(max(p[1] for p in before)+min(p[1] for p in before))*.5
    output=list(points)
    for i in selected:
        p=points[i];v=(p[2]-midheight)/height
        if v<=-.4 or v>=-.225:continue
        q=[abs(p[0])/height,-(p[1]-middepth)/height]
        source=list(q)
        band=next((j for j in range(len(profiles)-1) if profiles[j][0]<=v<=profiles[j+1][0]),None)
        if band is None:continue
        t=(v-profiles[band][0])/(profiles[band+1][0]-profiles[band][0])
        supports=[a*(1-t)+b*t for a,b in zip(profiles[band][1],profiles[band+1][1])]
        for iteration in range(8):
            for angle,support in zip(range(0,91,15),supports):
                nx,nz=cos(radians(angle)),-sin(radians(angle))
                excess=q[0]*nx+q[1]*nz-support-.001
                if excess>0:q[0]-=nx*excess;q[1]-=nz*excess
        weight=max(0,min(1,(v+.4)/.025,(-.225-v)/.025));weight=weight*weight*(3-2*weight)
        dx,dz=(q[0]-source[0])*height,(q[1]-source[1])*height
        factor=min(1,limit/max(sqrt(dx*dx+dz*dz),1e-12))*weight
        output[i]=(p[0]+(dx if p[0]>=0 else -dx)*factor,p[1]-dz*factor,p[2])
    return output


def calibrate_opening_sweep(before,points,selected,config,limit=.004):
    """Bounded lower posterior edge sweep in normalized head coordinates."""
    from math import sqrt
    from ear_return import return_weight,lower_return_weight
    profile=[[-.40,.1303,-.1281],[-.375,.1639,-.1446],[-.35,.1907,-.1581],[-.325,.2136,-.1702],[-.30,.2349,-.1819],[-.275,.2511,-.1920],[-.25,.2672,-.2021],[-.225,.2811,-.2125],[-.20,.2932,-.2232],[-.175,.3055,-.2339],[-.15,.3180,-.2445]]
    height=max(p[2] for p in before)-min(p[2] for p in before)
    midheight=(max(p[2] for p in before)+min(p[2] for p in before))*.5
    middepth=(max(p[1] for p in before)+min(p[1] for p in before))*.5
    result=list(points)
    for i in selected:
        x,y,z=points[i];v=(z-midheight)/height
        if not -.40<v<-.15:continue
        for (a,xa,da),(b,xb,db) in zip(profile,profile[1:]):
            if a<=v<=b:
                t=(v-a)/(b-a);target_x=(xa+(xb-xa)*t)*height*(1 if x>=0 else -1);target_y=middepth-height*(da+(db-da)*t);break
        weight=max(0.,min(1.,(v+.40)/.04,(-.15-v)/.04));weight=weight*weight*(3-2*weight)
        authored_height=(z+.021-config['origin_y_m'])/config['height_m']
        upper=return_weight(authored_height);lower=lower_return_weight(authored_height)
        return_width=config['ear_post_return_width_m']*upper+config['ear_post_lower_return_width_m']*lower
        target_x+=(1 if x>=0 else -1)*return_width
        target_y-=config['ear_post_return_depth_m']*upper
        dx,dy=target_x-x,target_y-y;factor=min(1.,limit/max(sqrt(dx*dx+dy*dy),1e-12))*weight
        result[i]=(x+dx*factor,y+dy*factor,z)
    return result


def coherent_bridge_targets(before,points,groups):
    """Keep a small semantic bridge triangle coherent under skin translation."""
    result=list(points)
    for ids in groups:
        delta=tuple(sum(points[i][k]-before[i][k] for i in ids)/len(ids) for k in range(3))
        for i in ids:result[i]=tuple(before[i][k]+delta[k] for k in range(3))
    return result


def refine_local_skin(data):
    """Split overly long interior skin edges without moving the welded rim."""
    import bmesh
    from face import H,Y0,Z0
    bm=bmesh.new();bm.from_mesh(data)
    regions=bm.faces.layers.int['NormalRegion']
    original=[v.co.copy() for v in bm.verts]
    def inside(v):
        p=v.co
        return .12<(p.z+.021-Y0)/H<.38 and (Z0+p.y)/H>.05 and abs(p.x)>.035
    edges=[e for e in bm.edges if not e.is_boundary and e.calc_length()>.008 and all(inside(v) for v in e.verts) and all(f[regions] in (1,2) for f in e.link_faces)]
    assert edges,'No local skin edges selected'
    before_faces=len(bm.faces)
    result=bmesh.ops.subdivide_edges(bm,edges=edges,cuts=2,use_grid_fill=True,smooth=0.)
    bm.normal_update()
    from mathutils.kdtree import KDTree
    tree=KDTree(len(bm.verts))
    for i,v in enumerate(bm.verts):tree.insert(v.co,i)
    tree.balance()
    assert all(tree.find(p)[2]<1e-9 for p in original),'Original skin vertices moved'
    assert all(f.calc_area()>1e-12 for f in bm.faces),'Refinement collapsed a face'
    assert all(len(e.link_faces)<=2 for e in bm.edges),'Refinement became non-manifold'
    report={'split_edges':len(edges),'added_faces':len(bm.faces)-before_faces,'new_vertices':len(bm.verts)-len(original),'original_vertices_fixed':True,'ear_rim_fixed':True}
    bm.to_mesh(data);bm.free();data.update()
    return report

"""Improve one internal lower-root diagonal per ear, without moving vertices."""
def triangle_quality(a,b,c):
    from math import sqrt
    u=[b[i]-a[i] for i in range(3)];v=[c[i]-a[i] for i in range(3)]
    cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    den=sum((p[i]-q[i])**2 for p,q in ((a,b),(b,c),(c,a)) for i in range(3))
    return 2*sqrt(3)*sqrt(sum(x*x for x in cross))/max(den,1e-20)

def improve_lower_root_diagonals(data,reports):
    import bmesh
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    old=[v.co.copy() for v in data.vertices]
    bm=bmesh.new();bm.from_mesh(data);bm.verts.ensure_lookup_table();bm.normal_update()
    region=bm.faces.layers.int.get('NormalRegion');assert region is not None
    tree=KDTree(len(bm.verts))
    for i,v in enumerate(bm.verts):tree.insert(v.co,i)
    tree.balance();results=[];accepted=[]
    for report in reports:
        p=min(report['root_positions'],key=lambda p:p[2])
        _,i,error=tree.find(Vector((p[0],p[1],p[2]-.021)));assert error<.000002
        root=bm.verts[i];candidates=[]
        if report.get('source_fan_policy') in ('angular-source-fan-v1','quality-budget-source-fan-v2'):
            quality = source_fan_quality(root, region)
            print('SOURCE_FAN_GATE', quality)
            if quality is not None:
                accepted.append(dict(quality, side=1 if root.co.x>0 else -1))
                continue
        for edge in root.link_edges:
            faces=list(edge.link_faces)
            if len(faces)!=2 or any(len(f.verts)!=3 for f in faces):continue
            if not (40<=faces[0][region]<50 and faces[0][region]==faces[1][region]):continue
            a,b=edge.verts
            c=next(v for v in faces[0].verts if v not in (a,b))
            d=next(v for v in faces[1].verts if v not in (a,b))
            if c==d or bm.edges.get((c,d)) is not None:continue
            if max((v.co-root.co).length for v in (a,b,c,d))>.012:continue
            q0=min(triangle_quality(*[v.co for v in f.verts]) for f in faces)
            q1=min(triangle_quality(a.co,c.co,d.co),triangle_quality(b.co,c.co,d.co))
            if q1<=q0+.05:continue
            normal=(faces[0].normal+faces[1].normal).normalized()
            # The new diagonal must split a convex projected quadrilateral.
            side_a=(d.co-c.co).cross(a.co-c.co).dot(normal)
            side_b=(d.co-c.co).cross(b.co-c.co).dot(normal)
            if side_a*side_b>=-1e-18:continue
            candidates.append((q1-q0,edge,q0,q1,normal.copy()))
        assert candidates,'No safe lower-root quality improvement found'
        _,edge,q0,q1,normal=max(candidates,key=lambda x:x[0])
        rotated=bmesh.utils.edge_rotate(edge,False);assert rotated is not None
        bm.normal_update()
        assert all(f.calc_area()>1e-12 and f.normal.dot(normal)>.1 for f in rotated.link_faces),'Rotated root face inverted'
        actual=min(triangle_quality(*[v.co for v in f.verts]) for f in rotated.link_faces)
        assert actual>q0+.05
        results.append({'before_quality':q0,'after_quality':actual,'side':1 if root.co.x>0 else -1})
    assert len(results)+len(accepted)==2
    bm.to_mesh(data);bm.free();data.update()
    assert len(data.vertices)==len(old) and all((v.co-p).length<1e-9 for v,p in zip(data.vertices,old))
    return {'edge_flips':results,'accepted_source_fans':accepted,'all_vertex_positions_unchanged':True}


"""Independent source-fan geometry gate; no topology or position changes."""

def source_fan_quality(root, region):
    from math import acos, degrees, isfinite, sqrt
    faces = [f for f in root.link_faces if 40 <= f[region] < 50]
    if not 3 <= len(faces) <= 8 or any(len(f.verts) != 3 for f in faces):
        return None
    if len({(f[region], f.material_index) for f in faces}) != 1:
        return None
    vertices = {v for f in faces for v in f.verts}
    if len(vertices) != len(faces)+2:
        return None
    if any(not isfinite(c) for v in vertices for c in v.co):
        return None
    if any(not isfinite(c) for face in faces for c in face.normal):
        return None
    if any(not isfinite(face.calc_area()) or abs(face.normal.length-1) > 1e-5 for face in faces):
        return None
    links = {}
    angles = []
    root_angles = []
    for face in faces:
        if root not in face.verts or face.calc_area() <= 1e-12:
            return None
        opposite = [v for v in face.verts if v != root]
        a,b = opposite
        links.setdefault(a,set()).add(b)
        links.setdefault(b,set()).add(a)
        points = [v.co for v in face.verts]
        for i in range(3):
            a,b = points[(i+1)%3]-points[i],points[(i+2)%3]-points[i]
            if a.length <= 1e-8 or b.length <= 1e-8:
                return None
            angle = degrees(acos(max(-1., min(1., a.dot(b)/(a.length*b.length)))))
            angles.append(angle)
            if face.verts[i] == root:
                root_angles.append(angle)
    if sum(len(v)==1 for v in links.values()) != 2 or any(len(v) not in (1,2) for v in links.values()):
        return None
    seen = set()
    pending = [next(iter(links))]
    while pending:
        v = pending.pop()
        if v not in seen:
            seen.add(v)
            pending.extend(links[v]-seen)
    if seen != set(links) or not 0 < sum(root_angles) < 180:
        return None
    if any(not isfinite(c) for v in vertices for c in v.co):
        return None
    if max((v.co-root.co).length for v in vertices) > .012:
        return None
    quality = min(triangle_quality(*[v.co for v in f.verts]) for f in faces)
    minimum_dot = min(a.normal.dot(b.normal) for a in faces for b in faces)
    if quality < .35 or min(angles) < 12 or max(angles) > 145 or minimum_dot < sqrt(.5):
        return None
    return {'policy':'independent-source-fan-quality-v1','minimum_quality':quality,
            'minimum_angle_degrees':min(angles),'maximum_angle_degrees':max(angles),
            'minimum_pair_normal_dot':minimum_dot,'triangles':len(faces),
            'operation':'none; independently well-conditioned source fan'}


def triangle_angles(points):
    from math import atan2, degrees, sqrt
    result=[]
    for i in range(3):
        a=[points[(i+1)%3][k]-points[i][k] for k in range(3)]
        b=[points[(i+2)%3][k]-points[i][k] for k in range(3)]
        cross=[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
        result.append(degrees(atan2(sqrt(sum(x*x for x in cross)),sum(x*y for x,y in zip(a,b)))))
    return result


def neighboring_quality_preserved(faces, old):
    from math import isfinite
    for face in faces:
        if len(face.verts)!=3:
            return False
        before=[old[v] for v in face.verts]
        after=[v.co for v in face.verts]
        if any(not isfinite(c) for point in before+after for c in point):
            return False
        q0,q1=triangle_quality(*before),triangle_quality(*after)
        if not isfinite(q0) or not isfinite(q1) or min(q0,q1)<=0:
            return False
        if q1<min(.35,q0)-1e-7:
            return False
        a,b=triangle_angles(before),triangle_angles(after)
        if any(not isfinite(v) or v<=0 for v in a+b):
            return False
        if min(b)<min(12.,min(a))-1e-5 or max(b)>max(145.,max(a))+1e-5:
            return False
    return True


"""Pure source-generation fan resampling, sampling points on the prior polyline with fixed endpoints."""
from math import atan2,sqrt


def resample_fan_chain(root, chain, fraction=1.):
    def vector(p):
        return tuple(a-b for a,b in zip(p,root))
    def unit(v):
        length=sqrt(sum(x*x for x in v))
        if length<1e-9:
            raise ValueError('Degenerate fan ray')
        return tuple(x/length for x in v)
    def angle(a,b):
        a,b=unit(a),unit(b)
        cross=(a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
        return atan2(sqrt(sum(x*x for x in cross)),sum(x*y for x,y in zip(a,b)))
    rays=[vector(p) for p in chain]
    cumulative=[0.]
    for a,b in zip(rays,rays[1:]):
        cumulative.append(cumulative[-1]+angle(a,b))
    if not .05<cumulative[-1]<3.141592653589793:
        raise ValueError('Unsupported source fan angular span')
    result=[tuple(chain[0])]
    for i in range(1,len(chain)-1):
        target=cumulative[i]+fraction*(cumulative[-1]*i/(len(chain)-1)-cumulative[i])
        index=next(j for j in range(len(chain)-1) if cumulative[j]<=target<=cumulative[j+1])
        lo,hi=0.,1.
        for _ in range(35):
            t=(lo+hi)/2
            ray=tuple(a+(b-a)*t for a,b in zip(rays[index],rays[index+1]))
            if cumulative[index]+angle(rays[index],ray)<target:lo=t
            else:hi=t
        t=(lo+hi)/2
        result.append(tuple(a+(b-a)*t for a,b in zip(chain[index],chain[index+1])))
    return result+[tuple(chain[-1])]


"""Bounded in-generation resampling after attachment fairing, before export."""

def reflow_source_fans(data, reports):
    from math import isfinite
    policies=[r.get('source_fan_policy') for r in reports]
    if all(policy is None for policy in policies):
        return {'policy':None,'not_applicable':True,'sides':[]}
    assert len(reports)==2 and len(set(policies))==1 and policies[0] in ('angular-source-fan-v1','quality-budget-source-fan-v2'),'Incomplete source fan policy coverage'
    expected_sides={1 if min(r['root_positions'],key=lambda p:p[2])[0]>0 else -1 for r in reports}
    assert expected_sides=={-1,1},'Duplicate or missing source fan side'
    import bmesh
    from mathutils import Vector
    from mathutils.kdtree import KDTree
    bm=bmesh.new()
    try:
        bm.from_mesh(data)
        bm.verts.ensure_lookup_table()
        bm.normal_update()
        region=bm.faces.layers.int.get('NormalRegion')
        assert region is not None
        old={v:v.co.copy() for v in bm.verts}
        assert all(isfinite(c) for p in old.values() for c in p),'Nonfinite source mesh'
        old_normals={f:f.normal.copy() for f in bm.faces}
        tree=KDTree(len(bm.verts))
        for i,v in enumerate(bm.verts):tree.insert(v.co,i)
        tree.balance()
        changes=set()
        result=[]
        for report in reports:
            if report.get('source_fan_policy') not in ('angular-source-fan-v1','quality-budget-source-fan-v2'):
                continue
            p=min(report['root_positions'],key=lambda p:p[2])
            _,index,error=tree.find(Vector((p[0],p[1],p[2]-.021)))
            assert error<.000002
            root=bm.verts[index]
            initial_quality=source_fan_quality(root,region)
            if initial_quality is not None:
                result.append(dict(initial_quality,side=1 if root.co.x>0 else -1,moved_vertices=0))
                continue
            faces=[f for f in root.link_faces if 40<=f[region]<50]
            assert len(faces)==5 and all(len(f.verts)==3 for f in faces)
            assert len({(f[region],f.material_index) for f in faces})==1
            links={}
            for face in faces:
                pair=[v for v in face.verts if v!=root]
                assert len(pair)==2
                a,b=pair
                links.setdefault(a,set()).add(b)
                links.setdefault(b,set()).add(a)
            ends=[v for v,neighbors in links.items() if len(neighbors)==1]
            assert len(links)==6 and len(ends)==2 and all(len(v) in (1,2) for v in links.values())
            chain=[min(ends,key=lambda v:tuple(v.co))]
            while True:
                options=links[chain[-1]]-set(chain)
                if not options:break
                assert len(options)==1
                chain.append(next(iter(options)))
            assert len(chain)==6
            points=[tuple(v.co) for v in chain]
            movable=set(chain[1:-1])
            for v in movable:
                assert all(f[region]==faces[0][region] and f.material_index==faces[0].material_index for f in v.link_faces),'Fan interior touches protected skin/return'
            uv_snapshots={}
            for layer in bm.loops.layers.uv.values():
                values={}
                for v in chain:
                    samples=[tuple(loop[layer].uv) for loop in v.link_loops
                             if loop.face[region]==faces[0][region] and loop.face.material_index==faces[0].material_index]
                    assert samples and all(isfinite(c) for value in samples for c in value), 'Nonfinite ear UV'
                    assert samples and all(max(abs(a-b) for a,b in zip(samples[0],p))<1e-7 for p in samples), 'Source fan crosses UV seam'
                    values[v]=samples[0]
                uv_snapshots[layer]=values
            core=set(movable)
            region_id,material=faces[0][region],faces[0].material_index
            for _ in range(2):
                neighbors={edge.other_vert(v) for v in movable for edge in v.link_edges}
                movable.update(v for v in neighbors if (v.co-root.co).length<.020
                               and all(f[region]==region_id and f.material_index==material for f in v.link_faces)
                               and all(len(edge.link_faces)==2 for edge in v.link_edges))
            assert root not in movable and not set(ends)&movable
            quality=None
            for factor in (1.,.75,.5,.25):
                target=resample_fan_chain(tuple(root.co),points,factor)
                assert all(isfinite(c) for p in target for c in p),'Nonfinite target fan'
                maximum=max((Vector(p)-old[v]).length for v,p in zip(chain,target))
                if maximum>.0015:continue
                for v,p in zip(chain,target):
                    if v not in core:continue
                    q=Vector(p)
                    segments=[]
                    for k,(a,b) in enumerate(zip(points,points[1:])):
                        # Resolve UV interpolation in source float64 before mesh float32 storage.
                        d=tuple(y-x for x,y in zip(a,b))
                        delta=tuple(y-x for x,y in zip(a,p))
                        t=max(0.,min(1.,sum(x*y for x,y in zip(delta,d))/max(sum(x*x for x in d),1e-20)))
                        distance=sum((value-(x+(y-x)*t))**2 for value,x,y in zip(p,a,b))**.5
                        segments.append((distance,k,t))
                    distance,k,t=min(segments)
                    assert distance<1e-8
                    v.co=q
                    for layer,values in uv_snapshots.items():
                        value=tuple(a+(b-a)*t for a,b in zip(values[chain[k]],values[chain[k+1]]))
                        for loop in v.link_loops:loop[layer].uv=value
                # Extend displacement through two interior rings, with every boundary fixed.
                delta={v:Vector((0,0,0)) for v in movable}
                for v in core:delta[v]=v.co-old[v]
                for _ in range(40):
                    previous={v:d.copy() for v,d in delta.items()}
                    for v in movable-core:
                        linked=[(edge.other_vert(v),1/max((old[edge.other_vert(v)]-old[v]).length,1e-7)) for edge in v.link_edges]
                        weight=sum(w for _,w in linked)
                        delta[v]=sum((previous.get(n,Vector((0,0,0)))*w for n,w in linked),Vector((0,0,0)))/weight
                for v in movable-core:v.co=old[v]+delta[v]
                maximum=max((v.co-old[v]).length for v in movable)
                assert maximum<=.0015
                bm.normal_update()
                adjacent={f for v in movable for f in v.link_faces}
                safe=all(f.calc_area()>1e-12 and f.normal.dot(old_normals[f])>.1 for f in adjacent)
                safe=safe and neighboring_quality_preserved(adjacent,old)
                quality=source_fan_quality(root,region) if safe else None
                if quality is not None:break
            assert quality is not None,'No bounded fan reflow preserves neighboring quality'
            changes.update(movable)
            result.append(dict(quality,side=1 if root.co.x>0 else -1,
                               operation="bounded in-generation resampling with harmonic neighbor extension",
                               selected_vertices=len(movable),moved_vertices=sum((v.co-old[v]).length>1e-9 for v in movable),maximum_displacement_m=maximum,
                               resampling_fraction=factor,neighbor_quality_budget_preserved=True,
                               neighbor_policy="q>=min(previous,.35); min_angle>=min(previous,12deg); max_angle<=max(previous,145deg)"))
        assert all((v.co-p).length<1e-9 for v,p in old.items() if v not in changes)
        assert len(result)==2 and {r['side'] for r in result}==expected_sides
        bm.to_mesh(data)
    finally:
        bm.free()
    data.update()
    return {'policy':policies[0],'topology_unchanged':True,
            'root_and_rim_endpoints_fixed':True,'sides':result}

"""Shared paired-pose validity for ordered triangle/quad source polygons."""

def polygon_diagonals(points, face, posed=None):
    """Return common nonfolding diagonals; reject crossed/degenerate boundaries."""
    import math
    def sub(a,b):return tuple(x-y for x,y in zip(a,b))
    def dot(a,b):return sum(x*y for x,y in zip(a,b))
    def cross(a,b):return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
    def norm(a):return math.sqrt(dot(a,a))
    def orient(a,b,c):return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
    def state(values):
        p=[tuple(values[i]) for i in face];n=len(p)
        if n not in (3,4) or len(set(face))!=n or not all(math.isfinite(c) for v in p for c in v):return {}
        scale=max(norm(sub(a,b)) for a in p for b in p)
        edge_tol=max(1e-10,scale*1e-8);area_tol=max(2e-12,scale*scale*1e-8)
        if any(norm(sub(p[i],p[(i+1)%n]))<=edge_tol for i in range(n)):return {}
        area=tuple(sum(cross(p[i],p[(i+1)%n])[j] for i in range(n)) for j in range(3))
        if norm(area)<=area_tol:return {}
        axis=max(range(3),key=lambda j:abs(area[j]));q=[tuple(v[j] for j in range(3) if j!=axis) for v in p]
        signed=sum(q[i][0]*q[(i+1)%n][1]-q[(i+1)%n][0]*q[i][1] for i in range(n))
        sign=1 if signed>0 else -1
        def on(a,b,c):return abs(orient(a,b,c))<=area_tol and all(min(a[j],b[j])-edge_tol<=c[j]<=max(a[j],b[j])+edge_tol for j in (0,1))
        def crossing(a,b,c,d):
            x,y,z,w=orient(a,b,c),orient(a,b,d),orient(c,d,a),orient(c,d,b)
            return (x*y<0 and z*w<0) or on(a,b,c) or on(a,b,d) or on(c,d,a) or on(c,d,b)
        if n==4 and (crossing(q[0],q[1],q[2],q[3]) or crossing(q[1],q[2],q[3],q[0])):return {}
        options={0:((0,1,2),)} if n==3 else {0:((0,1,2),(0,2,3)),1:((0,1,3),(1,2,3))}
        eligible={}
        for diagonal,triangles in options.items():
            normals=[];quality=1.;valid=True
            for a,b,c in triangles:
                normal=cross(sub(p[b],p[a]),sub(p[c],p[a]));length=norm(normal)
                if length<=area_tol or orient(q[a],q[b],q[c])*sign<=area_tol or dot(normal,area)<=0:valid=False;break
                normals.append(normal)
                denom=sum(dot(sub(p[u],p[v]),sub(p[u],p[v])) for u,v in ((a,b),(b,c),(c,a)))
                quality=min(quality,2*math.sqrt(3)*length/max(denom,1e-30))
            if not valid:continue
            if len(normals)==2:
                coherence=dot(normals[0],normals[1])/(norm(normals[0])*norm(normals[1]))
                if coherence<=0.:continue
            else:coherence=1.
            eligible[diagonal]=(coherence,quality)
        return eligible
    neutral=state(points)
    other=state(posed) if posed is not None else neutral
    if posed is not None and neutral and other:
        def area_vector(values):
            p=[tuple(values[i]) for i in face]
            return tuple(sum(cross(p[i],p[(i+1)%len(p)])[j] for i in range(len(p))) for j in range(3))
        a,b=area_vector(points),area_vector(posed)
        if dot(a,b)<=.1*norm(a)*norm(b):return {}
    return {k:(min(neutral[k][0],other[k][0]),min(neutral[k][1],other[k][1])) for k in neutral.keys() & other.keys()}


def triangulate_valid_skin(data, bm):
    """Preserve a common valid skin diagonal in neutral and mouth-open poses."""
    import bmesh, math
    # Cleanup may have changed BMesh connectivity; validate that actual mesh.
    bm.to_mesh(data);data.update();bm.clear();bm.from_mesh(data)
    bm.verts.index_update();bm.faces.index_update()
    regions=data.attributes.get('NormalRegion');delta=data.attributes.get('MouthOpenDelta')
    assert regions is not None and regions.domain=='FACE' and regions.data_type=='INT'
    assert delta is not None and delta.domain=='POINT' and delta.data_type=='FLOAT_VECTOR' and len(delta.data)==len(data.vertices)
    neutral=[tuple(v.co) for v in data.vertices]
    posed=[tuple(v.co+d.vector) for v,d in zip(data.vertices,delta.data)]
    bm.faces.ensure_lookup_table();bm.verts.ensure_lookup_table()
    choices={0:[],1:[]};wanted=[];untouched=[];invalid=[];warnings=[]
    for poly in data.polygons:
        if regions.data[poly.index].value not in (1,2):continue
        if len(poly.vertices)>4:
            untouched.append({'region':regions.data[poly.index].value,'corners':len(poly.vertices),'vertices':list(poly.vertices),'coordinates':[neutral[i] for i in poly.vertices]})
            continue
        assert len(poly.vertices)>=3, 'Degenerate skin polygon'
        if len(poly.vertices)==3:
            assert polygon_diagonals(neutral,tuple(poly.vertices),posed), f'Invalid existing skin triangle {poly.index}'
            continue
        eligible=polygon_diagonals(neutral,tuple(poly.vertices),posed)
        if not eligible:
            invalid.append({'polygon':poly.index,'region':regions.data[poly.index].value,'ids':list(poly.vertices),'neutral':[neutral[i] for i in poly.vertices],'posed':[posed[i] for i in poly.vertices],'neutral_choices':polygon_diagonals(neutral,tuple(poly.vertices)),'posed_choices':polygon_diagonals(posed,tuple(poly.vertices))})
            continue
        selected=max(eligible,key=lambda k:eligible[k]);face=bm.faces[poly.index]
        if eligible[selected][0]<.1:
            nc=polygon_diagonals(neutral,tuple(poly.vertices))[selected][0];pc=polygon_diagonals(posed,tuple(poly.vertices))[selected][0]
            warnings.append({'polygon':poly.index,'region':regions.data[poly.index].value,'selected_diagonal':selected,'minimum_coherence':eligible[selected][0],'neutral_angle_degrees':math.degrees(math.acos(max(-1.,min(1.,nc)))),'posed_angle_degrees':math.degrees(math.acos(max(-1.,min(1.,pc)))),'status':'severe crease; visual acceptance required'})
        assert tuple(v.index for v in face.verts)==tuple(poly.vertices), 'BMesh polygon ordering changed'
        choices[selected].append(face)
        ids=tuple(poly.vertices)
        local=((0,1,2),(0,2,3)) if selected==0 else ((0,1,3),(1,2,3))
        wanted.append((set(face.verts),[tuple(ids[i] for i in tri) for tri in local]))
    if invalid:
        import json
        print('SKIN_POLYGON_DIAGNOSTICS '+json.dumps({'invalid':invalid,'untouched_ngons':untouched}),flush=True)
        raise ValueError(f'Invalid paired skin polygons: {len(invalid)}')
    for choice,faces in choices.items():
        if faces:bmesh.ops.triangulate(bm,faces=faces,quad_method='FIXED' if choice==0 else 'ALTERNATE',ngon_method='BEAUTY')
    def canonical(tri):return min(tri[i:]+tri[:i] for i in range(3))
    for vertices,expected in wanted:
        actual=[tuple(v.index for v in f.verts) for f in bm.faces if len(f.verts)==3 and set(f.verts)<=vertices]
        assert len(actual)==2 and {canonical(t) for t in actual}=={canonical(t) for t in expected}, 'Chosen skin triangles or winding not preserved'
        assert all(polygon_diagonals(neutral,t,posed) for t in actual), 'Triangulated skin pose invalid'
    return {'quality_warnings':warnings,'untouched_ngons':untouched,'ngon_polygon_validity':'unverified; retained existing BEAUTY path','paired_skin_quads':sum(map(len,choices.values())),'fixed':len(choices[0]),'alternate':len(choices[1]),'selection':'common neutral/posed valid diagonal, then worst-pose coherence/quality'}


def verify_skin_triangles(data):
    """Hard-fail actual post-triangulation skin endpoint defects."""
    regions=data.attributes.get('NormalRegion');delta=data.attributes.get('MouthOpenDelta')
    assert regions is not None and regions.domain=='FACE' and regions.data_type=='INT'
    assert delta is not None and delta.domain=='POINT' and delta.data_type=='FLOAT_VECTOR'
    assert len(delta.data)==len(data.vertices)
    neutral=[tuple(v.co) for v in data.vertices]
    posed=[tuple(v.co+d.vector) for v,d in zip(data.vertices,delta.data)]
    checked=0
    for poly in data.polygons:
        if regions.data[poly.index].value not in (1,2):continue
        assert len(poly.vertices)==3 and polygon_diagonals(neutral,tuple(poly.vertices),posed), f'Invalid final skin triangle {poly.index}'
        checked+=1
    return {'checked':checked,'paired_endpoint_validity':True,'continuous_validity':'separate existing checks required'}

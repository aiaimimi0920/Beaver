"""Pure CPU late-source cap proposal. No mesh mutation; caller must audit full interfaces.
Inputs: current world vertices/faces, two triangle indices, ORIGINAL ear-template
vertex IDs (before late retopology). Global IDs are discovered, never baked in.
All displacement budgets below are meters. Existing ear-template points stay exact.
"""
import numpy as np

def solve_return_cap(vertices, faces, triangle_pair, original_ear_vertex_ids, least_squares, radius_m=.0005):
    v=np.asarray(vertices,dtype=float)
    if not np.isfinite(v).all() or radius_m<=0 or radius_m>.0005:
        raise ValueError('Invalid input or cap displacement budget exceeds 0.5 mm')
    pair=list(triangle_pair)
    if len(set(pair))!=2 or any(len(faces[i])!=3 for i in pair):
        raise ValueError('Expected exactly two source triangles')
    tris=[list(faces[i]) for i in pair]
    shared=set(tris[0])&set(tris[1])
    if len(shared)!=2:raise ValueError('Triangle pair must share exactly one edge')
    edges=[(a,b) for f in tris for a,b in zip(f,f[1:]+f[:1])]
    boundary=[e for e in edges if tuple(reversed(e)) not in edges]
    if len(boundary)!=4:raise ValueError('Inconsistent triangle winding')
    quad=[boundary[0][0]]
    for _ in range(3):
        nxt=[b for a,b in boundary if a==quad[-1]]
        if len(nxt)!=1 or nxt[0] in quad:raise ValueError('Non-simple cap boundary')
        quad.append(nxt[0])
    if (quad[-1],quad[0]) not in boundary:raise ValueError('Open cap boundary')
    original=set(original_ear_vertex_ids);moving=set(quad)-original
    if len(moving)!=2 or len(set(quad)&original)!=2 or len(shared&moving)!=1:
        raise ValueError('Cap must have two original-ear points and two new-return points')
    local=[i for i,g in enumerate(quad) if g in moving]
    origin=v[quad[0]].copy();unit=.001;base=(v[quad]-origin)/unit;initial=base[local].copy()
    adjacent=[i for i,f in enumerate(faces) if i not in pair and moving&set(f)]
    if any(len(faces[i]) not in (3,4) for i in adjacent):raise ValueError('Unsupported adjacent polygon')
    patterns=((0,1,2),(0,2,3),(0,1,3),(1,2,3))
    def normals(points,patterns):
        nn=np.array([np.cross(points[b]-points[a],points[c]-points[a]) for a,b,c in patterns]);size=np.linalg.norm(nn,axis=1)
        return nn/np.maximum(size[:,None],1e-30),size
    refs=[]
    for fi in adjacent:
        pat=patterns if len(faces[fi])==4 else ((0,1,2),)
        nn,aa=normals(v[list(faces[fi])],pat)
        if np.min(aa)<=1e-14:raise ValueError('Degenerate adjacent source polygon')
        refs.append((fi,pat,nn,aa))
    oldnorm=sum(np.cross(v[t[1]]-v[t[0]],v[t[2]]-v[t[0]]) for t in tris)
    oldnorm/=np.linalg.norm(oldnorm)
    def angles(points,patterns):
        values=[]
        for tri in patterns:
            ps=points[list(tri)]
            for k in range(3):
                a=ps[(k+1)%3]-ps[k];b=ps[(k+2)%3]-ps[k]
                values.append(np.degrees(np.arccos(np.clip(a@b/max(np.linalg.norm(a)*np.linalg.norm(b),1e-30),-1,1))))
        return np.array(values)
    old_min=min(float(angles(v[t],((0,1,2),)).min()) for t in tris)
    target_angle=old_min+.001
    def evaluate(x):
        p=base.copy();p[local]=x.reshape(-1,3);world=p*unit+origin
        # Restore original-ear anchors bit-for-bit, avoiding arithmetic roundoff.
        for i,g in enumerate(quad):
            if g not in moving:world[i]=v[g]
        nn,aa=normals(world,patterns);dots=np.array([nn[0]@nn[1],nn[2]@nn[3]])
        relative=[];ratios=[];neighbor_pairs=[]
        for fi,pat,oldn,olda in refs:
            ps=np.array([world[quad.index(g)] if g in moving else v[g] for g in faces[fi]])
            ns,areas=normals(ps,pat);relative.extend(np.sum(ns*oldn,axis=1));ratios.extend(areas/olda)
            if len(pat)==4:neighbor_pairs.extend([ns[0]@ns[1],ns[2]@ns[3]])
        return world,dots,angles(world,patterns),nn@oldnorm,np.array(relative),np.array(ratios),np.array(neighbor_pairs),aa
    def loss(x):
        world,d,a,n,rel,ratio,npair,area=evaluate(x)
        return np.r_[np.maximum(target_angle-a,0)*10,np.maximum(.55-d,0)*100,np.maximum(.25-n,0)*50,np.maximum(.8-rel,0)*30,np.maximum(.25-ratio,0)*30,np.maximum(.50-npair,0)*30,(x-initial.ravel())*.3]
    radius=radius_m/unit
    def project(x):
        delta=x.reshape(-1,3)-initial;length=np.linalg.norm(delta,axis=1)
        return (initial+delta*np.minimum(1.,radius/np.maximum(length,1e-30))[:,None]).ravel()
    r=least_squares(loss,initial.ravel(),bounds=(initial.ravel()-radius,initial.ravel()+radius),max_nfev=1000,project=project)
    world,d,a,n,rel,ratio,npair,area=evaluate(r.x)
    displacement=np.linalg.norm(world-v[quad],axis=1)
    safe=(np.isfinite(world).all() and d.min()>=.49 and a.min()>=old_min and n.min()>.14 and rel.min()>.7 and ratio.min()>.2 and npair.min()>=.49 and area.min()>1e-14 and displacement.max()<=radius_m+1e-12)
    report=dict(triangle_pair=pair,boundary_global_ids=quad,movable_global_ids=sorted(moving),vertices=world.tolist(),quads=[[0,1,2,3]],max_displacement_m=float(displacement.max()),min_pair_dot=float(d.min()),old_min_angle=float(old_min),min_angle=float(a.min()),min_reference_normal=float(n.min()),neighbor_min_relative_normal=float(rel.min()),neighbor_min_area_ratio=float(ratio.min()),neighbor_min_pair_dot=float(npair.min()),adjacent_faces=adjacent,quality_pass=bool(safe),interface_crossings='caller must check affected caps AND adjacent faces against complete source mesh')
    if not safe:raise ValueError('Bounded return cap geometry gate failed: '+str(report))
    return report

def solve_return_cap_with_seed(vertices, faces, triangle_pair, original_ear_vertex_ids, least_squares, radius_m=.0005):
    """Same strict gates; deterministic planar seed fallback for collinear input.
    Seed is 0.05 mm in the cap plane perpendicular to the original fixed-point
    chord. It changes optimizer initialization only, never the displacement origin.
    """
    try:
        result=solve_return_cap(vertices,faces,triangle_pair,original_ear_vertex_ids,least_squares,radius_m)
        result['initialization']='unperturbed'
        return result
    except ValueError as initial_error:
        original_error=initial_error
    v=np.asarray(vertices,dtype=float);pair=list(triangle_pair)
    if len(pair)!=2 or any(len(faces[i])!=3 for i in pair):raise original_error
    ids=set(faces[pair[0]])|set(faces[pair[1]])
    fixed=sorted(ids&set(original_ear_vertex_ids))
    if len(fixed)!=2:raise original_error
    normal=sum(np.cross(v[faces[i][1]]-v[faces[i][0]],v[faces[i][2]]-v[faces[i][0]]) for i in pair)
    direction=np.cross(normal,v[fixed[1]]-v[fixed[0]])
    length=np.linalg.norm(direction)
    if not np.isfinite(length) or length<=1e-20:raise original_error
    direction/=length
    errors=[str(original_error)]
    for sign in (1.,-1.):
        def seeded(fun,x,**kwargs):
            x=np.asarray(x,dtype=float).reshape(-1,3)+sign*.05*direction
            return least_squares(fun,x.ravel(),**kwargs)
        try:
            result=solve_return_cap(vertices,faces,triangle_pair,original_ear_vertex_ids,seeded,radius_m)
            result.update(initialization='planar_seed',seed_displacement_m=(sign*.00005*direction).tolist(),failed_initializations=errors)
            return result
        except ValueError as error:
            errors.append(str(error))
    raise ValueError('All bounded cap initializations failed: '+str(errors))

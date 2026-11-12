"""Bounded upper-front ear fairing without changing topology or authored UVs."""

def flow_refine_upper_front(result, original_vertices):
    import numpy as np
    from scipy.optimize import minimize
    original = np.asarray(original_vertices,dtype=float)*1000
    points = np.asarray(result['vertices'],dtype=float)*1000
    if original.shape!=(40,3) or points.shape!=(43,3):
        raise ValueError('Unsupported upper front ear source dimensions')
    if not {0,1,5,9,15,21,27,38,26}.issubset(result['fixed_vertex_ids']):
        raise ValueError('Upper front ear anchor contract changed')
    if not np.isfinite(original).all() or not np.isfinite(points).all():
        raise ValueError('Non-finite source ear geometry')
    side = 1 if original[0,0]>0 else -1
    original[:,0] *= -side
    points[:,0] *= -side
    quads = np.asarray(result['quads'],dtype=int)
    if quads.shape!=(34,4):
        raise ValueError('Unsupported upper front quad count')
    if list(quads[19]) != [23,24,25,27] or list(quads[21]) != [25,26,37,27]:
        raise ValueError('Upper front ear source topology changed')
    free = np.array([18,19,22,23,24,25,35,26])
    affected = np.where(np.any(np.isin(quads,free),axis=1))[0]
    radius = 3.999999*float(np.linalg.norm(original[27]-original[0])/56)
    reference=points.copy();reference[:40]=original
    patterns=np.array([(0,1,2),(0,2,3),(0,1,3),(1,2,3)])
    def normal(cells):
        n=np.cross(cells[:,1]-cells[:,0],cells[:,2]-cells[:,0])
        n+=np.cross(cells[:,2]-cells[:,0],cells[:,3]-cells[:,0])
        return n/np.maximum(np.linalg.norm(n,axis=1)[:,None],1e-20)
    ref=normal(reference[quads])
    def corners(cells):
        a=np.roll(cells,1,axis=1)-cells;b=np.roll(cells,-1,axis=1)-cells
        c=(a*b).sum(2)/np.maximum(np.linalg.norm(a,axis=2)*np.linalg.norm(b,axis=2),1e-20)
        return np.degrees(np.arccos(np.clip(c,-1,1)))
    def review(p,ids):
        patch=p[quads[ids]]
        side_view=patch[:,:,[1,2]]
        diagonal=np.stack([(patch[:,:,0]-patch[:,:,1])/np.sqrt(2),patch[:,:,2]],axis=2)
        return np.r_[corners(side_view).ravel(),corners(diagonal).ravel()]
    previous_front=review(points,[19,21])
    def decode(x):
        p=points.copy();p[free]=x.reshape(-1,3);return p
    def measures(p):
        d,a,ar=ear_quad_measures(p,quads)
        tri=p[quads][:,patterns]
        n=np.cross(tri[:,:,1]-tri[:,:,0],tri[:,:,2]-tri[:,:,0])
        n/=np.maximum(np.linalg.norm(n,axis=2)[:,:,None],1e-20)
        o=(n*ref[:,None,:]).sum(2)
        ns=normal(p[quads[[19,21]]]);cd=float(ns[0]@ns[1])
        return d,a,ar,o,cd
    def point_constraints(p):
        d,a,ar,o,cd=measures(p);ix=affected
        pr=review(p,[19,21]);ap=review(p,[31,32,33]);ac=corners(p[quads[31:34]])
        return np.r_[radius-.0003-np.linalg.norm(p[free[:-1]]-original[free[:-1]],axis=1),
                     1-.0003-np.linalg.norm(p[26]-original[26]),
                     (d[ix]-.4901).ravel(),(a[ix]-8.01).ravel()/30,
                     (159.99-a[ix]).ravel()/30,(o[ix]-.1401).ravel(),
                     (ar[ix]-1e-6).ravel(),(pr-6.101)/30,(166.98-pr)/30,cd-.6501,
                     (ap-9.901)/30,(159.999-ap)/30,(ac-20.001).ravel()/30,
                     (144.999-ac).ravel()/30,
                     (pr[4:8].min()-previous_front[4:8].min())/30,
                     (pr[12:16].min()-previous_front[12:16].min())/30]
    def constraints(x):
        return point_constraints(decode(x))
    initial=points[free].ravel();rng=np.random.default_rng(198)
    bounds=[((original[i,j] if i<40 else points[i,j])-radius,
             (original[i,j] if i<40 else points[i,j])+radius) for i in free for j in range(3)]
    horizontal=np.array([1.,-1.,0.])/np.sqrt(2)
    vertical=np.array([0.,0.,1.])
    u3=points[26]-points[37];v3=points[27]-points[37]
    u=np.array([u3@horizontal,u3@vertical]);v=np.array([v3@horizontal,v3@vertical])
    angle=float(np.arctan2(u[0]*v[1]-u[1]*v[0],u@v))
    rotation=np.sign(angle)*max(0.,abs(angle)-np.radians(166.9))
    rot=np.array([[np.cos(rotation),-np.sin(rotation)],[np.sin(rotation),np.cos(rotation)]])
    direction=rot@u/max(np.linalg.norm(u),1e-20)
    shift=(u@direction)*direction-u
    directed=initial.reshape(-1,3).copy()
    directed[-1]+=shift[0]*horizontal+shift[1]*vertical
    best=None
    for trial in range(8):
        start=directed.ravel().copy()+(rng.normal(size=len(initial))*.15 if trial else 0)
        fit=minimize(lambda x:float(np.sum((x-initial)**2)+10*np.sum((x[-3:]-initial[-3:])**2)),start,method='SLSQP',
                     bounds=bounds,constraints={'type':'ineq','fun':constraints},
                     options={'maxiter':500,'ftol':1e-10})
        score=float(constraints(fit.x).min())
        if np.isfinite(fit.x).all() and score>=-1e-7:
            best=decode(fit.x);break
    if best is None:
        raise ValueError('No bounded upper-front ear refinement passed all constraints')
    packed=(best/1000).astype(np.float32).astype(float)*1000
    if not np.isfinite(packed).all():
        raise ValueError('Non-finite quantized ear geometry')
    if np.linalg.norm(packed[free[:-1]]-original[free[:-1]],axis=1).max()>radius:
        raise ValueError('Float32 source displacement exceeded inherited bound')
    if np.linalg.norm(packed[26]-original[26])>1:
        raise ValueError('Upper outer support exceeded one millimeter design limit')
    if np.linalg.norm(packed[42]-packed[[37,36,39,27]].mean(0))>1:
        raise ValueError('Float32 attachment center exceeded inherited bound')
    d,a,ar,o,cd=measures(packed)
    if not all(np.isfinite(v).all() for v in (d,a,ar,o)):
        raise ValueError('Non-finite refined ear quality')
    pr=review(packed,[19,21]);ap=review(packed,[31,32,33]);ac=corners(packed[quads[31:34]])
    if pr[4:8].min()<previous_front[4:8].min()-.01 or pr[12:16].min()<previous_front[12:16].min()-.01:
        raise ValueError('Upper cap corner became narrower after refinement')
    if pr.min()<6.09 or pr.max()>167 or cd<.65 or ap.min()<9.9 or ap.max()>160 or ac.min()<20 or ac.max()>145:
        raise ValueError('Float32 upper front or attachment quality regressed: '+str((pr.min(),pr.max(),cd,ap.min(),ap.max(),ac.min(),ac.max())))
    if d.min()<.49 or a.min()<8 or a.max()>160 or o.min()<.14 or ar.min()<=1e-8:
        raise ValueError('Full ear quality regressed during upper-front refinement')
    output=best.copy();output[:,0]*=-side;output/=1000
    fixed=[i for i in result['fixed_vertex_ids'] if i!=26]
    result['fixed_vertex_ids']=fixed
    if not np.array_equal(output[fixed],np.asarray(result['vertices'])[fixed]):
        # Round-trip scale conversion must not alter any pinned source point.
        output[fixed]=np.asarray(result['vertices'])[fixed]
    unchanged=np.setdiff1d(np.arange(len(points)),free)
    output[unchanged]=np.asarray(result['vertices'])[unchanged]
    result['vertices']=output.tolist();result['min_diagonal_pair_dot']=float(d.min())
    result['source_fit_max_displacement_mm']=float(np.linalg.norm(best[:40]-original,axis=1).max())
    result['source_continuity'].update(upper_cap_normal_dot=cd,
        upper_front_review_min_corner_degrees=float(pr.min()),upper_front_review_max_corner_degrees=float(pr.max()))
    result['upper_attachment_flow'].update(minimum_pair_dot=float(d[31:34].min()),maximum_triangle_angle=float(a[31:34].max()),
        maximum_quad_corner_angle=float(ac.max()),minimum_quad_corner_angle=float(ac.min()),
        review_minimum_corner_angle=float(ap.min()),review_maximum_corner_angle=float(ap.max()))
    result['upper_front_refinement']={'source_bound_m':radius/1000,'outer_support_limit_m':.001,'released_outer_support_local':26,
        'attachment_vertices_unchanged':True,'topology_unchanged':True,
        'authored_uv_unchanged':True,'max_additional_move_m':float(np.linalg.norm(best-points,axis=1).max()/1000),
        'minimum_orientation':float(o.min()),'solver':'scipy SLSQP with post-validation'}
    return result

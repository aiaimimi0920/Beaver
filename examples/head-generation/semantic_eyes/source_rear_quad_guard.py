"""Keep valid source-quad diagonals valid during the rear depth deformation."""
import inspect


def source_quad_normal_dots(points,ids):
    from math import sqrt
    p=[points[i] for i in ids]
    normals=[]
    for a,b,c in ((0,1,2),(0,2,3),(0,1,3),(1,2,3)):
        u=[p[b][i]-p[a][i] for i in range(3)]
        v=[p[c][i]-p[a][i] for i in range(3)]
        n=(u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0])
        length=sqrt(sum(x*x for x in n))
        if length<=1e-14:return (-1.,-1.)
        normals.append(tuple(x/length for x in n))
    return tuple(sum(a*b for a,b in zip(normals[i],normals[i+1])) for i in (0,2))


def clamp_rear_quad_depth(data,before):
    proposed=[tuple(v.co) for v in data.vertices]
    changed={i for i,(a,b) in enumerate(zip(before,proposed)) if a!=b}
    regions=data.attributes['NormalRegion']
    protected=[]
    for p in data.polygons:
        if len(p.vertices)!=4 or regions.data[p.index].value not in (1,2) or not changed.intersection(p.vertices):continue
        ids=tuple(p.vertices);old=source_quad_normal_dots(before,ids)
        if max(old)<=0:raise ValueError('Invalid source quad before rear depth operation')
        choices=(0,1) if min(old)>0 else (0,) if old[0]>old[1] else (1,)
        protected.append((ids,[(i,min(.5,.99*old[i])) for i in choices]))
    for factor in [2.**(-k) for k in range(17)]+[0.]:
        for i in changed:data.vertices[i].co.y=before[i][1]+factor*(proposed[i][1]-before[i][1])
        data.update();actual=[tuple(v.co) for v in data.vertices]
        valid=True
        for ids,checks in protected:
            dots=source_quad_normal_dots(actual,ids)
            if any(dots[i]<floor for i,floor in checks):valid=False;break
        if valid:
            return factor,{'accepted_fraction':factor,'protected_quads':len(protected),
                           'maximum_applied_depth_m':max((abs(actual[i][1]-before[i][1]) for i in changed),default=0.)}
    raise ValueError('Unable to retain valid stored source quad geometry')


def adapt(sources):
    out=dict(sources);text=out['ear_boundary.py']
    start=text.index('def fit_ear_to_jaw_curve(')
    end=text.index('\n\ndef fair_ear_jaw_rim',start)
    function=text[start:end]
    anchor='    data.update()\n    for p,n in zip(data.polygons,normals):'
    assert function.count(anchor)==1
    function=function.replace(anchor,'    factor,quad_guard=clamp_rear_quad_depth(data,before)\n    maximum*=factor\n'+anchor,1)
    function=function.replace('shifted.append([p[0],p[1]+d,p[2]])','shifted.append([p[0],p[1]+factor*d,p[2]])')
    function=function.replace("return {'changed_vertices':len(changed)","return {'source_quad_guard':quad_guard,'changed_vertices':len(changed) if factor else 0")
    out['ear_boundary.py']=text[:start]+function+text[end:]+'\n\n'+inspect.getsource(source_quad_normal_dots)+'\n\n'+inspect.getsource(clamp_rear_quad_depth)
    return out

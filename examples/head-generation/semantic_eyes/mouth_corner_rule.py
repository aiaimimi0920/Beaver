"""CPU proposal: current lower-mouth support redistribution; no saved index selectors.

Source-space metres, X lateral/Y depth/Z height. Caller provides current semantic
oral aperture and protected first support/nasal/chin/ear ownership. This does not
regenerate expressions or authorize native edits; caller must validate those.
"""
from collections import Counter,defaultdict,deque
import numpy as np
from scipy.optimize import minimize
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_regular_triples,orbital_turns,orbital_quad_quality

def bind_lower_mouth_corner(vertices,faces,regions,oral,protected):
    v=np.asarray(vertices,float);faces=[list(map(int,q))for q in faces]
    neighbors=defaultdict(set)
    for q,owner in zip(faces,regions):
        if owner==1:
            for a,b in zip(q,q[1:]+q[:1]):neighbors[a].add(b);neighbors[b].add(a)
    aperture={int(i)for i in oral};positive={i for i in aperture if v[i,0]>0}
    if len(positive)<4:raise ValueError('Missing current semantic oral aperture')
    corner=max(positive,key=lambda i:v[i,0]);distance={i:0 for i in aperture};queue=deque(sorted(aperture))
    while queue:
        i=queue.popleft()
        for j in neighbors[i]:
            if j not in distance:distance[j]=distance[i]+1;queue.append(j)
    triples,_=orbital_regular_triples(v,faces,set(range(len(v))));opposite={}
    for a,b,c in triples:opposite[a,b]=c;opposite[c,b]=a
    options=[]
    for a,b,c in triples:
        if distance.get(b)!=2 or not(v[b,0]>v[corner,0] and v[b,2]<v[corner,2]):continue
        if {distance.get(a),distance.get(c)}!={1,3}:continue
        inward,outward=(a,c)if distance[a]==1 else(c,a)
        options.append((float(orbital_turns(v,[(inward,b,outward)])[0]),inward,b,outward))
    options.sort(reverse=True)
    if not options or options[0][0]<15 or(len(options)>1 and options[0][0]<1.5*options[1][0]):raise ValueError('Ambiguous current lower-mouth turn')
    _,first,second,third=options[0];outer=opposite.get((second,third))
    if outer is None or distance.get(outer)!=4:raise ValueError('Missing same-rail mouth-cheek continuation')
    moved=[second,third]
    if set(moved)&set(protected):raise ValueError('Protected mouth support selected')
    rings=[]
    for middle in moved:
        candidates=[t for t in triples if t[1]==middle and all(distance.get(j)==distance[middle]for j in t)]
        if len(candidates)!=1:raise ValueError('Ambiguous circumoral support continuation')
        rings.append(candidates[0])
    selected=[i for i,q in enumerate(faces)if set(q)&set(moved)]
    selected.sort(key=lambda i:tuple(sorted(tuple(v[j])for j in faces[i])))
    if len(selected)!=6 or any(regions[i]!=1 or len(faces[i])!=4 for i in selected):raise ValueError('Unsupported six-quad source star')
    return {'moved':moved,'path':[first,second,third,outer],'rings':rings,'selected':selected}

def propose_lower_mouth_corner(vertices,faces,regions,oral,protected,*,interface_budget_deg=2.):
    if not np.isfinite(interface_budget_deg)or not(0<interface_budget_deg<=2):raise ValueError('Unsupported interface budget')
    old=np.asarray(vertices,float);faces=[list(map(int,q))for q in faces]
    if not np.isfinite(old).all():raise ValueError('Nonfinite source')
    roles=bind_lower_mouth_corner(old,faces,regions,oral,protected);moved=roles['moved'];path=roles['path'];rings=roles['rings'];selected=roles['selected'];quads=np.array([faces[i]for i in selected])
    counts=Counter(tuple(sorted((a,b)))for q in quads.tolist()for a,b in zip(q,q[1:]+q[:1]));boundary={i for e,n in counts.items()if n==1 for i in e}
    if len(boundary)!=10 or set(quads.ravel())-boundary!=set(moved)or max(counts.values())>2:raise ValueError('Unsupported mouth disk boundary')
    boundary_degree=Counter(i for edge,n in counts.items()if n==1 for i in edge)
    if any(n!=2 for n in boundary_degree.values())or len(set(quads.ravel()))-len(counts)+len(quads)!=1:raise ValueError('Non-disk source interface')
    support=list(zip(path,path[1:],path[2:]));supportsets={frozenset(t)for t in support};triples,_=orbital_regular_triples(old,faces,set(moved));ports=[t for t in triples if frozenset(t)not in supportsets]
    source=AuthorQuadSurface(old,quads,[1]*6);a0,r0=orbital_quad_quality(old,quads);port0=orbital_turns(old,ports)
    lengths=lambda p,q:np.linalg.norm(np.diff(p[list(q)],axis=0),axis=1)
    density=lambda e:float(e.max()/e.min())
    growth=lambda e:float(max(np.maximum(e[1:]/e[:-1],e[:-1]/e[1:])))
    e0=lengths(old,path);density0=density(e0);growth0=growth(e0);ring0=np.array([density(lengths(old,q))for q in rings]);cache={}
    def sample(z):
        key=tuple(z)
        if key in cache:return cache[key]
        p=old.copy();lineage={}
        try:
            for i,offset in zip(moved,np.asarray(z).reshape(-1,2)):
                ds,w,pt=source.lineage(*(old[i,[0,2]]+offset*.001));p[i]=pt;lineage[i]={'donors':list(ds),'weights':list(w)}
        except ValueError:return None
        cache[key]=(p,lineage);return cache[key]
    def objective(z):
        value=sample(z)
        if value is None:return 1e9
        return float(np.sum((orbital_turns(value[0],support)/20)**2)+.03*np.dot(z,z))
    count=len(moved)+len(ports)+len(quads)*2+4+len(rings)
    def constraints(z):
        value=sample(z)
        if value is None:return np.full(count,-100.)
        p,_=value;angles,ratios=orbital_quad_quality(p,quads);e=lengths(p,path);xy=p[quads][:,:,[0,2]];ee=np.roll(xy,-1,1)-xy;cross=ee[:,:,0]*np.roll(ee[:,:,1],-1,1)-ee[:,:,1]*np.roll(ee[:,:,0],-1,1)
        return np.r_[growth0-1e-4-growth(e),ring0-1e-4-np.array([density(lengths(p,q))for q in rings]),1.5-np.linalg.norm(p[moved]-old[moved],axis=1)*1000,(port0+interface_budget_deg-.001-orbital_turns(p,ports))/10,(angles.min(1)-a0.min())/10,(a0.max()-angles.max(1))/10,r0.max()-ratios.max(),density0-1e-4-density(e),cross.min()*1e5]
    result=minimize(objective,np.zeros(4),method='SLSQP',bounds=[(-1.5,1.5)]*4,constraints=[{'type':'ineq','fun':constraints}],options={'maxiter':100,'ftol':1e-9,'eps':1e-5})
    if not result.success or min(constraints(result.x)) < -1e-7:raise ValueError('Constrained source solve failed')
    p,lineage=sample(result.x);p=p.astype(np.float32).astype(float);angles,ratios=orbital_quad_quality(p,quads);e=lengths(p,path)
    if (max(orbital_turns(p,ports)-port0)>interface_budget_deg or density(e)>density0 or growth(e)>growth0 or any(density(lengths(p,q))>before for q,before in zip(rings,ring0))or angles.min()<a0.min()-1e-4 or angles.max()>a0.max()+1e-4 or ratios.max()>r0.max()+1e-5):raise ValueError('Stored mouth constraints failed')
    if max(orbital_turns(p,support))>=max(orbital_turns(old,support)):raise ValueError('No actual support-turn improvement')
    outside=sorted(set(range(len(old)))-set(moved))
    if not np.array_equal(p[outside],old[outside]):raise ValueError('Unowned source position changed')
    xy=p[quads][:,:,[0,2]];ee=np.roll(xy,-1,1)-xy;cross=ee[:,:,0]*np.roll(ee[:,:,1],-1,1)-ee[:,:,1]*np.roll(ee[:,:,0],-1,1)
    if np.min(cross)<=1e-12:raise ValueError('Stored chart not strictly convex')
    for i,row in lineage.items():
        if not np.array_equal(np.asarray(np.asarray(row['weights'])@old[row['donors']],dtype=np.float32).astype(float),p[i]):raise ValueError('Stored donor mismatch')
    return {'vertices':p,'faces':faces,'lineage':lineage,'roles':roles,'boundary':sorted(boundary),'ports':ports,'port_before':port0.tolist(),'port_after':orbital_turns(p,ports).tolist(),'turns_before':orbital_turns(old,support).tolist(),'turns_after':orbital_turns(p,support).tolist(),'density_before':density0,'density_after':density(e),'growth_before':growth0,'growth_after':growth(e),'ring_before':ring0.tolist(),'ring_after':[density(lengths(p,q))for q in rings],'scope':'CPU current-source proposal only; fixed all topology and exterior; actual source fields, generated mouth expressions, paired export, shape and collision gates required.'}

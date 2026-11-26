"""Bounded mixed-old/new rail MIP. Pure inputs/return value; no case coordinates."""
from collections import defaultdict
import numpy as np
from scipy.optimize import milp, Bounds, LinearConstraint
from scipy.sparse import coo_matrix
from forehead_pattern_catalogue import forehead_pattern_edge, forehead_pattern_edges, forehead_pattern_counts


def forehead_pattern_solve(proposal, catalogue, *, time_limit=60., max_new_points=80,
                           max_route_edges=30, mip_rel_gap=.01, prescribed_routes=None,
                           point_cost=1000., length_cost=100., feasibility_only=False,
                           allowed_transit_by_role=None):
    if time_limit<=0 or max_new_points<0 or max_route_edges<1:raise ValueError('Invalid bounded options')
    pats=catalogue['patterns'];selected=set(catalogue['selected_base_faces']);faces=proposal['faces'];ctx=catalogue['context'];terminals=ctx['terminals'];ends=catalogue['guide_ends'];points=np.asarray(catalogue['points']);cost=[];lb=[];ub=[]
    def variable(c):
        index=len(cost);cost.append(c);lb.append(0.);ub.append(1.);return index
    cuts={e:variable(point_cost)for e in catalogue['interior']};centers={fi:variable(point_cost)for fi in selected};pvars=[variable(p['cost'])for p in pats];byface=defaultdict(list);incidence=defaultdict(list)
    for k,p in enumerate(pats):
        byface[p['face']].append(k)
        for e,n in forehead_pattern_counts(p['children']).items():incidence[e].append((pvars[k],n))
    if set(byface)!=selected:return {'feasible':False,'status':'A selected face has no admissible pattern'}
    exists={e:variable(0.)for e in incidence};adj=defaultdict(set)
    for a,b in incidence:adj[a].add(b);adj[b].add(a)
    fullcenters={catalogue['centers'][p['face']]for p in pats if p['mask']==15};allcenters=set(catalogue['centers'].values());allowed=set(adj)-catalogue['blocked']-(allcenters-fullcenters)
    flows={};incoming={};outgoing={}
    for role,(start,end)in ends.items():
        role_allowed=allowed if allowed_transit_by_role is None else allowed&set(allowed_transit_by_role[role])
        if start not in role_allowed or end not in role_allowed:raise ValueError('Role corridor omits a terminal')
        flows[role]={};incoming[role]=defaultdict(list);outgoing[role]=defaultdict(list)
        for u in sorted((role_allowed-terminals)|{start}):
            for v in sorted((adj[u]&role_allowed-terminals)|({end}if end in adj[u]else set())):
                k=variable(.2+np.linalg.norm(points[u]-points[v])*length_cost);flows[role][u,v]=k;incoming[role][v].append(k);outgoing[role][u].append(k)
    ri=[];ci=[];data=[];lower=[];upper=[]
    def row(terms,a,b):
        j=len(lower);lower.append(a);upper.append(b)
        for k,value in terms:ri.append(j);ci.append(k);data.append(value)
    for fi,pp in byface.items():
        row([(pvars[k],1.)for k in pp],1,1)
        for e in forehead_pattern_edges(faces[fi]):
            if e in cuts:row([(cuts[e],1.)]+[(pvars[k],-1.)for k in pp if e in pats[k]['cuts']],0,0)
        row([(centers[fi],1.)]+[(pvars[k],-1.)for k in pp if pats[k]['center']],0,0)
    for e,terms in incidence.items():row([(exists[e],1. if e in catalogue['boundary'] else 2.)]+[(k,-n)for k,n in terms],0,0)
    vertex_exists={catalogue['mids'][e]:var for e,var in cuts.items()};vertex_exists.update({catalogue['centers'][fi]:var for fi,var in centers.items()})
    for v in sorted(set(adj)-terminals):
        terms=[(k,1.)for role in flows for k in outgoing[role][v]]
        if v in vertex_exists:row(terms+[(vertex_exists[v],-1.)],-np.inf,0.)
        else:row(terms,0,1)
    for role,arcs in flows.items():
        start,end=ends[role]
        for v in sorted(set(incoming[role])|set(outgoing[role])):
            balance=1 if v==start else -1 if v==end else 0
            row([(k,1.)for k in outgoing[role][v]]+[(k,-1.)for k in incoming[role][v]],balance,balance)
        row([(k,1.)for k in arcs.values()],0,max_route_edges)
        for e,k in arcs.items():row([(k,1.),(exists[forehead_pattern_edge(*e)],-1.)],-np.inf,0.)
        for j,p in enumerate(pats):
            for q in p['children']:
                for k,b in enumerate(q):
                    a,c=q[k-1],q[(k+1)%4];terms=[(pvars[j],1.)]+[(arcs[e],1.)for e in ((a,b),(b,a),(b,c),(c,b))if e in arcs]
                    if len(terms)>2:row(terms,0,2)
        for fi,q in enumerate(faces):
            if fi in selected:continue
            for k,b in enumerate(q):
                a,c=q[k-1],q[(k+1)%len(q)];terms=[(arcs[e],1.)for e in ((a,b),(b,a),(b,c),(c,b))if e in arcs]
                if len(terms)>1:row(terms,0,1)
    # Both prefix and suffix attachment turns are guarded, even when the active segment is interior.
    for role,node,neighbor,is_start in ctx['attachments']:
        arcs=flows[role]
        for j,p in enumerate(pats):
            for q in p['children']:
                if node not in q:continue
                k=q.index(node);a,b=q[k-1],q[(k+1)%len(q)]
                if neighbor not in (a,b):continue
                other=b if neighbor==a else a;e=(node,other)if is_start else(other,node)
                if e in arcs:row([(pvars[j],1.),(arcs[e],1.)],0,1)
        for fi,q in enumerate(faces):
            if fi in selected or node not in q:continue
            k=q.index(node);a,b=q[k-1],q[(k+1)%len(q)]
            if neighbor in(a,b):
                other=b if neighbor==a else a;e=(node,other)if is_start else(other,node)
                if e in arcs:row([(arcs[e],1.)],0,0)
    row([(k,1.)for k in cuts.values()]+[(k,1.)for k in centers.values()],0,max_new_points)
    for role,path in (prescribed_routes or {}).items():
        if (path[0],path[-1])!=tuple(ends[role]):raise ValueError('Prescribed route endpoints mismatch')
        for e in zip(path,path[1:]):
            if e not in flows[role]:raise ValueError('Prescribed route includes a blocked or absent arc')
            row([(flows[role][e],1.)],1,1)
    matrix=coo_matrix((data,(ri,ci)),shape=(len(lower),len(cost))).tocsc()
    result=milp(np.zeros(len(cost))if feasibility_only else np.asarray(cost),integrality=np.ones(len(cost)),bounds=Bounds(lb,ub),constraints=LinearConstraint(matrix,lower,upper),options={'time_limit':float(time_limit),'mip_rel_gap':float(mip_rel_gap)})
    out={'feasible':result.x is not None,'status':str(result.message),'scope':'Combinatorial witness only; numerical geometry and field acceptance pending','model_variables':len(cost),'model_rows':len(lower)}
    if result.x is None:return out
    x=np.rint(result.x);value=matrix@x
    if np.max(abs(x-result.x))>1e-5 or np.any(value<np.array(lower)-1e-7)or np.any(value>np.array(upper)+1e-7):raise ValueError('Invalid integer incumbent')
    paths={}
    for role,(start,end)in ends.items():
        successors={u:v for (u,v),k in flows[role].items()if x[k]};path=[start]
        while path[-1]!=end:
            if path[-1]not in successors or successors[path[-1]]in path:raise ValueError('Invalid route extraction')
            path.append(successors[path[-1]])
        paths[role]=path
    out.update(selected_base_faces=sorted(selected),guide_ends=dict(ends),forbidden_old=catalogue['forbidden_old'],patterns=[dict(pats[k],variable=pvars[k])for k in range(len(pats))if x[pvars[k]]],paths=paths,points=catalogue['points'],lineage=catalogue['lineage'],new_midpoints=[catalogue['mids'][e]for e,k in cuts.items()if x[k]],new_centers=[catalogue['centers'][fi]for fi,k in centers.items()if x[k]],gap=float(result.mip_gap))
    return out

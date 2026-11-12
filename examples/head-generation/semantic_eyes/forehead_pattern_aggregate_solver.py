"""Initial-only aggregate-flow MIP. Pure inputs; exact semantic decomposition or rejection."""
from collections import defaultdict
import numpy as np
from scipy.optimize import milp, Bounds, LinearConstraint
from scipy.sparse import coo_matrix
from forehead_pattern_catalogue import forehead_pattern_edge, forehead_pattern_edges, forehead_pattern_counts


def forehead_pattern_solve_aggregate(proposal, catalogue, *, time_limit=60., max_new_points=160,
                                     max_route_edges=50, mip_rel_gap=.01,
                                     point_cost=1000., length_cost=100., feasibility_only=False,
                                     allowed_transit_vertices=None):
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
    if len(ends)!=5 or set(ends)!=set(proposal['paths']):raise ValueError('Initial aggregate requires all five semantic roles')
    if any('transition_rotation'in pattern for pattern in pats):raise ValueError('Initial aggregate axis constraints require ordinary patterns only')
    starts={pair[0]for pair in ends.values()};targets={pair[1]for pair in ends.values()}
    if starts&targets or len(starts)!=len(ends) or len(targets)!=len(ends):raise ValueError('Aggregate terminals must be distinct')
    if allowed_transit_vertices is not None:allowed&=set(allowed_transit_vertices)
    if not terminals<=allowed:raise ValueError('Aggregate corridor omits terminals')
    flows={'aggregate':{}};incoming={'aggregate':defaultdict(list)};outgoing={'aggregate':defaultdict(list)}
    for u in sorted(allowed-targets):
        for v in sorted(adj[u]&allowed-starts):
            if (u in allcenters and v<catalogue['original_count']) or (v in allcenters and u<catalogue['original_count']):continue
            k=variable(.2+np.linalg.norm(points[u]-points[v])*length_cost)
            flows['aggregate'][u,v]=k;incoming['aggregate'][v].append(k);outgoing['aggregate'][u].append(k)
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
        for v in sorted(set(incoming[role])|set(outgoing[role])):
            balance=1 if v in starts else -1 if v in targets else 0
            row([(k,1.)for k in outgoing[role][v]]+[(k,-1.)for k in incoming[role][v]],balance,balance)
        row([(k,1.)for k in arcs.values()],0,max_route_edges*len(ends))
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
    # Valid ordinary-template axis equalities strengthen the LP relaxation.
    # A two/adjacent-cut valence3 center cannot be an opposite-edge transit.
    arcs=flows['aggregate']
    for fi,center in catalogue['centers'].items():
        full=[pvars[j]for j,pattern in enumerate(pats)if pattern['face']==fi and pattern['mask']==15]
        terms=[(k,1.)for k in outgoing['aggregate'][center]]+[(k,-1.)for k in full]
        if terms:row(terms,-np.inf,0.)
        if not full:continue
        mids=[catalogue['mids'][edge]for edge in forehead_pattern_edges(faces[fi])]
        for k,node in enumerate(mids):
            first=(node,center);second=(center,mids[(k+2)%4]);terms=[]
            if first in arcs:terms.append((arcs[first],1.))
            if second in arcs:terms.append((arcs[second],-1.))
            if terms:row(terms,0.,0.)
    for edge,node in catalogue['mids'].items():
        a,b=edge
        for first,second in (((a,node),(node,b)),((b,node),(node,a))):
            terms=[]
            if first in arcs:terms.append((arcs[first],1.))
            if second in arcs:terms.append((arcs[second],-1.))
            if terms:row(terms,0.,0.)
    # Both prefix and suffix attachment turns are guarded, even when the active segment is interior.
    for role,node,neighbor,is_start in ctx['attachments']:
        arcs=flows['aggregate']
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
    matrix=coo_matrix((data,(ri,ci)),shape=(len(lower),len(cost))).tocsc()
    result=milp(np.zeros(len(cost))if feasibility_only else np.asarray(cost),integrality=np.ones(len(cost)),bounds=Bounds(lb,ub),constraints=LinearConstraint(matrix,lower,upper),options={'time_limit':float(time_limit),'mip_rel_gap':float(mip_rel_gap)})
    out={'feasible':result.x is not None,'status':str(result.message),'scope':'Combinatorial witness only; numerical geometry and field acceptance pending','model_variables':len(cost),'model_rows':len(lower)}
    if result.x is None:return out
    x=np.rint(result.x);value=matrix@x
    if np.max(abs(x-result.x))>1e-5 or np.any(value<np.array(lower)-1e-7)or np.any(value>np.array(upper)+1e-7):raise ValueError('Invalid integer incumbent')
    selected_arcs={edge for edge,k in flows['aggregate'].items()if x[k]}
    try:paths=forehead_pattern_decompose_aggregate(ends,selected_arcs,max_route_edges)
    except ValueError as error:
        out.update(feasible=False,integer_incumbent=True,decomposition_rejected=str(error));return out
    out['aggregate_decomposition']={'role_count':len(paths),'exact_semantic_targets':True,'vertex_disjoint':True,'no_extra_cycles':True}
    out.update(selected_base_faces=sorted(selected),guide_ends=dict(ends),forbidden_old=catalogue['forbidden_old'],patterns=[dict(pats[k],variable=pvars[k])for k in range(len(pats))if x[pvars[k]]],paths=paths,points=catalogue['points'],lineage=catalogue['lineage'],new_midpoints=[catalogue['mids'][e]for e,k in cuts.items()if x[k]],new_centers=[catalogue['centers'][fi]for fi,k in centers.items()if x[k]],gap=float(result.mip_gap))
    return out


def forehead_pattern_decompose_aggregate(guide_ends, selected_arcs, max_route_edges):
    """Reject swapped ownership, branching, collisions, missing routes and cycles."""
    selected_arcs=set(map(tuple,selected_arcs));starts={a for a,b in guide_ends.values()};targets={b for a,b in guide_ends.values()}
    if starts&targets or len(starts)!=len(guide_ends) or len(targets)!=len(guide_ends):raise ValueError('Aggregate terminals are not distinct')
    successors={};incoming={}
    for a,b in selected_arcs:
        if a in successors or b in incoming:raise ValueError('Aggregate branching or collision')
        successors[a]=b;incoming[b]=a
    paths={};used_arcs=set();used_vertices=set()
    for role,(start,end)in guide_ends.items():
        path=[start]
        while path[-1]not in targets:
            node=path[-1]
            if node not in successors or successors[node]in path:raise ValueError('Missing successor or route cycle')
            path.append(successors[node])
        if path[-1]!=end:raise ValueError('Aggregate route swaps semantic terminal ownership')
        if len(path)-1>max_route_edges:raise ValueError('Aggregate individual edge cap exceeded')
        if used_vertices&set(path):raise ValueError('Aggregate paths are not vertex-disjoint')
        used_vertices.update(path);used_arcs.update(zip(path,path[1:]));paths[role]=path
    if len(paths)!=len(guide_ends) or used_arcs!=selected_arcs:raise ValueError('Aggregate has missing paths or extra cycles')
    return paths

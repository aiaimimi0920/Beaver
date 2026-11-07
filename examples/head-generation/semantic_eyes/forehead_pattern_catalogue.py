"""Pure conforming quad-pattern catalogue; no case data, IO, or string execution.
Candidate births are previews, not a reset of existing frozen lineage.
"""
import copy
from collections import Counter, defaultdict
import numpy as np


def forehead_pattern_edge(a, b):
    return tuple(sorted((int(a), int(b))))


def forehead_pattern_edges(face):
    return [forehead_pattern_edge(a,b) for a,b in zip(face,face[1:]+face[:1])]


def forehead_pattern_counts(faces):
    return Counter(e for q in faces for e in forehead_pattern_edges(q))


def forehead_pattern_guide_corners(faces, path):
    turns=defaultdict(set)
    for a,b,c in zip(path,path[1:],path[2:]):turns[b].add(frozenset((a,c)))
    return [(fi,v)for fi,q in enumerate(faces)for k,v in enumerate(q)
            if frozenset((q[k-1],q[(k+1)%len(q)]))in turns[v]]



def forehead_pattern_hexagon(children, pair, shared):
    outgoing={}
    for j in pair:
        q=children[j]
        for a,b in zip(q,q[1:]+q[:1]):
            if forehead_pattern_edge(a,b)==shared:continue
            if a in outgoing:raise ValueError('Non-simple directed pair boundary')
            outgoing[a]=b
    if len(outgoing)!=6 or len(set(outgoing.values()))!=6:raise ValueError('Not a hexagon')
    h=[shared[0]]
    for _ in range(5):h.append(outgoing[h[-1]])
    if len(set(h))!=6 or outgoing[h[-1]]!=h[0]:raise ValueError('Not one hexagonal cycle')
    return h


def forehead_pattern_context(paths, segments):
    """segments maps semantic role to (existing start, existing end), preserving tails."""
    fixed_nodes=set();fixed_edges=set();active_nodes=set();attachments=[]
    terminals={v for pair in segments.values()for v in pair}
    if len(terminals)!=2*len(segments):raise ValueError('Segment terminals must be distinct')
    for role,path in paths.items():
        if len(path)!=len(set(path)):raise ValueError('Repeated guide vertex')
        if role not in segments:
            fixed_nodes.update(path);fixed_edges.update(forehead_pattern_edges(path)[:-1]);continue
        first,last=segments[role];a,b=path.index(first),path.index(last)
        if a>=b:raise ValueError('Active segment must follow guide order')
        active_nodes.update(path[a:b+1]);fixed_nodes.update(path[:a]+path[b+1:])
        fixed_edges.update(forehead_pattern_edge(u,v)for k,(u,v)in enumerate(zip(path,path[1:]))if not a<=k<b)
        if a:attachments.append((role,first,path[a-1],True))
        if b+1<len(path):attachments.append((role,last,path[b+1],False))
    return {'terminals':terminals,'fixed_nodes':fixed_nodes,'fixed_edges':fixed_edges,
            'active_nodes':active_nodes,'attachments':attachments}


def forehead_pattern_select_cavity(faces, paths, segments, eligible_faces, halo_layers=2):
    """Eligibility is supplied from semantic roles; no hemisphere/region assumptions."""
    context=forehead_pattern_context(paths,segments);eligible=set(eligible_faces)
    incidence=defaultdict(set)
    for fi,q in enumerate(faces):
        for e in forehead_pattern_edges(q):incidence[e].add(fi)
    selected={fi for fi in eligible if set(faces[fi])&context['active_nodes']}
    for _ in range(halo_layers):
        selected|={j for fi in list(selected)for e in forehead_pattern_edges(faces[fi])for j in incidence[e]if j in eligible}
    return sorted(selected)


def forehead_pattern_catalogue(proposal, selected_faces, current_chart, sample_birth,
                               source_discrete, neutral_parent_edges, segments,
                               blocked_vertices=(), protected_vertices=(), protected_edges=(), quality_cost=None, allowed_protected_transits=()):
    """sample_birth(chart_point)->(position, {donors,weights}) or None.
    source_discrete: original-author point arrays only. current_chart: all current
    vertices, arbitrary vector dimension, interpreted only by caller's sampler.
    Source face/edge owners remain in proposal['topology']; no mutation occurs.
    """
    faces=proposal['faces'];points=copy.deepcopy(proposal['vertices']);n=len(points)
    selected=sorted(set(selected_faces));chart=np.asarray(current_chart,float)
    if len(chart)!=n:raise ValueError('Current chart length mismatch')
    if any(len(faces[fi])!=4 for fi in selected):raise ValueError('Only quad cavity supported')
    context=forehead_pattern_context(proposal['paths'],segments)
    context['fixed_edges']|={forehead_pattern_edge(*e)for e in protected_edges}
    if context['terminals']&set(blocked_vertices):raise ValueError('Blocked terminal requires a different segment')
    counts=forehead_pattern_counts([faces[fi]for fi in selected]);boundary={e for e,c in counts.items()if c==1};interior=sorted(e for e,c in counts.items()if c==2)
    if any(c>2 for c in counts.values()):raise ValueError('Nonmanifold cavity')
    parent={forehead_pattern_edge(*z['edge']):z['parent']for z in proposal['topology']['edge_lineage']}
    neutral={forehead_pattern_edge(*e)for e in neutral_parent_edges}
    lineage={int(i):copy.deepcopy(row)for i,row in proposal['lineage'].items()};births={};birth_parameters={};invalid=set();mids={};centers={}
    discrete={k:list(v)for k,v in source_discrete.items()if k!='.select_vert'}
    original_counts={len(values)for values in discrete.values()}
    if len(original_counts)>1:raise ValueError('Inconsistent original discrete fields')
    original_count=next(iter(original_counts),n)
    frozen={int(i):row for i,row in proposal['topology']['lineage'].items()}
    def point_values(row):
        out={}
        for key,values in source_discrete.items():
            if key=='.select_vert':continue
            choices={values[d]for d,w in zip(row['donors'],row['weights'])if w>0}
            if len(choices)!=1:return None
            out[key]=next(iter(choices))
        return out
    for i in range(original_count,n):
        values=point_values(frozen[i])
        if values is None:raise ValueError('Ambiguous existing discrete field')
        for key,value in values.items():discrete[key].append(value)
    def allocate(parameter,fallback):
        node=len(points);birth_parameters[node]=np.asarray(parameter,float).tolist();answer=sample_birth(np.asarray(parameter).copy())
        if answer is None:invalid.add(node);points.append(list(fallback));return node
        position,row=answer;row=copy.deepcopy(row)
        if len(row['donors'])!=len(row['weights'])or abs(sum(row['weights'])-1)>1e-7 or min(row['weights'])<-1e-10:raise ValueError('Invalid birth interpolation')
        values=point_values(row)
        if values is None:invalid.add(node)
        points.append(np.asarray(position,float).tolist());births[node]=row;lineage[node]=row
        return node
    for e in interior:mids[e]=allocate(chart[list(e)].mean(0),np.asarray(points)[list(e)].mean(0))
    for fi in selected:centers[fi]=allocate(chart[faces[fi]].mean(0),np.asarray(points)[faces[fi]].mean(0))
    patterns=[]
    for fi in selected:
        q=list(faces[fi]);ee=forehead_pattern_edges(q)
        for mask in range(16):
            bits=[k for k in range(4)if mask>>k&1]
            if len(bits)%2 or any(ee[k]not in mids or ee[k]in context['fixed_edges'] for k in bits):continue
            if any(any(values[a]!=values[b]for values in discrete.values())for a,b in (ee[k]for k in bits)):continue
            center=False
            if not bits:children=[q]
            elif len(bits)==2 and (bits[1]-bits[0])%4==2:
                k=bits[0];a,b,c,d=q[k:]+q[:k];u=mids[forehead_pattern_edge(a,b)];v=mids[forehead_pattern_edge(c,d)];children=[[a,u,v,d],[u,b,c,v]]
            elif len(bits)==2:
                pivot=q.index(next(iter(set(ee[bits[0]])&set(ee[bits[1]]))));d,a,b,c=q[pivot:]+q[:pivot];u=mids[forehead_pattern_edge(d,c)];v=mids[forehead_pattern_edge(d,a)];z=centers[fi];children=[[a,b,z,v],[b,c,u,z],[z,u,d,v]];center=True
            else:
                z=centers[fi];children=[[v,mids[forehead_pattern_edge(v,q[(k+1)%4])],z,mids[forehead_pattern_edge(q[k-1],v)]]for k,v in enumerate(q)];center=True
            if any(v in invalid for child in children for v in child):continue
            if center and any(len({values[v]for v in q})>1 for values in discrete.values()):continue
            # Birth semantics must agree with each generating edge/face, not merely be internally uniform.
            expected={key:values[q[0]]for key,values in discrete.items()} if center else None
            if center and point_values(births[centers[fi]])!=expected:continue
            if any(point_values(births[mids[ee[k]]])!={key:values[ee[k][0]]for key,values in discrete.items()}for k in bits):continue
            cost=0. if quality_cost is None else float(quality_cost(np.asarray(points),children))
            patterns.append({'face':fi,'mask':mask,'cuts':[ee[k]for k in bits],'center':center,'children':children,'cost':cost})
    true_boundary={v for e,c in forehead_pattern_counts(faces).items()if c==1 for v in e}
    coordinate_pins=set(proposal.get('protected',()))|set(protected_vertices)
    blocked=((coordinate_pins-set(allowed_protected_transits))|context['fixed_nodes']|true_boundary)-context['terminals'];blocked|=set(blocked_vertices)
    allowed_old=set(range(n))-blocked;mid_to_edge={i:e for e,i in mids.items()};extra=[]
    for pat in patterns:
        if pat['mask']!=15:continue
        incidence=defaultdict(list)
        for j,q in enumerate(pat['children']):
            for e in forehead_pattern_edges(q):incidence[e].append(j)
        for edge,pair in incidence.items():
            if len(pair)!=2:continue
            cut=next(v for v in edge if v in mid_to_edge);par=parent[mid_to_edge[cut]]
            if par is not None and forehead_pattern_edge(*par)not in neutral:continue
            h=forehead_pattern_hexagon(pat['children'],pair,edge)
            for shift in (1,2):
                ring=h[shift:]+h[:shift];added=[ring[0],ring[3]]
                if not set(added)&allowed_old:continue
                cc=copy.deepcopy(pat['children']);cc[pair[0]]=ring[:4];cc[pair[1]]=ring[3:]+ring[:1]
                extra.append(dict(pat,children=cc,cost=10.,transition_rotation={'removed':list(edge),'added':added,'source_parent':par}))
    patterns.extend(extra)
    return {'selected_base_faces':selected,'patterns':patterns,'points':points,'lineage':lineage,'births':births,'birth_parameters':birth_parameters,'mids':mids,'centers':centers,'boundary':boundary,'interior':interior,'blocked':blocked,'context':context,'guide_ends':dict(segments),'forbidden_old':sorted(blocked_vertices),'original_count':n}

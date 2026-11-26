"""Pure append-only materialization with immutable existing birth rows."""
import copy
from collections import defaultdict
import numpy as np
from forehead_pattern_catalogue import forehead_pattern_edge, forehead_pattern_edges, forehead_pattern_counts, forehead_pattern_guide_corners



def forehead_pattern_plain_records(value):
    """Own mutable construction records without a JSON round trip.

    NumPy scalars/arrays and tuple/list rows become value-equivalent Python
    scalars/lists. Dictionary keys are preserved, including source edge tuples.
    Inputs and nested aliases are never mutated; numerical values are not rounded.
    """
    if isinstance(value, np.ndarray):
        return forehead_pattern_plain_records(value.tolist())
    if isinstance(value, np.generic):
        return value.item()
    if isinstance(value, dict):
        return {key: forehead_pattern_plain_records(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [forehead_pattern_plain_records(item) for item in value]
    return copy.deepcopy(value)


def forehead_pattern_validate_parent_chains(provenance, source_edge_fields):
    children=defaultdict(list);seen=set();known={forehead_pattern_edge(*e)for e in source_edge_fields}
    for row in provenance:
        edge=forehead_pattern_edge(*row['edge'])
        if edge in seen:raise ValueError('Duplicate edge provenance')
        seen.add(edge)
        if row['parent']is not None:
            par=forehead_pattern_edge(*row['parent'])
            if par not in known:raise ValueError('Non-original source edge parent')
            children[par].append(edge)
    count=0
    for raw_parent,raw_fields in source_edge_fields.items():
        parent=forehead_pattern_edge(*raw_parent);fields=tuple(raw_fields)
        if fields==(0.,0,False,True):continue
        count+=1;adj=defaultdict(set)
        for a,b in children[parent]:adj[a].add(b);adj[b].add(a)
        a,b=parent
        if len(adj[a])!=1 or len(adj[b])!=1 or any(len(v)!=2 for i,v in adj.items()if i not in parent):raise ValueError('Lost or branched nonneutral source chain')
        reached={a};queue=[a]
        for node in queue:
            for neighbor in adj[node]-reached:reached.add(neighbor);queue.append(neighbor)
        if b not in reached or reached!=set(adj):raise ValueError('Disconnected source chain')
    return count


def forehead_pattern_materialize(proposal, witness, source_vertices, source_open_delta,
                                 source_discrete, source_edge_fields, neutral_parent_edges,
                                 *, birth_rows=None, protected_edges=(), nonneutral_birth_validator=None):
    """Returns (new_proposal,audit). birth_rows overrides ONLY newly allocated IDs.
    All numerical/UV/native geometry gates remain external and mandatory.
    Existing current positions and both current/frozen lineage are copied exactly.
    """
    proposal = forehead_pattern_plain_records(proposal)
    witness = forehead_pattern_plain_records(witness)
    birth_rows = forehead_pattern_plain_records(birth_rows)
    source_discrete = forehead_pattern_plain_records(source_discrete)
    if not witness.get('feasible'):raise ValueError('No combinatorial witness')
    p=copy.deepcopy(proposal);t=copy.deepcopy(p['topology']);p['topology']=t
    # Value-equivalent builder arrays may alias; each appended row domain owns its list.
    for obj,key in ((p,'vertices'),(p,'open_delta'),(t,'vertices'),(t,'chart')):obj[key]=copy.deepcopy(obj[key])
    n=len(p['vertices']);nf=len(p['faces']);faces=copy.deepcopy(p['faces']);owners=list(t['face_owners']);source=np.asarray(source_vertices);delta=np.asarray(source_open_delta);chart=np.asarray(p['source_chart'])
    if not np.array_equal(np.asarray(witness['points'])[:n],np.asarray(p['vertices'])):raise ValueError('Witness uses a different base')
    for obj in (p,t):obj['lineage']={int(k):v for k,v in obj['lineage'].items()}
    frozen=copy.deepcopy(t['lineage']);current=copy.deepcopy(p['lineage']);top_vertices=copy.deepcopy(t['vertices']);top_chart=copy.deepcopy(t['chart'])
    rows={int(k):v for k,v in witness['lineage'].items()};overrides={int(k):v for k,v in (birth_rows or {}).items()}
    newids=sorted(set(witness['new_midpoints'])|set(witness['new_centers']))
    if any(i<n for i in newids)or set(overrides)-set(newids):raise ValueError('Birth override may not reset an existing point')
    rows.update(overrides);remap={i:i for i in range(n)};remap.update({i:n+k for k,i in enumerate(newids)})
    counts=forehead_pattern_counts([faces[fi]for fi in witness['selected_base_faces']]);interior=sorted(e for e,c in counts.items()if c==2);mids={e:n+k for k,e in enumerate(interior)}
    parent={forehead_pattern_edge(*z['edge']):z['parent']for z in t['edge_lineage']};neutral={forehead_pattern_edge(*e)for e in neutral_parent_edges};mid_to_edge={i:e for e,i in mids.items()}
    for pat in witness['patterns']:
        if 'interior_rotation'in pat:raise ValueError('Two-face composite rotations are not in this proven API')
        if 'transition_rotation'in pat:
            removed=pat['transition_rotation']['removed'];cut=next((i for i in removed if i in mid_to_edge),None)
            if cut is None:raise ValueError('Invalid generated-spoke rotation')
            par=parent[mid_to_edge[cut]]
            if par is not None and forehead_pattern_edge(*par)not in neutral:raise ValueError('Would bend a nonneutral parent')
    for i in newids:
        if i in overrides and i in mid_to_edge:
            par=parent[mid_to_edge[i]]
            if par is not None and forehead_pattern_edge(*par)not in neutral and overrides[i]!=witness['lineage'].get(i,witness['lineage'].get(str(i))):
                if nonneutral_birth_validator is None or not nonneutral_birth_validator(i,mid_to_edge[i],par,overrides[i]):raise ValueError('Nonneutral birth relocation requires an explicit source-chain locus validator')
        row=rows[i];donors=np.asarray(row['donors'],int);weights=np.asarray(row['weights'],float)
        if len(donors)!=len(weights)or len(set(donors))!=len(donors)or np.any(donors<0)or np.any(donors>=len(source))or not np.isfinite(weights).all()or np.any(weights<0)or abs(weights.sum()-1)>1e-8:raise ValueError('Invalid author birth interpolation')
        for key,values in source_discrete.items():
            choices = [values[d] for d, weight in zip(donors, weights) if weight > 0]
            if key != '.select_vert' and (not choices or any(value != choices[0] for value in choices)):
                raise ValueError('New birth crosses discrete semantics')
        pos=np.asarray(weights@source[donors],np.float32).astype(float).tolist();record=copy.deepcopy(row)
        p['vertices'].append(pos);p['open_delta'].append((weights@delta[donors]).tolist());p['lineage'][remap[i]]=copy.deepcopy(record)
        t['vertices'].append(pos);t['chart'].append((weights@chart[donors]).tolist());t['lineage'][remap[i]]=record
    for e,mid in mids.items():
        if mid not in newids:continue
        par=parent.pop(e)
        for a in e:parent[forehead_pattern_edge(a,remap[mid])]=par
    changed=[];pattern_faces=set()
    for pat in witness['patterns']:
        fi=pat['face']
        if fi in pattern_faces:raise ValueError('Multiple selected patterns for one face')
        pattern_faces.add(fi);children=[[remap[i]for i in q]for q in pat['children']]
        if pat['mask']:changed.append(fi)
        p['faces'][fi]=children[0];p['faces'].extend(children[1:]);t['face_owners'].extend([owners[fi]]*(len(children)-1))
        for q in children:
            for e in forehead_pattern_edges(q):parent.setdefault(e,None)
    if pattern_faces!=set(witness['selected_base_faces']):raise ValueError('Incomplete cavity cover')
    for role,local in witness['paths'].items():
        first,last=witness['guide_ends'][role];path=p['paths'][role];a,b=path.index(first),path.index(last)
        if local[0]!=first or local[-1]!=last:raise ValueError('Route endpoint mismatch')
        if any(i in set(witness.get('forbidden_old',()))for i in local):raise ValueError('Route uses forbidden field blocker')
        p['paths'][role]=path[:a]+[remap[i]for i in local]+path[b+1:]
    t['paths']=copy.deepcopy(p['paths']);t['faces']=copy.deepcopy(p['faces']);t['edge_lineage']=[{'edge':list(e),'parent':v}for e,v in parent.items()]
    before=forehead_pattern_counts(faces);after=forehead_pattern_counts(p['faces'])
    if any(forehead_pattern_edge(*e)not in after for e in protected_edges):raise ValueError('Split or removed an exact protected edge')
    if max(after.values())>2 or set(after)!=set(parent):raise ValueError('Nonmanifold topology or invalid edge provenance')
    if {e for e,c in before.items()if c==1}!={e for e,c in after.items()if c==1}:raise ValueError('Changed protected mesh boundary')
    if len(p['vertices'])-len(after)+len(p['faces'])!=n-len(before)+nf:raise ValueError('Changed Euler characteristic')
    allguide=[i for path in p['paths'].values()for i in path]
    if len(allguide)!=len(set(allguide)):raise ValueError('Guide collision or repeated vertex')
    for path in p['paths'].values():
        if any(forehead_pattern_edge(a,b)not in after for a,b in zip(path,path[1:]))or forehead_pattern_guide_corners(p['faces'],path):raise ValueError('Guide is absent or has a quad elbow')
    if any(t['lineage'][i]!=row for i,row in frozen.items())or any(p['lineage'][i]!=row for i,row in current.items()):raise ValueError('Existing lineage reset')
    if t['vertices'][:n]!=top_vertices or t['chart'][:n]!=top_chart or p['vertices'][:n]!=proposal['vertices']:raise ValueError('Existing coordinates reset')
    chain_count=forehead_pattern_validate_parent_chains(t['edge_lineage'],source_edge_fields)
    p['affected_new']=sorted(set(p.get('affected_new',()))|set(changed)|set(range(nf,len(p['faces']))));p['affected_old']=sorted(set(p.get('affected_old',()))|{owners[i]for i in changed})
    t['retiled_new_faces']=sorted(set(t.get('retiled_new_faces',()))|set(changed)|set(range(nf,len(p['faces']))));t['retiled_old_faces']=sorted(set(t.get('retiled_old_faces',()))|{owners[i]for i in changed})
    p['moving']=sorted(set(p.get('moving',()))|set(range(n,len(p['vertices'])))|({i for path in p['paths'].values()for i in path}-set(p.get('protected',()))));p['support']=sorted(set(p['moving'])-set(allguide))
    return p,{'scope':'Graph/birth contract only; all geometry, field, UV and independent1mm gates still required','new_points':len(newids),'parent_chains':chain_count,'old_positions_exact':True,'old_current_lineage_exact':True,'old_frozen_lineage_exact':True,'old_topology_chart_exact':True,'guide_disjoint_no_elbows':True,'allocation_to_compact':{i:remap[i]for i in newids}}

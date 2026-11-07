"""Fail-closed support-operator admission. No I/O or native/model acceptance."""
from forehead_pattern_materialize import forehead_pattern_plain_records
import numpy as np
from forehead_flow_rule import forehead_chart,forehead_convex_chart,forehead_no_chart_crossings
from forehead_spacing import forehead_frozen_field
from forehead_retile_fields import validate_forehead_edge_chains
from forehead_star_topology import forehead_edge_counts,forehead_guide_corners
from orbital_support_rule import orbital_quad_quality
from source_polygon_validity import polygon_diagonals

def forehead_support_rows(rows):
    rows = forehead_pattern_plain_records(rows)
    return {int(i):row for i,row in rows.items()}

def forehead_support_held_nodes(proposal):
    proposal = forehead_pattern_plain_records(proposal)
    edges=forehead_edge_counts(proposal["faces"])
    return set(proposal["protected"])|{i for path in proposal["paths"].values()for i in path}|{i for edge,count in edges.items()if count==1 for i in edge}

def forehead_support_parameters(proposal):
    proposal = forehead_pattern_plain_records(proposal)
    chart=np.asarray(proposal['source_chart']);out=np.zeros((len(proposal['vertices']),3));out[:len(chart)]=chart
    for i,row in forehead_support_rows(proposal['lineage']).items():out[i]=np.asarray(row['weights'])@chart[row['donors']]
    return out

def forehead_support_topology_contract(before,after):
    """Whole-mesh vertex links, winding, Euler, boundary, ownership and guide graph."""
    before = forehead_pattern_plain_records(before)
    after = forehead_pattern_plain_records(after)
    faces=after['faces'];edges=forehead_edge_counts(faces);old=forehead_edge_counts(before['faces']);n=len(after['vertices'])
    if n<len(before['vertices']):raise ValueError('Support operator may not remove/reindex existing points')
    if len(after['topology']['face_owners'])!=len(faces):raise ValueError('Missing face owners')
    if after['topology']['faces']!=faces:raise ValueError('Topology face snapshot mismatch')
    if any(len(q)<3 or len(set(q))!=len(q) or min(q)<0 or max(q)>=n for q in faces):raise ValueError('Invalid polygon')
    if any(len(q)!=4 for j,q in enumerate(faces)if j>=len(before['faces'])or q!=before['faces'][j]):raise ValueError('Changed face is not a quad')
    if len({tuple(sorted(q))for q in faces})!=len(faces):raise ValueError('Duplicate face')
    if max(edges.values())>2:raise ValueError('Nonmanifold edge')
    if {e for e,k in edges.items()if k==1}!={e for e,k in old.items()if k==1}:raise ValueError('Changed open edges')
    if n-len(edges)+len(faces)!=len(before['vertices'])-len(old)+len(before['faces']):raise ValueError('Euler changed')
    directed={};links={};before_links={}
    for q in before['faces']:
        for k,i in enumerate(q):before_links.setdefault(i,[]).append(tuple(sorted((q[k-1],q[(k+1)%len(q)]))))
    for q in faces:
        for k,i in enumerate(q):
            a,b=q[k-1],q[(k+1)%len(q)];links.setdefault(i,[]).append((a,b));edge=(i,b);directed[edge]=directed.get(edge,0)+1
    for (a,b),count in edges.items():
        if count==2 and(directed.get((a,b),0)!=1 or directed.get((b,a),0)!=1):raise ValueError('Inconsistent winding')
    boundary={i for e,k in edges.items()if k==1 for i in e}
    for i,pairs in links.items():
        adj={}
        for a,b in pairs:adj.setdefault(a,[]).append(b);adj.setdefault(b,[]).append(a)
        seen=set();pending=[next(iter(adj))]
        while pending:
            j=pending.pop()
            if j in seen:continue
            seen.add(j);pending.extend(adj[j])
        degrees=sorted(len(z)for z in adj.values())
        valid=degrees.count(1)==2 and all(d in(1,2)for d in degrees)if i in boundary else all(d==2 for d in degrees)
        if len(seen)!=len(adj)or not valid:
            if sorted(tuple(sorted(e))for e in pairs)!=sorted(before_links.get(i,[])):raise ValueError('New invalid vertex link')
    if after['paths']!=before['paths']:raise ValueError('Guide identities/stations changed')
    for path in after['paths'].values():
        if any(tuple(sorted(e))not in edges for e in zip(path,path[1:]))or forehead_guide_corners(faces,path):raise ValueError('Guide edge/elbow defect')
    lineage={tuple(sorted(z['edge'])):z['parent']for z in after['topology']['edge_lineage']}
    if len(lineage)!=len(after['topology']['edge_lineage'])or set(lineage)!=set(edges):raise ValueError('Missing/duplicate edge lineage')
    return True

def forehead_support_admission(source,before,after,field_context,extra_admission):
    """Mandatory external predicate retains project-specific caps/locus/pose contracts.
    field_context contains original point_discrete and native edge_fields records.
    This does NOT substitute independent dense full-pose/native/UV validation.
    """
    source = forehead_pattern_plain_records(source)
    before = forehead_pattern_plain_records(before)
    after = forehead_pattern_plain_records(after)
    field_context = forehead_pattern_plain_records(field_context)
    if not callable(extra_admission):raise ValueError('Explicit additional admission predicate required')
    old=np.asarray(source['vertices']);od=np.asarray(source['open_delta']);n=len(before['vertices']);v=np.asarray(after['vertices']);d=np.asarray(after['open_delta']);held=forehead_support_held_nodes(before)
    if not np.array_equal(v,v.astype(np.float32).astype(float))or not np.array_equal(d,d.astype(np.float32).astype(float)):raise ValueError('Saved geometry/field must be float32 quantized')
    if not np.array_equal(v[list(held)],np.asarray(before['vertices'])[list(held)]):raise ValueError('Held geometry moved')
    if after['protected']!=before['protected']:raise ValueError('Protection metadata changed')
    for key in before:
        if ('caps' in key or 'reference_length' in key)and after.get(key)!=before[key]:raise ValueError('Retained cap/reference metadata changed')
    if after['topology']['face_owners'][:len(before['faces'])]!=before['topology']['face_owners']:raise ValueError('Existing face ownership changed')
    if any(i<0 or i>=len(source['faces'])for i in after['topology']['face_owners']):raise ValueError('Invalid original face owner')
    for key in ('vertices','chart'):
        if after['topology'][key][:n]!=before['topology'][key]:raise ValueError('Existing frozen topology baseline changed')
    for key in ('roles','endpoints'):
        if after['topology'].get(key)!=before['topology'].get(key):raise ValueError('Frozen role metadata changed')
    for key in ('source_chart','source_faces'):
        if after[key]!=before[key]:raise ValueError('Frozen author domain changed')
    br=forehead_support_rows(before['topology']['lineage']);ar=forehead_support_rows(after['topology']['lineage'])
    if any(ar.get(i)!=z for i,z in br.items())or any(i<n and i not in br for i in ar):raise ValueError('Existing frozen rows changed')
    current=forehead_support_rows(after['lineage']);discrete=field_context['point_discrete'];max_reconstruction=0.
    for i in range(len(v)):
        row=current.get(i,{'donors':[i],'weights':[1.]});don=np.asarray(row['donors'],int);w=np.asarray(row['weights'],float)
        if not len(w)or np.any(w< -1e-12)or abs(w.sum()-1)>1e-9 or min(don)<0 or max(don)>=len(old):raise ValueError('Invalid donor row')
        expected=np.asarray(w@old[don],np.float32).astype(float);expected_d=np.asarray(w@od[don],np.float32).astype(float)
        if not np.array_equal(expected,v[i])or not np.array_equal(expected_d,d[i]):raise ValueError('Scalar source reconstruction mismatch')
        ref=br.get(i,{'donors':[i],'weights':[1.]})if i<n else ar[i];active=[j for j,t in zip(ref['donors'],ref['weights'])if t>0]
        for key,values in discrete.items():
            if key.startswith('.select'):continue
            target=values[active[0]]
            if any(values[j]!=target for j in active)or any(values[j]!=target for j,t in zip(don,w)if t>0):raise ValueError('Discrete donor field conflict')
    frozen=forehead_frozen_field(od,len(v),ar);field=float(np.linalg.norm(d-frozen,axis=1).max()*1000)
    if field>.5:raise ValueError('Cumulative frozen field exceeds .5mm')
    quads=np.asarray([after['faces'][i]for i in after['affected_new']],int);angles,ratio=orbital_quad_quality(v,quads)
    if angles.min()<25 or angles.max()>150 or ratio.max()>7:raise ValueError('Quantized quad quality failed')
    forehead_convex_chart(forehead_chart(v),quads);forehead_no_chart_crossings(forehead_chart(v),quads.tolist())
    opened=(v.astype(np.float32)+d.astype(np.float32)).astype(float)
    if any(not polygon_diagonals(v,list(q),opened)for q in quads):raise ValueError('No paired neutral/open export diagonal')
    forehead_support_topology_contract(before,after)
    chains=validate_forehead_edge_chains(after['topology']['edge_lineage'],{tuple(z['edge']):tuple(z['values'])for z in field_context['edge_fields']})
    if not extra_admission(after):raise ValueError('Caller retained caps/locus/extra admission rejected')
    return {'quality':[float(angles.min()),float(angles.max()),float(ratio.max())],'global_field_mm':field,'nonneutral_chains':chains,'existing_frozen_rows_exact':True,'guides_and_pins_exact':True,'scalar_rows_exact':True,'scope':'Local operator admission, not full-pose/native acceptance'}

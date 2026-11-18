"""Semantic five-flow corridor/anchor caller. No cached faces, IDs or coordinates.
A freshly recomputed semantic scaffold is input, not a stored final mesh.
"""
import copy
from collections import defaultdict, deque
import numpy as np
from forehead_pattern_catalogue import forehead_pattern_edge, forehead_pattern_edges, forehead_pattern_counts, forehead_pattern_catalogue
from forehead_pattern_solver import forehead_pattern_solve
from forehead_pattern_aggregate_solver import forehead_pattern_solve_aggregate
from forehead_pattern_materialize import forehead_pattern_materialize


def forehead_corridor_select(proposal, source_vertices, source_faces, source_eligible_faces,
                             semantic_seed_pairs, ear_role, fitted_ear_endpoint,
                             *, vertical_direction, halo_layers=2, fill_holes=True,
                             max_hole_faces=64, original_protected_vertices=()):
    """Use retiled semantic bend ownership above the fitted-ear level as focus.
    Eligibility, vertical frame and fitted-ear vertex come from LIVE semantic
    binding/provenance supplied by caller. No anatomical height is hardcoded.
    Paths retain their first two original seed vertices and complete tails.
    """
    points=np.asarray(proposal['vertices'],float);source=np.asarray(source_vertices,float);N=len(source);top=proposal['topology'];paths=proposal['paths'];owners=top['face_owners'];eligible=set(source_eligible_faces);up=np.asarray(vertical_direction,float)
    if up.shape!=(3,)or not np.isfinite(up).all()or np.linalg.norm(up)<=0:raise ValueError('Supply a semantic vertical direction')
    if not isinstance(halo_layers,int)or not 0<=halo_layers<=16 or max_hole_faces<0:raise ValueError('Invalid bounded cavity options')
    up=up/np.linalg.norm(up)
    if set(semantic_seed_pairs)!=set(paths):raise ValueError('Every current guide requires a semantic seed pair')
    for role,path in paths.items():
        if path[:2]!=list(semantic_seed_pairs[role])or any(i>=N for i in path[:2]):raise ValueError('First two ORIGINAL semantic nodes changed')
        if not np.array_equal(points[path[:2]],source[path[:2]]):raise ValueError('First two ORIGINAL positions changed')
    if paths[ear_role][-1]!=fitted_ear_endpoint or fitted_ear_endpoint>=N:raise ValueError('Guide does not terminate at actual semantic fitted ear root')
    if not np.array_equal(points[fitted_ear_endpoint],source[fitted_ear_endpoint]):raise ValueError('Actual fitted-ear endpoint moved')
    incidence=defaultdict(set)
    for fi,q in enumerate(source_faces):
        for e in forehead_pattern_edges(q):incidence[e].add(fi)
    neighbors={fi:{j for e in forehead_pattern_edges(source_faces[fi])for j in incidence[e]if j in eligible and j!=fi}for fi in eligible}
    ear_level=float(points[fitted_ear_endpoint]@up)
    # The lower-jaw closed star is deliberately not mistaken for a temple focus.
    retiled=set(top.get('retiled_old_faces',()))
    selected={fi for fi in retiled&eligible if max(source[source_faces[fi]]@up)>=ear_level}
    if not selected:
        # Scaffold-free fallback: source faces at actual geometric path bends
        # above the ear level. This selects a domain, not a universal topology.
        bends=set()
        for path in paths.values():
            for a,b,c in zip(path,path[1:],path[2:]):
                u=points[b]-points[a];v=points[c]-points[b];den=np.linalg.norm(u)*np.linalg.norm(v)
                if den>1e-15 and np.dot(u,v)/den<np.cos(np.deg2rad(20.)) and points[b]@up>=ear_level:bends.add(b)
        selected={fi for fi in eligible if set(source_faces[fi])&bends}
    if not selected:raise ValueError('No semantic upper-side corridor focus; explicit recipe required')
    # Include real ear-root approach ownership, not a truncated surrogate endpoint.
    ear_tail=set(paths[ear_role][-2:]);selected|={fi for fi in eligible if set(source_faces[fi])&ear_tail}
    for _ in range(halo_layers):selected|={j for fi in list(selected)for j in neighbors[fi]}
    filled=[]
    if fill_holes:
        # Fill only small enclosed eligible components, never domains touching
        # ineligible/mesh boundaries. Traversal/order has no vertex-ID tie-break.
        remaining=eligible-selected
        while remaining:
            first=min(remaining);component={first};queue=[first];remaining.remove(first)
            for fi in queue:
                for j in neighbors[fi]&remaining:remaining.remove(j);component.add(j);queue.append(j)
            touches_exterior=any(len(incidence[e])!=2 or any(j not in eligible for j in incidence[e])for fi in component for e in forehead_pattern_edges(source_faces[fi]))
            if len(component)<=max_hole_faces and not touches_exterior:selected|=component;filled.append(sorted(component))
    current_faces=sorted(i for i,owner in enumerate(owners)if owner in selected)
    counts=forehead_pattern_counts([proposal['faces'][i]for i in current_faces]);boundary={v for e,c in counts.items()if c==1 for v in e};used={v for fi in current_faces for v in proposal['faces'][fi]};protected=set(proposal.get('protected',()))|set(original_protected_vertices);segments={};details={}
    for role,path in paths.items():
        candidates=[k for k,v in enumerate(path)if k>=1 and v<N and v in boundary]
        if not candidates:raise ValueError('Corridor has no original attachment crossing for '+role)
        start_index=candidates[0]
        inside=[k for k,v in enumerate(path)if k>start_index and v<N and v in used]
        if not inside:raise ValueError('Corridor has no distinct original terminal for '+role)
        end_index=inside[-1]
        # Pinned original stations can remain as a fixed downstream tail. They
        # must not silently be forbidden internal transits in the joint model.
        pinned=[k for k in range(start_index+1,end_index+1)if path[k]in protected]
        if pinned:end_index=pinned[0]
        if end_index<=start_index:raise ValueError('Degenerate semantic attachment segment')
        segments[role]=(path[start_index],path[end_index]);details[role]={'start_index':start_index,'end_index':end_index,'fixed_prefix':path[:start_index+1],'fixed_tail':path[end_index:],'first_two_original':path[:2]}
    return {'selected_source_faces':sorted(selected),'selected_base_faces':current_faces,'guide_segments':segments,'anchor_details':details,'filled_components':filled,'semantic_ear_endpoint':fitted_ear_endpoint,'ear_endpoint_preserved_as_full_path_end':True,'policy':'Retiled semantic bend owners at/above live fitted-ear level, bounded original-face halo, enclosed-hole fill, original crossing anchors, complete fixed tails'}


def forehead_corridor_joint_foundation(proposal, selection, current_chart, sample_birth,
                                      source_vertices, source_open_delta, source_discrete,
                                      source_edge_fields, neutral_parent_edges,
                                      *, blocked_vertices=(),quality_cost=None,
                                      solve_options=None,birth_geometry_admitter=None,
                                      transit_width_in_stations=1.5,allow_unchanged_fallback=True,
                                      allow_transition_rotations=True,allowed_protected_transits=(),
                                      aggregate_initial_flow=False):
    """Invoke explicit catalogue/MIP/materialization on a selected live corridor.
    Optional birth_geometry_admitter(candidate,catalogue,witness) is an injected
    bounded geometry stage returning a locally admitted record or rejection.
    Without it the result is GRAPH ONLY; nothing is frozen/committed externally.
    """
    proposal=copy.deepcopy(proposal)
    seed_pins={detail['first_two_original'][0]for detail in selection['anchor_details'].values()}|{selection['semantic_ear_endpoint']}
    proposal['protected']=sorted(set(proposal.get('protected',()))|seed_pins)
    proposal['topology']['protected']=sorted(set(proposal['topology'].get('protected',()))|seed_pins)
    proposal['moving']=sorted(set(proposal.get('moving',()))-seed_pins)
    proposal['support']=sorted(set(proposal.get('support',()))-seed_pins)
    protected_edges={forehead_pattern_edge(*detail['first_two_original'])for detail in selection['anchor_details'].values()}
    cat=forehead_pattern_catalogue(proposal,selection['selected_base_faces'],current_chart,sample_birth,source_discrete,neutral_parent_edges,selection['guide_segments'],blocked_vertices,protected_edges=protected_edges,quality_cost=quality_cost,allowed_protected_transits=allowed_protected_transits)
    if aggregate_initial_flow or not allow_transition_rotations:
        cat['patterns']=[pattern for pattern in cat['patterns']if 'transition_rotation'not in pattern]
    options=dict(solve_options or {})
    if transit_width_in_stations is not None:
        options['allowed_transit_by_role']=forehead_corridor_role_transits(cat['points'],proposal['paths'],selection['guide_segments'],transit_width_in_stations)
    if aggregate_initial_flow:
        if 'allowed_transit_by_role'in options:
            options['allowed_transit_vertices']=set().union(*options.pop('allowed_transit_by_role').values())
        witness=forehead_pattern_solve_aggregate(proposal,cat,**options)
    else:witness=forehead_pattern_solve(proposal,cat,**options)
    search_status={'feasible':witness['feasible'],'status':witness['status'],'model_variables':witness.get('model_variables'),'model_rows':witness.get('model_rows')}
    fallback=False
    if not witness['feasible']and allow_unchanged_fallback:
        witness=forehead_corridor_unchanged_witness(proposal,cat);fallback=True
    result={'scope':'Initial topology foundation only; all final geometry/native guards remain mandatory','selection':selection,'witness':witness,'geometry_admitted':False,'joint_search':search_status,'unchanged_scaffold_fallback':fallback}
    if not witness['feasible']:return result
    candidate,audit=forehead_pattern_materialize(proposal,witness,source_vertices,source_open_delta,source_discrete,source_edge_fields,neutral_parent_edges,protected_edges=protected_edges)
    for role,path in candidate['paths'].items():
        if path[:2]!=selection['anchor_details'][role]['first_two_original']:raise ValueError('Joint search changed seed pair')
        tail=selection['anchor_details'][role]['fixed_tail']
        if path[-len(tail):]!=tail:raise ValueError('Joint search truncated a downstream tail')
    if proposal.get('prospective_terminal_requests'):
        requested=proposal['prospective_terminal_requests'];previous=proposal['prospective_terminal_previous_endpoints']
        if any(candidate['paths'][role][-1]!=target for role,target in requested.items()):raise ValueError('Actual joint route misses requested terminal')
        candidate['topology']['roles']['boundary_guide_reassignment']={'changed':any(previous[role]!=target for role,target in requested.items()),'method':'Fresh joint conforming-pattern graph','previous_endpoints':dict(previous),'requested_endpoints':dict(requested),'actual_endpoints':{role:candidate['paths'][role][-1]for role in requested},'acceptance':'Graph only; geometry admission pending'}
    result.update(proposal=candidate,graph_audit=audit)
    if birth_geometry_admitter is not None:
        admission=birth_geometry_admitter(candidate,cat,witness);result['geometry_admission']=admission;result['geometry_admitted']=bool(admission.get('admitted'))
    return result


def forehead_corridor_role_transits(candidate_points,paths,segments,width_in_stations=1.5):
    """Bound each role's GRAPH search by distance to its live3D polyline.
    This cannot jump surface sheets: it only filters vertices of the conforming
    source-connected catalogue. Width is normalized by live median edge length.
    """
    if not np.isfinite(width_in_stations)or width_in_stations<=0:raise ValueError('Invalid role search width')
    points=np.asarray(candidate_points,float);allowed={}
    for role,(start,end)in segments.items():
        path=paths[role];a,b=path.index(start),path.index(end);reference=points[path[a:b+1]];edges=np.diff(reference,axis=0);length=np.linalg.norm(edges,axis=1)
        if np.any(length<=1e-10):raise ValueError('Collapsed role reference edge')
        offset=points[:,None,:]-reference[:-1];fraction=np.clip(np.einsum('vkj,kj->vk',offset,edges)/(length*length),0,1);distance=np.linalg.norm(offset-fraction[:,:,None]*edges,axis=2).min(axis=1);radius=float(np.median(length))*width_in_stations
        allowed[role]=set(np.flatnonzero(distance<=radius).tolist())|set(path[a:b+1])
    return allowed


def forehead_corridor_unchanged_witness(proposal,catalogue):
    """Cheap fail-closed graph fallback; does NOT claim rerouting/improvement.
    Must pass the same materializer contracts as a solver-produced witness.
    """
    patterns=[]
    for fi in catalogue['selected_base_faces']:
        candidates=[p for p in catalogue['patterns']if p['face']==fi and p['mask']==0]
        if len(candidates)!=1:raise ValueError('No unchanged cavity pattern')
        patterns.append(candidates[0])
    paths={}
    for role,(start,end)in catalogue['guide_ends'].items():
        path=proposal['paths'][role];part=path[path.index(start):path.index(end)+1]
        if set(part)&set(catalogue['forbidden_old']):raise ValueError('Unchanged fallback would retain a detected field blocker')
        paths[role]=part
    return {'feasible':True,'status':'Unchanged live-scaffold route fallback; joint search did not find an incumbent','selected_base_faces':catalogue['selected_base_faces'],'guide_ends':catalogue['guide_ends'],'forbidden_old':catalogue['forbidden_old'],'patterns':patterns,'paths':paths,'points':catalogue['points'],'lineage':catalogue['lineage'],'new_midpoints':[],'new_centers':[]}

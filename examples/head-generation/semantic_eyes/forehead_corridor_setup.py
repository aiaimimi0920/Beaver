"""Fresh-author proposal assembly for the shared constrained fitter.
No native access, cached geometry, numeric IDs, or filesystem IO.
"""
import copy
import numpy as np
from nasal_source_domain import current_source_features
from forehead_flow_bind import bind_forehead_paths
from forehead_retile_roles import prepare_forehead_star_topology
from forehead_retile_fair import fair_forehead_guides
from forehead_fit_domain import prepare_forehead_fit_domain
from forehead_flow_rule import forehead_chart, forehead_convex_chart
from forehead_spacing import forehead_frozen_field
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality
from forehead_corridor_foundation import forehead_corridor_select,forehead_corridor_joint_foundation


def forehead_corridor_immutable_value(value):
    if isinstance(value,dict):return tuple(sorted((key,forehead_corridor_immutable_value(v))for key,v in value.items()))
    if isinstance(value,(list,tuple)):return tuple(forehead_corridor_immutable_value(v)for v in value)
    return value


def forehead_corridor_decode_source_fields(captured,vertex_count,face_count):
    if captured['original_vertex_count']!=vertex_count or captured['original_face_count']!=face_count or not captured.get('frozen_before_allocation'):raise ValueError('Source field capture provenance/count mismatch')
    if captured.get('native_source_mesh_unchanged',True)is not True:raise ValueError('Capture changed source mesh')
    raw=captured['edge_fields'];edges={tuple(sorted(pair)):tuple(values)for pair,values in raw.items()}if isinstance(raw,dict)else{tuple(sorted(row['edge'])):tuple(row['values'])for row in raw}
    neutral={tuple(sorted(pair))for pair in captured['neutral_parent_edges']}
    if not neutral<=set(edges)or any(edges[e]!=(0.,0,False,True)for e in neutral):raise ValueError('Inconsistent neutral source-edge capture')
    discrete={key:[forehead_corridor_immutable_value(v)for v in values]for key,values in captured['point_discrete'].items()}
    if any(len(values)!=vertex_count for values in discrete.values()):raise ValueError('Source point-field count mismatch')
    return {'edge_fields':edges,'neutral_parent_edges':neutral,'point_discrete':discrete}


def forehead_corridor_prepare_shared_proposal(vertices,faces,regions,report,ordinals,
                                             old_open_delta,captured_source_fields,
                                             *,fitted_ear_endpoint,ear_role,
                                             vertical_direction,rings=2,
                                             joint_options=None,run_joint=True,
                                             topology_builder=None,terminal_requester=None,
                                             corridor_options=None,joint_caller_options=None,
                                             permit_pinned_ear_star_transit=False,source_features=None,
                                             return_joint_failure=False):
    """Exact order: source capture decode -> live semantic scaffold -> joint
    graph -> seed-pin-aware fairing -> domain closure ONCE -> current fields.
    Returns a proposal-shaped initializer, NOT a feasible accepted shared state.
    Caller must retain source positions/fields and author_context for fitting.
    """
    old=np.asarray(vertices,float);old_delta=np.asarray(old_open_delta,float);original_faces=[list(q)for q in faces]
    fields=forehead_corridor_decode_source_fields(captured_source_fields,len(old),len(original_faces))
    features=copy.deepcopy(source_features)if source_features is not None else current_source_features(old,original_faces,regions,ordinals);binding=bind_forehead_paths(old,original_faces,regions,report);top=(topology_builder or prepare_forehead_star_topology)(old,original_faces,regions,report,features)
    top=copy.deepcopy(top);top['vertices']=np.asarray(top['vertices']).tolist();top['chart']=np.asarray(top['chart']).tolist();top['lineage']={int(i):row for i,row in top['lineage'].items()};chart=forehead_chart(old)
    p={'vertices':copy.deepcopy(top['vertices']),'faces':copy.deepcopy(top['faces']),'paths':copy.deepcopy(top['paths']),'topology':top,'lineage':copy.deepcopy(top['lineage']),'source_chart':chart.tolist(),'open_delta':forehead_frozen_field(old_delta,len(top['vertices']),top['lineage']).tolist(),'protected':list(top['protected']),'moving':[],'affected_new':list(top['retiled_new_faces']),'affected_old':list(top['retiled_old_faces'])}
    eligible=[]
    # This uses the same CURRENT raw13 skin/positive-side convention as the
    # imported semantic scaffold. A new semantic recipe must supply its adapter.
    for fi,(q,region)in enumerate(zip(original_faces,regions)):
        if region!=1 or len(q)!=4 or min(old[q,0])< -1e-8:continue
        try:forehead_convex_chart(chart,np.asarray([q]));eligible.append(fi)
        except ValueError:pass
    if fitted_ear_endpoint is None:fitted_ear_endpoint=top['paths'][ear_role][-1]
    selection=forehead_corridor_select(p,old,original_faces,eligible,{role:path[:2]for role,path in binding['positive_paths'].items()},ear_role,fitted_ear_endpoint,vertical_direction=vertical_direction,**dict(corridor_options or {}))
    caller_options=dict(joint_caller_options or {})
    if permit_pinned_ear_star_transit:
        caller_options['allowed_protected_transits']=set(binding['ear_root_star'])-set(features['protected'])
    if terminal_requester is not None:
        if not run_joint:raise ValueError('Prospective terminal requests require actual joint graph admission')
        p,selection=terminal_requester(p,selection,top['roles']['boundary_target_requests'])
        caller_options['allow_unchanged_fallback']=False
    if run_joint:
        source=AuthorQuadSurface(chart,[original_faces[i]for i in selection['selected_source_faces']],[1]*len(selection['selected_source_faces']));source.vertices=old.copy()
        def sample(parameter):
            try:donors,weights,point=source.lineage(parameter[0],parameter[2])
            except ValueError:return None
            return np.asarray(point,np.float32).astype(float).tolist(),{'donors':list(donors),'weights':list(weights)}
        def quality(points,children):
            angles,ratio=orbital_quad_quality(points,np.asarray(children));return float(np.sum(np.maximum(25-angles,0)**2+np.maximum(angles-150,0)**2)/100+np.sum(np.log(ratio)**2)/10)
        joint=forehead_corridor_joint_foundation(p,selection,np.asarray(top['chart']),sample,old,old_delta,fields['point_discrete'],fields['edge_fields'],fields['neutral_parent_edges'],quality_cost=quality,solve_options=joint_options or {'time_limit':30.,'max_new_points':100,'max_route_edges':50},**caller_options)
        if 'proposal'not in joint:
            if return_joint_failure:return {'admitted':False,'status':'joint_graph_rejected','joint':joint}
            raise ValueError('Joint foundation returned no materializable graph')
        p=joint['proposal'];top=p['topology'];joint_summary={'search':joint['joint_search'],'fallback':joint['unchanged_scaffold_fallback'],'audit':joint.get('graph_audit')}
    else:joint_summary={'search_skipped':True}
    seed_pins={pair[0]for pair in binding['positive_paths'].values()}|{fitted_ear_endpoint};top['protected']=sorted(set(top['protected'])|seed_pins)
    frozen_rows=copy.deepcopy(top['lineage'])
    fair=fair_forehead_guides(old,original_faces,regions,top,features)
    # Fairing supplies CURRENT rows only. Complete unchanged appended rows from
    # the original birth records; never replace topology['lineage'] with fair rows.
    fair['lineage']={**copy.deepcopy(frozen_rows),**{int(i):row for i,row in fair['lineage'].items()}}
    if fair['topology']['lineage']!=frozen_rows:raise ValueError('Fairing reset frozen birth lineage')
    if 'support_domain'in fair:raise ValueError('Domain expansion must run exactly once')
    proposal=prepare_forehead_fit_domain(old,original_faces,regions,fair,rings=rings)
    current=forehead_frozen_field(old_delta,len(proposal['vertices']),frozen_rows)
    for i,row in proposal['lineage'].items():current[int(i)]=np.asarray(row['weights'])@old_delta[row['donors']]
    proposal['open_delta']=current
    for role,path in proposal['paths'].items():
        if path[:2]!=binding['positive_paths'][role][:2]or not np.array_equal(np.asarray(proposal['vertices'])[path[:1]],old[path[:1]]):raise ValueError('Fair/domain setup changed original seed pair')
    if proposal['paths'][ear_role][-1]!=fitted_ear_endpoint or not np.array_equal(proposal['vertices'][fitted_ear_endpoint],old[fitted_ear_endpoint]):raise ValueError('Fair/domain setup changed real ear endpoint')
    return {'proposal':proposal,'scope':'Fresh shared-fit INITIALIZER only; feasibility and independent native acceptance unproven','setup':{'source_fields_decoded_before_allocation':True,'semantic_scaffold_fresh':True,'joint':joint_summary,'domain_calls':1,'rings':rings,'frozen_birth_rows_unchanged':True,'first_two_source_ids_preserved':True,'original_coordinate_pins_and_actual_ear_preserved':True,'no_extra_second_station_coordinate_pins':True,'selection':selection}}

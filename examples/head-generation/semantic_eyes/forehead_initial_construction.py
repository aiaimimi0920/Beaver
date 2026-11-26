"""Fresh source-only initial five-flow construction. Pure arrays/records; no IO.
Local quad/field admission is NOT final flow, gap, pose, UV or visual acceptance.
"""
import copy
import numpy as np
from forehead_corridor_setup import forehead_corridor_prepare_shared_proposal,forehead_corridor_decode_source_fields
from forehead_corridor_seed import forehead_corridor_prepare_seed,forehead_corridor_request_terminals
from forehead_birth_geometry import forehead_birth_solve
from forehead_surface_batch import make_forehead_surface_batch
from forehead_spacing import forehead_initial_parameters
from forehead_flow_rule import forehead_chart,forehead_convex_chart
from forehead_pattern_materialize import forehead_pattern_validate_parent_chains
from forehead_pattern_catalogue import forehead_pattern_counts,forehead_pattern_edge,forehead_pattern_guide_corners
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality


def forehead_initial_chain_dependencies(proposal,source_chart,existing_count,neutral_parent_edges):
    """Source-parent labels remain fixed; fractions seed CURRENT parameter loci.
    Only unambiguous interior births on a single original parent edge qualify.
    Junctions and off-parent seed records fail closed rather than inventing loci.
    """
    chart=np.asarray(source_chart,float);neutral={tuple(sorted(edge))for edge in neutral_parent_edges};parents={};rows={int(i):row for i,row in proposal['topology']['lineage'].items()}
    for record in proposal['topology']['edge_lineage']:
        parent=record['parent']
        if parent is None or tuple(sorted(parent))in neutral:continue
        parent=tuple(sorted(parent))
        if any(i<0 or i>=existing_count for i in parent):raise ValueError('Initial parent identity must belong to original source')
        for i in record['edge']:
            if i<existing_count:continue
            if i in parents and parents[i]!=parent:raise ValueError('Nonneutral prospective birth is a multi-parent junction')
            parents[i]=parent
    dependent={}
    for i,(a,b)in parents.items():
        row=rows[i];parameter=np.asarray(row['weights'])@chart[row['donors']][:,[0,2]];edge=chart[b,[0,2]]-chart[a,[0,2]];length2=float(edge@edge)
        if length2<=1e-20:raise ValueError('Collapsed source-parent parameter edge')
        fraction=float((parameter-chart[a,[0,2]])@edge/length2)
        if not 0<fraction<1 or np.linalg.norm(parameter-chart[a,[0,2]]-fraction*edge)>1e-8:raise ValueError('Birth seed has no unique original parent fraction')
        dependent[i]=(a,b,fraction)
    return dependent


def forehead_initial_commit_births(proposal,admission,source_vertices,source_fields,edge_fields,dependent_births=None):
    """Commit admitted NEW rows only; OLD frozen positions/chart/fields stay owned.
    This is the single fresh whole-guide length-reference freezing point.
    """
    if not admission.get('admitted'):raise ValueError('No local birth admission')
    if 'uniform_station_reference_length_mm'in proposal:raise ValueError('Initial whole-guide reference lengths already frozen')
    p=copy.deepcopy(proposal);p['topology']=copy.deepcopy(p['topology']);t=p['topology'];source=np.asarray(source_vertices,float);delta=np.asarray(source_fields,float);N=len(source);n=len(p['vertices']);chart=np.asarray(p['source_chart'],float)
    for obj,key in ((p,'vertices'),(p,'open_delta'),(t,'vertices'),(t,'chart')):obj[key]=copy.deepcopy(obj[key])
    p['lineage']={int(i):copy.deepcopy(row)for i,row in p['lineage'].items()};t['lineage']={int(i):copy.deepcopy(row)for i,row in t['lineage'].items()};old_top=copy.deepcopy(t['vertices'][:N]);old_chart=copy.deepcopy(t['chart'][:N]);old_frozen_rows={i:copy.deepcopy(row)for i,row in t['lineage'].items()if i<N}
    births={int(i):row for i,row in admission['new_birth_rows'].items()};updates={int(i):row for i,row in admission['old_current_row_updates'].items()}
    if set(births)!=set(range(N,n)) or any(i<0 or i>=N for i in updates):raise ValueError('Initial row ownership mismatch')
    for i,position in admission['positions'].items():p['vertices'][int(i)]=list(position)
    p['lineage'].update(copy.deepcopy(updates));p['lineage'].update(copy.deepcopy(births))
    for i,row in births.items():
        t['lineage'][i]=copy.deepcopy(row);t['vertices'][i]=list(p['vertices'][i]);t['chart'][i]=(np.asarray(row['weights'])@chart[row['donors']]).tolist()
    for i,value in admission['current_field_updates'].items():p['open_delta'][int(i)]=list(value)
    for i,row in {**updates,**births}.items():
        donors=np.asarray(row['donors'],int);weights=np.asarray(row['weights'],float)
        if len(donors)!=len(weights)or len(set(donors))!=len(donors)or np.any(donors<0)or np.any(donors>=N)or not np.isfinite(weights).all()or np.any(weights<0)or abs(weights.sum()-1)>1e-8:raise ValueError('Invalid committed author row')
        if not np.array_equal(np.asarray(weights@source[donors],np.float32).astype(float),np.asarray(p['vertices'][i]))or np.linalg.norm(weights@delta[donors]-np.asarray(p['open_delta'][i]))>1e-10:raise ValueError('Committed author position/field row mismatch')
    if t['vertices'][:N]!=old_top or t['chart'][:N]!=old_chart or {i:row for i,row in t['lineage'].items()if i<N}!=old_frozen_rows:raise ValueError('OLD frozen lineage/geometry reset')
    points=np.asarray(p['vertices']);held=list(p['protected'])
    if not np.array_equal(points[held],np.asarray(proposal['vertices'])[held]):raise ValueError('Held source point moved')
    if any(not np.array_equal(points[path[:1]],source[path[:1]])for path in p['paths'].values()):raise ValueError('Original first guide point moved')
    angles,ratio=orbital_quad_quality(points,np.asarray([p['faces'][i]for i in p['affected_new']]))
    if angles.min()<25 or angles.max()>150 or ratio.max()>7:raise ValueError('Committed actual quad quality rejected')
    forehead_convex_chart(forehead_chart(points),np.asarray([p['faces'][i]for i in p['affected_new']]))
    if np.linalg.norm(np.asarray(p['open_delta'])[:N]-delta,axis=1).max()*1000>.5:raise ValueError('Global OLD field budget exceeded')
    counts=forehead_pattern_counts(p['faces']);nodes=[i for path in p['paths'].values()for i in path]
    if len(nodes)!=len(set(nodes)):raise ValueError('Guide collision')
    for path in p['paths'].values():
        if forehead_pattern_guide_corners(p['faces'],path) or any(forehead_pattern_edge(a,b)not in counts for a,b in zip(path,path[1:])):raise ValueError('Guide is absent or has an elbow')
    chains=forehead_pattern_validate_parent_chains(t['edge_lineage'],edge_fields)
    p['uniform_station_reference_length_mm']={role:float(np.linalg.norm(np.diff(points[path],axis=0),axis=1).sum()*1000)for role,path in p['paths'].items()}
    p['initial_old_field_baseline']=delta.copy().tolist()
    p['initial_birth_locus_provenance']={}
    def at_parameter(i):
        if i in p['lineage']:
            row=p['lineage'][i];return np.asarray(row['weights'])@chart[row['donors']][:,[0,2]]
        return chart[i,[0,2]]
    for raw_i,dependency in (dependent_births or {}).items():
        i=int(raw_i);a,b=dependency['endpoints']if isinstance(dependency,dict)else dependency[:2];fraction=float(dependency['fraction']if isinstance(dependency,dict)else dependency[2])
        parameter=at_parameter(i);ends=[at_parameter(a),at_parameter(b)]
        if np.linalg.norm(parameter-((1-fraction)*ends[0]+fraction*ends[1]))>1e-8:raise ValueError('Admitted birth locus provenance mismatch')
        p['initial_birth_locus_provenance'][i]={'endpoints':[int(a),int(b)],'fraction':fraction,'endpoint_parameters_at_birth':[value.tolist()for value in ends],'birth_parameter':parameter.tolist(),'frozen_birth_row':copy.deepcopy(t['lineage'][i]),'position_at_birth':list(t['vertices'][i]),'field_baseline_at_birth':list(p['open_delta'][i]),'persistent_geometry_constraint':False,'scope':'Exact CURRENT author-parameter chain during initial allocation only; later CURRENT lineage may evolve under frozen-field and parent-chain guards'}
    p['initial_construction_admission']={'local_init_only':True,'matched_gap_readiness':'not_checked','scope':'Local initial quad/OLD-field/row admission only; flow/gap/pose/independent1mm/native/visual gates pending','original_vertex_count':N,'new_birth_count':n-N,'source_parent_chains':chains,'fresh_guide_length_references_frozen_once':True,'old_baselines_untouched':True,'dependency_constraints_persist_after_admission':False}
    return p


def forehead_initial_construct(vertices,faces,regions,report,ordinals,source_open_delta,
                               native_source_fields,features=None,*,provenance=None,
                               halo_layers=6,joint_seconds=180.,max_pattern_births=160,
                               max_route_edges=50,birth_seconds=120.,birth_evaluations=(250,450),
                               rings=2,ear_role='green',vertical_direction=(0.,0.,1.),blocked_transits=()):
    """Semantic seed -> ordinary aggregate/axis -> fair/domain ONCE -> birth admission.
    No fixture/cached proposal/reference lengths. Explicit rejected status has no
    committed proposal. Source hashes/native provenance are supplied by caller.
    The current semantic seed is a fail-closed raw13 recipe, not universal anatomy.
    blocked_transits is caller-derived from immutable sector witnesses, never a
    baked route. This function does not silently retry or assert atlas readiness.
    """
    old=np.asarray(vertices,float).copy();old_fields=np.asarray(source_open_delta,float).copy();stage='source_and_graph'
    try:
        source_context=forehead_corridor_decode_source_fields(copy.deepcopy(native_source_fields),len(old),len(faces))
        setup=forehead_corridor_prepare_shared_proposal(old,copy.deepcopy(faces),copy.deepcopy(regions),copy.deepcopy(report),copy.deepcopy(ordinals),old_fields,copy.deepcopy(native_source_fields),fitted_ear_endpoint=None,ear_role=ear_role,vertical_direction=vertical_direction,rings=rings,joint_options={'time_limit':joint_seconds,'max_new_points':max_pattern_births,'max_route_edges':max_route_edges,'feasibility_only':False},topology_builder=forehead_corridor_prepare_seed,terminal_requester=forehead_corridor_request_terminals,corridor_options={'halo_layers':halo_layers},joint_caller_options={'aggregate_initial_flow':True,'allow_transition_rotations':False,'transit_width_in_stations':None,'allow_unchanged_fallback':False,'blocked_vertices':tuple(blocked_transits)},permit_pinned_ear_star_transit=True,source_features=features,return_joint_failure=True)
        if 'proposal'not in setup:return {'admitted':False,'status':'joint_graph_rejected','details':setup,'provenance':copy.deepcopy(provenance)}
        stage='prospective_birth_geometry';p=setup['proposal'];N=len(old);n=len(p['vertices']);chart=np.asarray(p['source_chart']);positions=np.asarray(p['vertices'],float).copy();current=np.asarray(p['open_delta'],float).copy();lineage={int(i):row for i,row in p['lineage'].items()};parameters=forehead_initial_parameters(old,positions,chart,lineage,list(range(n)));dependent=forehead_initial_chain_dependencies(p,chart,N,source_context['neutral_parent_edges']);held=set(p['protected']);fixed={i for i in held if i>=N}
        if fixed&set(dependent):raise ValueError('Held chain birth cannot be a dependent variable')
        free=set(range(N,n))-fixed-set(dependent);movable=sorted((set(p['moving'])&set(range(N)))-held);fixed_rows={i:copy.deepcopy(p['topology']['lineage'][i])for i in fixed};source_batch=make_forehead_surface_batch(chart,old,np.asarray(p['source_faces']));field_batch=make_forehead_surface_batch(chart,old_fields,np.asarray(p['source_faces']));surface=AuthorQuadSurface(chart,p['source_faces'],[1]*len(p['source_faces']));surface.vertices=old.copy()
        def sample_row(parameter):
            try:donors,weights,point=surface.lineage(*parameter)
            except ValueError:return None
            return {'donors':list(donors),'weights':list(weights)}
        for i,(a,b,fraction)in dependent.items():
            parameters[i]=(1-fraction)*parameters[a]+fraction*parameters[b];sample=source_batch.sample(parameters[i:i+1]);field=field_batch.sample(parameters[i:i+1])
            if sample is None or field is None:raise ValueError('Dependent initial locus outside unambiguous author source')
            positions[i]=sample[0];current[i]=field[0]
        admission=forehead_birth_solve(vertices=positions,faces=p['faces'],paths=p['paths'],parameters=parameters,existing_count=N,frozen_old_fields=old_fields,current_fields=current,source_sampler=source_batch.sample,field_sampler=field_batch.sample,row_sampler=sample_row,source_vertices=old,source_fields=old_fields,source_parameters=chart[:,[0,2]],actual_chart=lambda points:forehead_chart(points)[:,[0,2]],movable_old=movable,held_nodes=held,free_births=free,fixed_births=fixed,fixed_birth_rows=fixed_rows,dependent_births=dependent,active_roles=set(p['paths']),guard_face_ids=p['affected_new'],front_caps={role:max(6.,forehead_initial_front_turn(old[path[:2]]))for role,path in p['paths'].items()},source_discrete=source_context['point_discrete'],discrete_reference=source_context['point_discrete'],phase_evaluations=birth_evaluations,wall_seconds=birth_seconds)
        if not admission['admitted']:return {'admitted':False,'status':'birth_geometry_rejected','birth_geometry':admission,'setup':setup['setup'],'provenance':copy.deepcopy(provenance)}
        stage='commit_local_admission';committed=forehead_initial_commit_births(p,admission,old,old_fields,source_context['edge_fields'],dependent_births=dependent)
        return {'admitted':True,'local_init_only':True,'matched_gap_readiness':'not_checked','status':'local_initializer_admitted','scope':'FULL shared-fit proposal, but flow/gap/pose/shape1mm/native/visual acceptance pending','proposal':committed,'setup':setup['setup'],'birth_geometry':admission,'dependent_births_at_admission':dependent,'provenance':copy.deepcopy(provenance)}
    except ValueError as error:return {'admitted':False,'status':stage+'_rejected','reason':str(error),'provenance':copy.deepcopy(provenance)}


def forehead_initial_front_turn(pair):
    a,b=np.asarray(pair,float)
    return float(2*np.degrees(np.arctan2(abs(b[2]-a[2]),abs(b[0]-a[0]))))

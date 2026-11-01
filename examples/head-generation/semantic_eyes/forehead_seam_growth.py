"""Optional bounded active-set seam restoration; original admission is external."""
import copy
import time
import numpy as np
from forehead_lp_restore import forehead_restore_linearized
from forehead_seam_restore import forehead_restore_seam_stage
from forehead_seam_support import forehead_seam_integer_vector


def forehead_validate_seam_growth_selection(selection, start, context):
    """Validate evidence against ORIGINAL accepted frame, never failed coordinates."""
    out=copy.deepcopy(selection)
    vertices=forehead_seam_integer_vector(out.get('held_vertices',()),'held vertices',allow_empty=True)
    columns=forehead_seam_integer_vector(out.get('held_columns',()),'held columns',allow_empty=True)
    if not len(vertices):
        if len(columns):raise ValueError('Columns without held vertices')
        return {'held_vertices':[],'held_columns':[],'evidence':[]}
    ids=forehead_seam_integer_vector(context['variable_ids'],'variable IDs')
    frozen=np.asarray(context['frozen_field_reference_m'],float)
    anchor=np.asarray(start,float).reshape(-1,2)
    if frozen.shape!=(len(ids),3) or anchor.shape!=(len(ids),2) or not np.isfinite(frozen).all():raise ValueError('Invalid accepted field frame')
    identity=context['source_identity']
    if not isinstance(identity,str) or not identity or out.get('source_identity')!=identity:raise ValueError('Different immutable source identity')
    evidence=out.get('evidence',[])
    ev_ids=forehead_seam_integer_vector([r['vertex']for r in evidence],'evidence vertices')
    if set(ev_ids)!=set(vertices):raise ValueError('Missing or duplicate selected evidence')
    support=set(context['support_ids']);blocked=set(context['protected_ids'])|{v for p in context['guide_paths'].values()for v in p}
    expected=[]
    for v in vertices:
        if v not in ids or v not in support or v in blocked:raise ValueError('Held vertex is not eligible free support')
        index=list(ids).index(v);cols=[2*index,2*index+1];expected+=cols
        row=next(r for r in evidence if r['vertex']==v)
        if row.get('columns')!=cols:raise ValueError('Evidence column mapping changed')
        if not np.array_equal(np.asarray(row['accepted_parameters_mm']),anchor[index]):raise ValueError('Hold is not ORIGINAL accepted coordinates')
        if not np.array_equal(np.asarray(row['frozen_field_reference_m']),frozen[index]):raise ValueError('Frozen field baseline changed')
        slack=row.get('accepted_field_slack_mm')
        if slack is None or not np.isfinite(slack) or slack<0:raise ValueError('Accepted field is not feasible')
    if sorted(columns)!=sorted(expected):raise ValueError('Held columns mismatch')
    out['held_vertices']=list(map(int,vertices));out['held_columns']=expected
    return out


def forehead_restore_seam_growth(start,radii,constraints,jacobian,admit,
        orientation_rows,domain,selection,margin_callback,orientation_floor,
        max_iterations=24,wall_seconds=120.,failed_selection=None,
        *,active_set_growth=False,growth_context=None,max_holds=4):
    """Legacy two-leg default, or <=3 legs/ONE deadline with evidence-only growth.

    growth_context contains ORIGINAL variable_ids, frozen_field_reference_m,
    support_ids, protected_ids, guide_paths and immutable source_identity.
    failed_selection must analytically prove a donor seam and accepted field
    feasibility, as the existing failed-selector does. This layer verifies
    identity/frame/eligibility, not donor mathematics. Caller still enforces
    exact successful held WORLD coordinates after scalar reconstruction.
    """
    if not active_set_growth:
        return forehead_restore_seam_stage(start,radii,constraints,jacobian,admit,
            orientation_rows,domain,selection,margin_callback,orientation_floor,
            max_iterations,wall_seconds,failed_selection)
    if not np.isfinite(wall_seconds) or not 0<wall_seconds<=120:raise ValueError('Invalid shared deadline')
    if type(max_iterations)is not int or not 1<=max_iterations<=30:raise ValueError('Invalid iteration budget')
    if type(max_holds)is not int or not 1<=max_holds<=4:raise ValueError('Invalid hold capacity')
    initial=np.asarray(start,float).copy();radius=np.asarray(radii,float).copy()
    if initial.ndim!=1 or initial.shape!=radius.shape or not np.isfinite(np.r_[initial,radius]).all() or np.any(radius<=0):raise ValueError('Invalid original bounds')
    current=forehead_validate_seam_growth_selection(selection,initial,growth_context)
    if len(current['held_vertices'])>max_holds:raise ValueError('Initial hold capacity exceeded')
    original=copy.deepcopy(current);retained=None;began=time.monotonic();legs=[];tried=set();history=[];solved=None;metadata={};successful=None;major=soc=0
    def margin(x,c):
        nonlocal retained
        request=margin_callback(x.copy(),c.copy())
        evidence=None
        if request is not None:
            target,evidence=request;target=np.asarray(target,float)
            if target.shape!=c.shape or not np.isfinite(target).all() or np.any(target<0):raise ValueError('Invalid positive targets')
            retained=target.copy()if retained is None else np.maximum(retained,target)
        return None if retained is None else (retained.copy(),{'current':copy.deepcopy(evidence),'retained_across_same_problem_legs':True})
    for number in range(3):
        remaining=wall_seconds-(time.monotonic()-began)
        if remaining<=0:break
        key=tuple(sorted(current['held_vertices']))
        if key in tried:raise ValueError('Repeated hold set')
        tried.add(key)
        # Fair reservation across remaining potential legs; last consumes remainder.
        budget=remaining/(3-number) if failed_selection is not None or key else remaining
        solved,metadata=forehead_restore_linearized(initial.copy(),radius.copy(),constraints,jacobian,admit,orientation_rows,domain,
            margin_callback=None if margin_callback is None else margin,max_iterations=max_iterations,wall_seconds=budget,
            soc_source_domain=domain,soc_orientation_floor=orientation_floor,temporary_held_columns=current['held_columns'])
        count=sum(len(t.get('SOC',()))for s in metadata.get('steps',())for t in s.get('trials',()))
        major+=int(solved.nit);soc+=count
        legs.append({'selection':copy.deepcopy(current),'held_columns':list(current['held_columns']),'wall_budget_seconds':budget,'status':metadata.get('status'),'admitted_checkpoint':bool(solved.success),'major_iterations':int(solved.nit),'soc_corrections':count})
        if solved.success:successful=copy.deepcopy(current);break
        if number==2 or time.monotonic()-began>=wall_seconds:break
        candidate=None
        if failed_selection is not None:
            try:
                found=forehead_validate_seam_growth_selection(failed_selection(np.asarray(solved.x,float).copy()),initial,growth_context)
                history.append(copy.deepcopy(found))
                added=[v for v in found['held_vertices']if v not in key]
                if added:
                    vertices=sorted(set(key)|set(added))
                    if len(vertices)<=max_holds:
                        rows={r['vertex']:r for r in current['evidence']}
                        # Duplicate evidence must agree on immutable accepted frame;
                        # validation above checks both against the same context.
                        rows.update({r['vertex']:r for r in found['evidence']})
                        candidate={'held_vertices':vertices,'held_columns':[c for v in vertices for c in rows[v]['columns']],
                            'evidence':[rows[v]for v in vertices],'source_identity':growth_context['source_identity']}
                    else:history.append({'growth_rejected':'maximum temporary holds exceeded'})
            except ValueError as error:history.append({'selection_unavailable':str(error)})
        if candidate is not None and tuple(candidate['held_vertices'])not in tried:current=candidate
        elif ()not in tried:current={'held_vertices':[],'held_columns':[],'evidence':[]}
        else:break
    if solved is None:raise ValueError('Deadline expired before first leg')
    metadata=copy.deepcopy(metadata);metadata.update(seam_selection=original,seam_restore_legs=legs,failed_seam_selection_history=history,
        successful_selection=successful,successful_held_columns=[]if successful is None else successful['held_columns'],
        total_major_iterations=major,total_soc_corrections=soc,total_work_iterations=major+soc,total_elapsed_seconds=time.monotonic()-began,
        active_set_growth=True,maximum_legs=3,maximum_holds=max_holds,original_accepted_start_and_bounds_retained=True,
        permanent_protection_unchanged=True,positive_working_targets_retained_across_legs=True)
    return solved,metadata

"""RECONSTRUCTED bounded held/free or free/failed-guided-held orchestration."""
import copy
import time
import numpy as np
from forehead_lp_restore import forehead_restore_linearized
from forehead_seam_support import forehead_intersect_held_bounds


def forehead_restore_seam_stage(start,radii,constraints,jacobian,admit,
        orientation_rows,domain,selection,margin_callback,orientation_floor,
        max_iterations=24,wall_seconds=120.,failed_selection=None):
    """At most TWO legs, ONE deadline, the SAME original problem closures.

    selection is the initial selector or already-revalidated hint result.
    failed_selection(failed_parameters_copy) is optional evidence-only selector;
    it must close over ORIGINAL accepted params/field/source/variable IDs.
    Neither leg is reanchored on the rejected iterate. Final scalar admission
    and exact successful-held world-coordinate check remain caller-owned.
    """
    if not np.isfinite(wall_seconds) or not 0<wall_seconds<=120:raise ValueError('Invalid total restoration deadline')
    if type(max_iterations)is not int or not 1<=max_iterations<=30:raise ValueError('Invalid bounded iteration count')
    initial=np.asarray(start,float).copy();original_radii=np.asarray(radii,float).copy()
    initial_selection=copy.deepcopy(selection);current_selection=copy.deepcopy(selection)
    first_columns=list(current_selection.get('held_columns',()))
    forehead_intersect_held_bounds(initial,original_radii,first_columns)
    # First leg reserves half for a real optional alternative; no extra120s.
    reserve=bool(first_columns or failed_selection is not None)
    retained_targets=None
    def retained_margin(values,residuals):
        nonlocal retained_targets
        request=margin_callback(values,residuals)
        evidence=None
        if request is not None:
            wanted,evidence=request;wanted=np.asarray(wanted,float)
            if wanted.shape!=np.asarray(residuals).shape or not np.isfinite(wanted).all() or np.any(wanted<0):raise ValueError('Invalid cross-leg positive working targets')
            retained_targets=wanted.copy()if retained_targets is None else np.maximum(retained_targets,wanted)
        return None if retained_targets is None else (retained_targets.copy(),{'current':copy.deepcopy(evidence),'retained_across_same_problem_legs':True})
    margin=retained_margin if margin_callback is not None else None
    began=time.monotonic();legs=[];solved=None;metadata={};total_major=total_soc=0
    successful_selection=None;failed_evidence=None
    for leg_index in range(2):
        remaining=wall_seconds-(time.monotonic()-began)
        if remaining<=0:break
        columns=list(current_selection.get('held_columns',()))
        forehead_intersect_held_bounds(initial,original_radii,columns)
        budget=min(remaining,wall_seconds/2)if leg_index==0 and reserve else remaining
        solved,metadata=forehead_restore_linearized(initial.copy(),original_radii.copy(),
            constraints,jacobian,admit,orientation_rows,domain,
            margin_callback=margin,max_iterations=max_iterations,
            wall_seconds=budget,soc_source_domain=domain,
            soc_orientation_floor=orientation_floor,temporary_held_columns=columns)
        corrections=sum(len(trial.get('SOC',()))for step in metadata.get('steps',())for trial in step.get('trials',()))
        total_major+=int(solved.nit);total_soc+=corrections
        legs.append({'held_columns':columns,'selection':copy.deepcopy(current_selection),
            'status':metadata.get('status',str(solved.message)),'major_iterations':int(solved.nit),
            'soc_corrections':corrections,'wall_budget_seconds':budget,
            'elapsed_seconds':metadata.get('elapsed_seconds'),
            'admitted_checkpoint':bool(solved.success)})
        if solved.success:
            successful_selection=copy.deepcopy(current_selection)if columns else {'held_columns':[],'held_vertices':[],'evidence':[]}
            break
        if leg_index==1 or wall_seconds-(time.monotonic()-began)<=0:break
        if first_columns:
            current_selection={'held_columns':[],'held_vertices':[],'evidence':[],
                               'selection_basis':'Release optional initial hold; original freedom'}
        elif failed_selection is not None:
            try:
                candidate=failed_selection(np.asarray(solved.x,float).copy())
            except ValueError as error:
                failed_evidence={'selection_unavailable':str(error)};break
            if not isinstance(candidate,dict):raise ValueError('Failed selector must return evidence dictionary')
            failed_evidence=copy.deepcopy(candidate)
            if not candidate.get('held_columns'):break
            current_selection=copy.deepcopy(candidate)
        else:break
    if solved is None:raise ValueError('Restoration deadline expired before first leg')
    metadata=copy.deepcopy(metadata)
    metadata.update(seam_selection=initial_selection,failed_seam_selection=failed_evidence,
        seam_restore_legs=legs,successful_selection=successful_selection,
        successful_held_columns=[]if successful_selection is None else list(successful_selection.get('held_columns',())),
        total_major_iterations=total_major,total_soc_corrections=total_soc,
        total_work_iterations=total_major+total_soc,total_elapsed_seconds=time.monotonic()-began,
        unheld_retry_attempted=bool(first_columns and len(legs)>1),
        failed_guided_held_retry_attempted=bool(not first_columns and len(legs)>1),
        original_accepted_start_and_bounds_retained=True,permanent_protection_unchanged=True,positive_working_targets_retained_across_legs=True)
    return solved,metadata

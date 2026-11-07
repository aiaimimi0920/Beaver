"""RECONSTRUCTED: failed frame diagnoses; original accepted frame supplies hold."""
import numpy as np
from forehead_seam_support import forehead_seam_inputs,forehead_seam_policy,forehead_seam_field_sample,forehead_seam_probe_pairs


def forehead_select_failed_seam_supports(accepted_mm,failed_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids,sample_field_differential,source_donors,*,original_radii_mm,budget_mm=.499,probe_mm=.0001,minimum_jump=.05,relative_jump=.25,max_holds=1,max_candidates=16):
    accepted,ids,frozen,support,forbidden=forehead_seam_inputs(accepted_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids)
    failed=np.asarray(failed_mm,float);radii=np.asarray(original_radii_mm,float)
    if failed.shape!=accepted.shape or radii.shape!=accepted.shape or not np.isfinite(failed).all() or not np.isfinite(radii).all() or np.any(radii<=0) or np.any(abs(failed-accepted)>radii):raise ValueError('Failed coordinates exceed ORIGINAL bounds/alignment')
    forehead_seam_policy(budget_mm,probe_mm,minimum_jump,relative_jump,max_holds,max_candidates)
    a=forehead_seam_field_sample(accepted,frozen,sample_field_differential,budget_mm);f=forehead_seam_field_sample(failed,frozen,sample_field_differential,budget_mm)
    if a is None or f is None:raise ValueError('Source invalid in seam diagnostic')
    candidates=[i for i,v in enumerate(ids)if v in support and v not in forbidden and a[0][i]>=0 and f[0][i]<0]
    candidates=sorted(candidates,key=lambda i:(f[0][i],int(ids[i])))[:max_candidates];evidence=[];probed=0
    for i in candidates:
        if len(evidence)>=max_holds:break
        probed+=1;w=forehead_seam_probe_pairs(failed,i,frozen,sample_field_differential,source_donors,budget_mm,probe_mm,minimum_jump,relative_jump,True)
        if w:evidence.append({'vertex':int(ids[i]),'columns':[2*i,2*i+1],'accepted_parameters_mm':accepted[i].tolist(),'failed_parameters_mm_diagnostic_only':failed[i].tolist(),'frozen_field_reference_m':frozen[i].tolist(),'accepted_field_slack_mm':float(a[0][i]),'failed_field_slack_mm':float(f[0][i]),'frames':{'accepted':{'role':'immutable stage anchor/hold coordinates','parameters_mm':accepted[i].tolist(),'field_slack_mm':float(a[0][i])},'failed':{'role':'diagnostic identification only','parameters_mm':failed[i].tolist(),'field_slack_mm':float(f[0][i])}},'probe_witness_count':len(w),'probes':[max(w,key=lambda row:row['gradient_jump'])]})
    return {'held_vertices':[e['vertex']for e in evidence],'held_columns':[c for e in evidence for c in e['columns']],'evidence':evidence,'eligible_probed_count':probed,'selection_basis':'Failed field/seam identification; ORIGINAL accepted coordinates held','failed_state_never_baseline':True,'original_bounds_checked':True,'probe_mm':probe_mm,'policy':{'budget_mm':budget_mm,'minimum_jump':minimum_jump,'relative_jump':relative_jump,'max_holds':max_holds,'max_candidates':max_candidates}}

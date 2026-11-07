"""RECONSTRUCTED from reviewed design; not recovered original file bytes.
Pure optional accepted-source seam support selection and bound intersection.
"""
import numpy as np


def forehead_seam_integer_vector(values, name, allow_empty=False):
    a=np.asarray(values)
    if allow_empty and a.ndim==1 and not a.size:return np.empty(0,dtype=int)
    if a.ndim!=1 or a.dtype.kind not in 'iu' or len(np.unique(a))!=len(a):
        raise ValueError(name+' must contain unique integers')
    return a.astype(np.int64,copy=False)


def forehead_intersect_held_bounds(anchor,radii,held_columns=()):
    x=np.asarray(anchor,float);r=np.asarray(radii,float)
    if x.ndim!=1 or not len(x) or r.shape!=x.shape or not np.isfinite(np.r_[x,r]).all() or np.any(r<=0):raise ValueError('Invalid ORIGINAL accepted bounds')
    h=forehead_seam_integer_vector(held_columns,'held columns',True)
    if np.any(h<0) or np.any(h>=len(x)):raise ValueError('Held column outside original parameters')
    lo=x-r;hi=x+r;lo[h]=x[h];hi[h]=x[h]
    return lo,hi


def forehead_seam_inputs(coordinates_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids):
    xy=np.asarray(coordinates_mm,float);ids=forehead_seam_integer_vector(variable_ids,'variable IDs');frozen=np.asarray(frozen_field,float)
    if xy.shape!=(len(ids),2) or frozen.shape!=(len(ids),3) or np.any(ids<0) or not np.isfinite(xy).all() or not np.isfinite(frozen).all():raise ValueError('Invalid parameter/field alignment')
    support=set(forehead_seam_integer_vector(list(support_ids),'support IDs',True))
    forbidden=set(forehead_seam_integer_vector(list(protected_ids),'protected IDs',True))
    forbidden|={i for path in guide_paths.values()for i in forehead_seam_integer_vector(path,'guide path',True)}
    return xy,ids,frozen,support,forbidden


def forehead_seam_policy(budget,probe,jump,relative,holds,candidates):
    if not np.isfinite([budget,probe,jump,relative]).all() or min(budget,probe,jump,relative)<=0 or probe>1:raise ValueError('Invalid seam policy')
    if type(holds)is not int or type(candidates)is not int or not 0<=holds<=4 or not holds<=candidates<=64:raise ValueError('Invalid bounded selection count')


def forehead_seam_field_sample(params,frozen,callback,budget):
    result=callback(params/1000)
    if result is None:return None
    value,D,boundary=np.asarray(result[0],float),np.asarray(result[1],float),np.asarray(result[2])
    if value.shape!=frozen.shape or D.shape!=(len(params),3,2) or boundary.shape!=(len(params),) or boundary.dtype.kind!='b' or not np.isfinite(value).all() or not np.isfinite(D).all():raise ValueError('Malformed field differential')
    diff=value-frozen;norm=np.linalg.norm(diff,axis=1);unit=np.divide(diff,norm[:,None],out=np.zeros_like(diff),where=norm[:,None]>0)
    return budget-1000*norm,-np.einsum('ni,nij->nj',unit,D),boundary


def forehead_seam_probe_pairs(params,index,frozen,callback,donors,budget,probe,jump,relative,require_shared_edge=False):
    samples=[];witnesses=[]
    for axis in range(2):
        for sign in (-1,1):
            q=params.copy();q[index,axis]+=sign*probe;sample=forehead_seam_field_sample(q,frozen,callback,budget)
            if sample is None or sample[2][index]:continue
            vertices=forehead_seam_integer_vector(donors(q[index]/1000),'source donors').tolist()
            if len(vertices)!=4 or min(vertices)<0:raise ValueError('Source donor must be original cyclic quad')
            samples.append({'axis':axis,'offset_mm':sign*probe,'donors':vertices,'field_gradient_mm_per_mm':sample[1][index].tolist(),'boundary_flag':False})
    for k,one in enumerate(samples):
        for two in samples[k+1:]:
            a,b=one['donors'],two['donors']
            if set(a)==set(b):continue
            common={tuple(sorted((a[j],a[(j+1)%4])))for j in range(4)} & {tuple(sorted((b[j],b[(j+1)%4])))for j in range(4)}
            if require_shared_edge and len(common)!=1:continue
            g1=np.asarray(one['field_gradient_mm_per_mm']);g2=np.asarray(two['field_gradient_mm_per_mm']);difference=float(np.linalg.norm(g1-g2));scale=max(np.linalg.norm(g1),np.linalg.norm(g2),1e-12)
            if difference>=jump and difference>=relative*scale:
                row={'first':one,'second':two,'gradient_jump':difference}
                if len(common)==1:row['shared_original_edge']=list(next(iter(common)))
                witnesses.append(row)
    return witnesses


def forehead_select_seam_supports(coordinates_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids,sample_field_differential,source_donors,*,budget_mm=.499,active_slack_mm=.002,probe_mm=.0001,minimum_jump=.05,relative_jump=.25,max_holds=1,max_candidates=16):
    """Original ACCEPTED start only. Temporary optimization hint, never a pin.
    Full-batch sample coordinates in metres preserve sampler tie behavior.
    Compare two off-edge probes: scalar/global batch baseline tie owners differ.
    """
    xy,ids,frozen,support,forbidden=forehead_seam_inputs(coordinates_mm,variable_ids,frozen_field,support_ids,guide_paths,protected_ids)
    forehead_seam_policy(budget_mm,probe_mm,minimum_jump,relative_jump,max_holds,max_candidates)
    if not np.isfinite(active_slack_mm) or active_slack_mm<=0:raise ValueError('Invalid active slack heuristic')
    sample=forehead_seam_field_sample(xy,frozen,sample_field_differential,budget_mm)
    if sample is None:raise ValueError('Original accepted source invalid')
    slack,gradient,boundary=sample
    candidates=[i for i,v in enumerate(ids)if v in support and v not in forbidden and 0<=slack[i]<=active_slack_mm]
    candidates=sorted(candidates,key=lambda i:(not bool(boundary[i]),slack[i],int(ids[i])))[:max_candidates]
    evidence=[];probed=0
    for i in candidates:
        if len(evidence)>=max_holds:break
        probed+=1;w=forehead_seam_probe_pairs(xy,i,frozen,sample_field_differential,source_donors,budget_mm,probe_mm,minimum_jump,relative_jump)
        if w:evidence.append({'vertex':int(ids[i]),'parameter_index':i,'columns':[2*i,2*i+1],'accepted_parameters_mm':xy[i].tolist(),'accepted_field_slack_mm':float(slack[i]),'frozen_field_reference_m':frozen[i].tolist(),'field_gradient_mm_per_mm':gradient[i].tolist(),'boundary_flag':bool(boundary[i]),'probes':w})
    return {'held_vertices':[e['vertex']for e in evidence],'held_columns':[c for e in evidence for c in e['columns']],'evidence':evidence,'eligible_probed_count':probed,'scope':'Transient restriction only; all original primal/final gates remain','policy':{'budget_mm':budget_mm,'active_slack_mm':active_slack_mm,'probe_mm':probe_mm,'minimum_jump':minimum_jump,'relative_jump':relative_jump,'max_holds':max_holds,'max_candidates':max_candidates}}

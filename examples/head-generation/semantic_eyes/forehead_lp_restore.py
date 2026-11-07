# Reconstructed optional hold extension of restored v25 helper; not lost-file bytes.
"""Bounded same-closure LP restoration; final scalar/model admission stays external."""
import time
import copy
import numpy as np
from scipy import sparse
from scipy.optimize import linprog, OptimizeResult
from forehead_seam_support import forehead_intersect_held_bounds
from forehead_extrema_lp import forehead_extrema_rows, forehead_extrema_rhs, forehead_extrema_prediction


def forehead_restore_linearized(start, radii, constraints, jacobian, admit,
        orientation_rows, domain, margin_callback=None, warm=None,
        max_iterations=12, wall_seconds=120., trust_mm=1., progress=None,
        soc_source_domain=None, soc_orientation_floor=None, reduction_fraction=1.,
        temporary_held_columns=(), linear_model=None):
    """Restore one frozen problem; NEVER rebuild caps, fields or references.

    All parameters/bounds are chart millimetres. constraints returns the exact
    fixed-scaled residual vector (nonnegative feasible); jacobian has same rows.
    domain(x) returns bool or {'valid': bool, 'reason': ...}; it must retain the
    COMPLETE atlas/source validity, not merely necessary anchor conditions.
    admit(x) is the original quantized checkpoint selector, not solver status.
    Caller MUST still perform final scalar reconstruction/all original gates.

    Optional margin_callback(x, raw_residuals) requests an ABSOLUTE nonnegative
    vector of stronger residual targets plus evidence as (targets, evidence).
    E.g. caller maps measured float32 corner drift + safety into original scaled
    angle rows, and tiny positive targets into variable near-active raw rows.
    It may return None when no request is justified. Targets only INCREASE;
    all original residual rows and final tolerances remain unchanged. Callback
    owns unit conversion, variable/fixed row ownership and quantization policy.
    Copies isolate callback inputs; closures/source baseline remain caller-owned.
    """
    # Optional all-child rows affect only LP directions, never aggregate merit,
    # working targets, derivative consumers, or saved admission. Unsupported
    # owners keep the original Jacobian, including its finite-difference fallback.
    if linear_model is not None and not callable(linear_model):
        raise ValueError('Invalid all-child linear model callback')
    anchor=np.asarray(start,float).copy();radius=np.asarray(radii,float).copy()
    if (anchor.ndim!=1 or radius.shape!=anchor.shape or not len(anchor) or
            not np.isfinite(np.r_[anchor,radius]).all() or np.any(radius<=0)):
        raise ValueError('Invalid immutable parameter bounds')
    if type(max_iterations)is not int or not 1<=max_iterations<=30 or not 0<wall_seconds<=300 or not 0<trust_mm<=1:
        raise ValueError('Invalid bounded LP budget')
    x=anchor.copy()if warm is None else np.asarray(warm,float).copy()
    lower,upper=forehead_intersect_held_bounds(anchor,radius,temporary_held_columns)
    if x.shape!=anchor.shape or not np.isfinite(x).all() or np.any((x<lower)|(x>upper)):
        raise ValueError('Warm exceeds ORIGINAL bounds')
    if not np.isfinite(reduction_fraction) or not 0 < reduction_fraction <= 1:raise ValueError('Invalid fractional reduction')
    if (soc_source_domain is None) != (soc_orientation_floor is None):raise ValueError('SOC requires source domain and orientation floor')
    began=time.monotonic();records=[];requests=[];target=None;selected=False;status='iteration_limit'
    a,b=orientation_rows
    def inspect(values):
        c=np.asarray(constraints(values),float)
        if c.ndim!=1 or not len(c)or not np.isfinite(c).all():raise ValueError('Nonfinite/invalid original residual vector')
        if not 0<=a<=b<=len(c):raise ValueError('Invalid orientation row range')
        d=domain(values.copy());valid=bool(d.get('valid',False))if isinstance(d,dict)else bool(d)
        return c,valid and bool(np.all(c[a:b]>0)),d
    c,valid,d=inspect(x)
    if not valid:raise ValueError('Initial source/domain/orientation invalid')
    target=np.zeros(len(c));row_count=len(c)
    floor=None if soc_orientation_floor is None else np.asarray(soc_orientation_floor,float).copy()
    if floor is not None and (floor.shape!=(b-a,) or not np.isfinite(floor).all() or np.any(floor<=0) or np.any(c[a:b]<floor)):raise ValueError('SOC floor must be positive and initially feasible')
    def checkpoint(values,c):
        nonlocal selected,target
        if len(c)!=row_count:raise ValueError('Constraint row ownership changed')
        if bool(admit(values.copy())):selected=True;return True
        if margin_callback is not None:
            request=margin_callback(values.copy(),c.copy())
            if request is not None:
                wanted,evidence=request;wanted=np.asarray(wanted,float)
                if wanted.shape!=target.shape or not np.isfinite(wanted).all()or np.any(wanted<0):
                    raise ValueError('Invalid strict interior margin request')
                raised=np.maximum(target,wanted)
                if np.any(raised>target):requests.append({'iteration':len(records),'rows':np.flatnonzero(raised>target).tolist(),'evidence':evidence});target=raised.copy()
        return False
    def source_valid(values):
        result=soc_source_domain(values.copy())
        return bool(result.get('valid',False)) if isinstance(result,dict) else bool(result)
    def second_order_trial(base, trial, linear_goal, record):
        # Only internal samples; never call admission until complete exact checks.
        y=trial.copy(); goals=np.minimum(linear_goal,target); logs=[];record['SOC']=logs
        for correction in range(2):
            remaining=wall_seconds-(time.monotonic()-began)
            if remaining<=0 or not source_valid(y):return None
            cy=np.asarray(constraints(y),float);Jy=sparse.csr_matrix(jacobian(y))
            if cy.shape!=(row_count,) or not np.isfinite(cy).all() or Jy.shape!=(row_count,len(y)) or not np.isfinite(Jy.data).all():return None
            desired=goals.copy(); desired[a:b]=np.maximum(target[a:b],floor*(1. if correction==0 else 1.1))
            values, rows, owners = forehead_extrema_rows(
                cy, Jy, None if linear_model is None else linear_model(y.copy()))
            nn=np.maximum(np.sqrt(np.asarray(rows.multiply(rows).sum(axis=1)).ravel()),1e-8);n=len(y)
            rhs = forehead_extrema_rhs(cy, values, owners, cy-desired)
            matrix=sparse.vstack((sparse.hstack((-sparse.diags(1/nn)@rows,sparse.csr_matrix((len(values),1)))),sparse.hstack((sparse.eye(n),-np.ones((n,1)))),sparse.hstack((-sparse.eye(n),-np.ones((n,1))))),format='csr')
            lo=np.maximum.reduce((lower-y,base-trust_mm-y,np.full(n,-trust_mm)));hi=np.minimum.reduce((upper-y,base+trust_mm-y,np.full(n,trust_mm)))
            if np.any(lo>hi):return None
            remaining=wall_seconds-(time.monotonic()-began)
            if remaining<=0:return None
            sol=linprog(np.r_[np.zeros(n),1.],A_ub=matrix,b_ub=np.r_[rhs/nn,np.zeros(2*n)],bounds=list(zip(lo,hi))+[(0,trust_mm)],method='highs',options={'time_limit':min(10.,remaining),'primal_feasibility_tolerance':1e-9})
            item={'correction':correction+1,'LP_success':bool(sol.success),'LP_message':str(sol.message)};logs.append(item)
            if not sol.success:return None
            y=y+sol.x[:-1];item['max_correction_mm']=float(abs(sol.x[:-1]).max())
            if np.any((y<lower)|(y>upper)):item['rejected']='Original bounds';return None
            try:cc,valid,dd=inspect(y)
            except ValueError as error:item['rejected']=str(error);return None
            merit=float(np.linalg.norm(np.minimum(cc-target,0)));item.update(merit=merit,domain=dd,orientation_floor_pass=bool(np.all(cc[a:b]>=floor)))
            if valid and np.all(cc[a:b]>=floor) and merit<record['major_merit']*(1-1e-4*.5):return y,cc
        return None
    checkpoint(x,c)
    while not selected and len(records)<max_iterations:
        remaining=wall_seconds-(time.monotonic()-began)
        if remaining<=0:status='wall_limit';break
        c,valid,d=inspect(x);residual=c-target
        J=sparse.csr_matrix(jacobian(x))
        if J.shape!=(row_count,len(x))or not np.isfinite(J.data).all():raise ValueError('Invalid exact constraint Jacobian')
        values, rows, owners = forehead_extrema_rows(
            c, J, None if linear_model is None else linear_model(x.copy()))
        norm=np.maximum(np.sqrt(np.asarray(rows.multiply(rows).sum(axis=1)).ravel()),1e-8);n=len(x)
        parent_rhs = np.where(residual<0,reduction_fraction*residual,residual)
        rhs = forehead_extrema_rhs(c, values, owners, parent_rhs)
        A=sparse.vstack((sparse.hstack((-sparse.diags(1/norm)@rows,sparse.csr_matrix((len(values),1)))),sparse.hstack((sparse.eye(n),-np.ones((n,1)))),sparse.hstack((-sparse.eye(n),-np.ones((n,1))))),format='csr')
        lo=np.maximum(lower-x,-trust_mm);hi=np.minimum(upper-x,trust_mm)
        remaining=wall_seconds-(time.monotonic()-began)
        if remaining<=0:status='wall_limit';break
        lp=linprog(np.r_[np.zeros(n),1.],A_ub=A,b_ub=np.r_[rhs/norm,np.zeros(2*n)],bounds=list(zip(lo,hi))+[(0,trust_mm)],method='highs',options={'time_limit':min(10.,remaining),'primal_feasibility_tolerance':1e-9})
        row={'iteration':len(records),'LP_success':bool(lp.success),'LP_message':str(lp.message),'minimum_original':float(c.min()),'merit_before':float(np.linalg.norm(np.minimum(residual,0))),'trials':[]};records.append(row)
        if not lp.success:
            parent_norm=np.maximum(np.sqrt(np.asarray(J.multiply(J).sum(axis=1)).ravel()),1e-8)
            row['worst_scaled_rows']=[{'index':int(i),'residual':float(residual[i]),'gradient_norm':float(parent_norm[i])}for i in np.argsort(residual)[:24]]
            status='linearized_infeasible_or_limit';break
        direction=lp.x[:-1];row['max_step_mm']=float(abs(direction).max());advanced=False
        for alpha in (1.,.5,.25,.125,.0625,.03125,.015625,.0078125,.00390625,.001,.0001,.00001,.000001):
            if time.monotonic()-began>=wall_seconds:break
            trial=x+alpha*direction
            if np.any((trial<lower)|(trial>upper)):
                row['trials'].append({'alpha':alpha,'rejected':'Original bounds'});continue
            try:cc,valid,d=inspect(trial)
            except ValueError as error:
                row['trials'].append({'alpha':alpha,'rejected':str(error)});continue
            merit=float(np.linalg.norm(np.minimum(cc-target,0)));entry={'alpha':alpha,'merit':merit,'domain':d};row['trials'].append(entry)
            if floor is not None and (not valid or np.any(cc[a:b]<floor)) and alpha==.5 and source_valid(trial):
                entry['major_merit']=row['merit_before']
                corrected=second_order_trial(x,trial,forehead_extrema_prediction(
                    row_count, values, rows, owners, alpha*direction),entry)
                if corrected is not None:
                    x,cc=corrected;advanced=True;checkpoint(x,cc);entry['direction_accepted_not_model']=True;row['admitted_checkpoint']=selected;break
            if not valid or (floor is not None and np.any(cc[a:b]<floor)):
                entry['rejected']=('Full source/domain/orientation' if floor is None else 'Full source/domain/orientation or positive working floor');continue
            if merit<row['merit_before']*(1-1e-4*alpha)or np.min(cc-target)>=0:
                x=trial;advanced=True;checkpoint(x,cc);entry['direction_accepted_not_model']=True;row['admitted_checkpoint']=selected;break
            entry['rejected']='No exact nonlinear merit descent'
        if progress is not None:progress({'steps':copy.deepcopy(records),'margin_requests':copy.deepcopy(requests),'parameters_mm':x.copy(),'admitted_checkpoint':selected})
        if selected:status='admitted_checkpoint';break
        if not advanced:status='no_safe_descent';break
    if selected:status='admitted_checkpoint'
    metadata={'status':status,'admitted_checkpoint':selected,'steps':records,'margin_requests':requests,'original_bounds_retained':True,'constraints_and_final_tolerances_unchanged':True,'elapsed_seconds':time.monotonic()-began,'full_model_success':False}
    return OptimizeResult(x=x.copy(),success=selected,message=status,nit=len(records)),metadata


def forehead_quantization_margin_targets(residuals, jacobian, raw_per_scaled,
        lower_rows, upper_rows, smooth_angles, quantized_angles,
        lower_hard=25., upper_hard=150., lower_fit=25.003, upper_fit=149.997,
        safety_degrees=.001, positive_scaled=1e-7):
    """Pure optional caller adapter for measured corner rounding, no case IDs.

    Angles/rows have matching flattened corner ownership. raw_per_scaled is
    the ORIGINAL positive fixed solver scaling; never infer it from trial rows.
    Only variable near-active rows receive a tiny positive target. Quantized
    angle failures request measured drift + safety relative to HARD bounds,
    accounting for the already-present fit buffer. Does not modify any gate.
    """
    c=np.asarray(residuals,float);scale=np.asarray(raw_per_scaled,float);J=sparse.csr_matrix(jacobian)
    lower_ids = np.asarray(lower_rows)
    upper_ids = np.asarray(upper_rows)
    if (lower_ids.dtype.kind not in 'iu' or upper_ids.dtype.kind not in 'iu'
            or not np.isfinite(J.data).all()
            or not np.isfinite([lower_hard, upper_hard, lower_fit, upper_fit,
                                safety_degrees, positive_scaled]).all()):
        raise ValueError('Invalid finite margin policy or integer corner row ownership')
    lo=lower_ids.astype(int).ravel();hi=upper_ids.astype(int).ravel()
    if len(np.unique(np.r_[lo, hi])) != len(lo) + len(hi):
        raise ValueError('Corner rows must be unique and disjoint')
    smooth=np.asarray(smooth_angles,float).ravel();saved=np.asarray(quantized_angles,float).ravel()
    if (c.ndim!=1 or scale.shape!=c.shape or J.shape[0]!=len(c) or
            lo.shape!=hi.shape or lo.shape!=smooth.shape or smooth.shape!=saved.shape or
            not np.isfinite(np.r_[c,scale,smooth,saved]).all()or np.any(scale<=0)or
            np.any(np.r_[lo,hi]<0)or np.any(np.r_[lo,hi]>=len(c))or
            not lower_hard<=lower_fit<upper_fit<=upper_hard or safety_degrees<=0 or positive_scaled<=0):
        raise ValueError('Invalid measured quantization row ownership/scaling')
    variable=np.asarray(J.multiply(J).sum(axis=1)).ravel()>1e-20
    target=np.where(variable&(c<1e-5),positive_scaled,0.);evidence=[]
    for k in np.flatnonzero((saved<lower_hard)|(saved>upper_hard)):
        upper=bool(saved[k]>upper_hard);row=int(hi[k]if upper else lo[k])
        if not variable[row]:raise ValueError('Quantized failing angle has no variable gradient')
        drift=float(saved[k]-smooth[k]if upper else smooth[k]-saved[k])
        present=upper_hard-upper_fit if upper else lower_fit-lower_hard
        extra=max(0.,drift+safety_degrees-present)
        target[row]=max(target[row],extra/scale[row])
        evidence.append({'row':row,'smooth_degrees':float(smooth[k]),'quantized_degrees':float(saved[k]),'measured_outward_drift_degrees':drift,'additional_raw_target_degrees':extra})
    return target,{'quantized_angle_repairs':evidence,'variable_near_active_positive_scaled':positive_scaled,'fixed_rows_unchanged':True}

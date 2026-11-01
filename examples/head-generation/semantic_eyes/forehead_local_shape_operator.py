"""Bounded pure support-only local shape conditioning; no filesystem or native work."""
import copy
import time
import numpy as np
from scipy.optimize import minimize,Bounds
from scipy.optimize._numdiff import approx_derivative
from forehead_surface_batch import ForeheadSurfaceBatch
from forehead_spacing import forehead_frozen_field
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_quad_quality
from forehead_operator_selected_shape_samples import ForeheadSelectedForwardSamples
from forehead_operator_reverse_shape_samples import ForeheadReverseShapeSamples
from forehead_support_contract import forehead_support_parameters,forehead_support_rows,forehead_support_admission,forehead_support_held_nodes

class ForeheadLocalShapeBudget(Exception):
    pass

def forehead_condition_local_shape(source,proposal,witness,field_context,extra_admission,*,halo_rings=1,box_mm=3.,wall_seconds=120.,max_iterations=90,maximum_variables=64,maximum_local_faces=100,local_limit_mm=.975):
    """Return diagnostic result; proposal key exists ONLY after scalar admission.

    witness = {target_face_ids: [...], source_face_ids: [...]} derives from actual
    error localization, NOT nearest-distance masking. Every supplied source face
    is sampled at33x33 in both poses; reverse search uses ALL affected targetquads.
    Forward fitting covers all quads incident to eligible nearby supports, frozen
    against ALL affected original author quads. This is NOT a dense global guard.
    Existing lineage baselines, all guides/protected points, paths and caps held.
    No topology births. A rejected result never returns an applicable proposal.
    extra_admission is mandatory project-specific retained-cap/locus validator.
    """
    if not callable(extra_admission):raise ValueError('Mandatory retained-cap admission callback missing')
    if not 0<local_limit_mm<=.975 or not 0<wall_seconds<=120 or not 0<box_mm<=3 or halo_rings not in (0,1,2):raise ValueError('Operator limits exceeded')
    if proposal.get('support_operator_pending_birth_admission'):raise ValueError('Birth geometry must be admitted/frozen first')
    began=time.monotonic();p=proposal;old=np.asarray(source['vertices']);od=np.asarray(source['open_delta']);v=np.asarray(p['vertices']);delta=np.asarray(p['open_delta']);sc=np.asarray(p['source_chart']);params=forehead_support_parameters(p);faces=p['faces']
    targets=sorted(set(map(int,witness['target_face_ids'])));sources=sorted(set(map(int,witness['source_face_ids'])))
    if not targets or not sources or not set(targets)<=set(p['affected_new'])or not set(sources)<=set(p['affected_old']):raise ValueError('Witness outside declared affected coverage')
    core={i for j in targets for i in faces[j]};near=set(core)
    for unused in range(halo_rings):near|={i for q in faces if set(q)&near for i in q}
    held=forehead_support_held_nodes(p);nongons={i for q in faces if len(q)!=4 for i in q};ids=sorted((near&set(p['support']))-held-nongons)
    changed=[j for j,q in enumerate(faces)if set(q)&set(ids)]
    if not ids or len(ids)>maximum_variables or len(changed)>maximum_local_faces:raise ValueError('Local support domain empty or exceeds limits')
    if not set(changed)<=set(p['affected_new']):raise ValueError('Moving support would leave declared affected patch')
    qq=np.asarray([faces[j]for j in changed]);allq=np.asarray([faces[j]for j in p['affected_new']]);x0=params[ids][:,[0,2]].ravel()*1000
    batch=ForeheadSurfaceBatch(sc,old,np.asarray(p['source_faces']));db=ForeheadSurfaceBatch(sc,od,np.asarray(p['source_faces']));frozen=forehead_frozen_field(od,len(v),forehead_support_rows(p['topology']['lineage']));surface=AuthorQuadSurface(sc,p['source_faces'],[1]*len(p['source_faces']));surface.vertices=old.copy()
    forward=ForeheadSelectedForwardSamples(old,[source['faces'][i]for i in p['affected_old']],od,v,delta,qq,limit_mm=local_limit_mm)
    reverse=ForeheadReverseShapeSamples(old,[source['faces'][i]for i in sources],od,v,delta,allq,limit_mm=local_limit_mm,dense_source_face_indices=list(range(len(sources))))
    Q=len(qq);B=13*Q+len(ids);radius=np.median(np.linalg.norm(old[:,:2],axis=1));cache={};history=[];last=x0.copy();accepted=None;last_rejection=None
    def evaluate(x):
        if time.monotonic()-began>wall_seconds-2:raise ForeheadLocalShapeBudget()
        key=x.tobytes()
        if key in cache:return cache[key]
        xy=x.reshape(-1,2)/1000;w=batch.sample(xy);d=db.sample(xy)
        if w is None or d is None:return None
        vv=v.copy();dd=delta.copy();vv[ids]=w;dd[ids]=d
        if len(cache)>200:cache.clear()
        cache[key]=(vv,dd);return vv,dd
    initial=evaluate(x0)
    if initial is None or not np.array_equal(initial[0][ids].astype(np.float32),v[ids].astype(np.float32))or not np.array_equal(initial[1][ids].astype(np.float32),delta[ids].astype(np.float32)):raise ValueError('Initial author-sheet reconstruction mismatch')
    def base(x):
        z=evaluate(x)
        if z is None:return np.full(B,-1000.)
        vv,dd=z;a,ratio=orbital_quad_quality(vv,qq);xy=np.column_stack((radius*np.arctan2(vv[:,0],-vv[:,1]),vv[:,2]))[qq];e=np.roll(xy,-1,axis=1)-xy;cr=(e[:,:,0]*np.roll(e[:,:,1],-1,axis=1)-e[:,:,1]*np.roll(e[:,:,0],-1,axis=1))*1e6;field=np.sum((dd[ids]-frozen[ids])**2,axis=1)*1e6
        return np.r_[(a-25.0005).ravel()/10,(149.9995-a).ravel()/10,6.9995-ratio,(cr-1e-8).ravel(),.4999**2-field]
    def constraints(x):
        z=evaluate(x)
        if z is None:return np.full(B+forward.count+reverse.count,-1000.)
        return np.r_[base(x),forward.constraints(*z),reverse.constraints(*z)]
    def jacobian(x):
        z=evaluate(x)
        if z is None:return approx_derivative(constraints,x,method='2-point',abs_step=1e-5)
        xy=x.reshape(-1,2)/1000;_,J,flags=batch.sample_differential(xy);_,D,df=db.sample_differential(xy)
        if J is None or D is None:return approx_derivative(constraints,x,method='2-point',abs_step=1e-5)
        out=np.vstack((approx_derivative(base,x,method='2-point',abs_step=1e-5),forward.jacobian(*z,ids,J/1000,D/1000).toarray(),reverse.jacobian(*z,ids,J/1000,D/1000).toarray()))
        for k in np.flatnonzero(flags|df):
            for axis in(0,1):
                c=2*k+axis;h=1e-4;plus=x.copy();minus=x.copy();plus[c]+=h;minus[c]-=h;out[:,c]=(constraints(plus)-constraints(minus))/(2*h)
        return out
    def scalar_candidate(x):
        pp=copy.deepcopy(p);lin=forehead_support_rows(pp['lineage'])
        for i,xy in zip(ids,x.reshape(-1,2)/1000):
            donors,w,point=surface.lineage(*xy);pp['vertices'][i]=np.asarray(point,np.float32).astype(float).tolist();lin[i]={'donors':list(donors),'weights':list(w)};pp['open_delta'][i]=np.asarray(np.asarray(w)@od[list(donors)],np.float32).astype(float).tolist()
        pp['lineage']={str(i):z for i,z in lin.items()};return pp
    def callback(x):
        nonlocal last,accepted,last_rejection
        last=x.copy();c=constraints(x);history.append(float(c.min()))
        if c.min() < -1e-8:return
        try:
            candidate=scalar_candidate(x);vv=np.asarray(candidate['vertices']);dd=np.asarray(candidate['open_delta']);fc=forward.full_constraints(vv,dd);rc=reverse.full_constraints(vv,dd)
            if fc.min()<0 or rc.min()<0:return
            audit=forehead_support_admission(source,p,candidate,field_context,extra_admission)
            if candidate['topology']!=p['topology']:raise ValueError('Support-only operation changed topology snapshot')
            audit.update(forward_max_mm=float(np.sqrt(local_limit_mm**2-fc.min())),reverse_max_mm=float(np.sqrt(local_limit_mm**2-rc.min())),reverse_target_count=len(allq))
            accepted={'proposal':candidate,'audit':audit};raise StopIteration()
        except ValueError as error:last_rejection=str(error)
    message=''
    try:
        result=minimize(lambda x:1e-4*np.mean((x-x0)**2),x0,jac=lambda x:2e-4*(x-x0)/len(x),method='SLSQP',bounds=Bounds(x0-box_mm,x0+box_mm),constraints={'type':'ineq','fun':constraints,'jac':jacobian},callback=callback,options={'maxiter':max_iterations,'ftol':1e-9,'disp':False});last=result.x;message=str(result.message)
    except StopIteration:message='Stopped on scalar quantized admitted local witness'
    except ForeheadLocalShapeBudget:message='Bounded wall budget exhausted'
    out={'local_admitted':accepted is not None,'fit_complete':False,'solver_message':message,'seconds':time.monotonic()-began,'variable_ids':ids,'history':history,'parameters_mm':last.tolist(),'last_admission_rejection':last_rejection,'source_face_ids':sources,'target_face_ids':targets,'reverse_target_count':len(allq),'scope':'Local sparse/dense-witness fitting only; independent full-pose guards required'}
    if accepted:out.update(accepted)
    return out

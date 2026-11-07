"""Pure bounded prospective-birth geometry fitting. No baseline is mutated/frozen.
A returned admitted=True is a LOCAL geometry/field/station admission only, never
an independent surface-error, topology, UV, physical-turn or native acceptance.
"""
import time
import numpy as np
from scipy.optimize import least_squares
from scipy.sparse import lil_matrix


class ForeheadBirthBudgetExceeded(Exception):
    pass


def forehead_birth_quad_quality(points, quads):
    q=np.asarray(points)[quads];e=np.roll(q,-1,axis=1)-q;length=np.linalg.norm(e,axis=2)
    safe=np.maximum(length,1e-14);cosine=np.sum(-np.roll(e,1,axis=1)*e,axis=2)/(np.roll(safe,1,axis=1)*safe)
    return np.degrees(np.arccos(np.clip(cosine,-1,1))),safe.max(axis=1)/safe.min(axis=1),length


def forehead_birth_chart_cross(parameters, quads):
    q=np.asarray(parameters)[quads];e=np.roll(q,-1,axis=1)-q
    return e[:,:,0]*np.roll(e[:,:,1],-1,axis=1)-e[:,:,1]*np.roll(e[:,:,0],-1,axis=1)


def forehead_birth_solve(*, vertices, faces, paths, parameters, existing_count,
                         frozen_old_fields, current_fields, source_sampler,
                         field_sampler, row_sampler, source_vertices, source_fields,
                         actual_chart, movable_old, held_nodes, free_births,
                         fixed_births, canonical_parameters=None, active_roles=(),
                         guard_face_ids=None, rail_targets=None, station_caps=None, front_caps=None,
                         target_tolerance_mm=None, source_discrete=None,
                         discrete_reference=None, fixed_birth_rows=None, dependent_births=None, source_parameters=None,
                         chain_locus_validator=None, old_radius_mm=10.,new_radius_mm=20.,
                         phase_evaluations=(250,450),wall_seconds=120.,
                         minimum_angle=25.,maximum_angle=150.,maximum_ratio=7.,
                         field_budget_mm=.5,quality_margin=.02,field_margin_mm=.001,
                         parameter_cross_margin_mm2=.02,regularization=.002,
                         target_weight=.08,station_weight=10.,station_fit_margin=.0001):
    """Samplers take a batch of2D author parameters; row_sampler takes one2D point.
    row_sampler returns original-author {donors,weights}. actual_chart(points)
    returns an Nx2 chart of ACTUAL quantized geometry. Positions are in metres.
    free_births, fixed_births, and dependent_births partition ALL prospective new vertices.
    With dependent_births omitted, the existing free/fixed API is unchanged.
    frozen_old_fields covers every existing vertex, including earlier births.
    fixed_birth_rows are immutable input records for new nonneutral-chain seeds;
    their coordinates/rows remain exact. Dependent births are a separate partition.
    Dependent prospective births sample the affine AUTHOR-PARAMETER segment
    between two current endpoints in an acyclic dependency graph at each evaluation; their rows are not
    frozen until successful quantized admission. Parent identity/edge fields
    remain caller-owned topology contracts, not original geographic loci.
    Static seeds may be supplied differently
    prospective seeds BEFORE calling this function (never an existing reset).
    Optional chain_locus_validator(parameters, rows)->bool is mandatory externally
    whenever moving old support points also needs a source-chain geometric guard.
    """
    start_time=time.monotonic();v=np.asarray(vertices,float).copy();base=np.asarray(parameters,float).copy();n=len(v);N=int(existing_count);oldf=np.asarray(frozen_old_fields,float).copy();fields=np.asarray(current_fields,float).copy();source=np.asarray(source_vertices,float);source_delta=np.asarray(source_fields,float)
    if not all(np.isfinite(a).all()for a in (v,base,oldf,fields,source,source_delta)):raise ValueError('Non-finite geometry/field inputs')
    if base.shape!=(n,2)or not 0<N<=n or oldf.shape!=(N,3)or fields.shape!=(n,3):raise ValueError('Inconsistent mesh/parameter/field shapes')
    if not np.isfinite(wall_seconds)or wall_seconds<=0 or len(phase_evaluations)!=2 or any(not np.isfinite(x)or int(x)<1 for x in phase_evaluations):raise ValueError('Unbounded or invalid solver budget')
    if (minimum_angle,maximum_angle,maximum_ratio,field_budget_mm)!=(25.,150.,7.,.5):raise ValueError('This admission API preserves25..150/aspect7/old-field.5 guards')
    if any(not np.isfinite(x)or x<0 for x in (quality_margin,field_margin_mm,parameter_cross_margin_mm2,regularization,target_weight,station_fit_margin))or not np.isfinite(station_weight)or station_weight<=0:raise ValueError('Invalid solver weights/margins')
    old=sorted(set(movable_old));new=sorted(set(free_births));fixed=set(fixed_births);held=set(held_nodes)
    if any(i<0 or i>=N for i in old)or any(i<N or i>=n for i in new)or fixed&set(new):raise ValueError('Invalid old/free/fixed birth partition')
    dependent,dependent_order=forehead_birth_dependency_order(dependent_births or {},n,N)
    if fixed&set(dependent) or set(new)&set(dependent) or fixed|set(new)|set(dependent)!=set(range(N,n)):raise ValueError('Free/static-fixed/dependent births must partition prospective births')
    source_parameters_array=None if source_parameters is None else np.asarray(source_parameters,float)
    if dependent and (source_parameters_array is None or source_parameters_array.shape!=(len(source),2)or not np.isfinite(source_parameters_array).all()):raise ValueError('Dependent births require original-author source_parameters for row/locus verification')
    other_guides={i for role,path in paths.items()if role not in active_roles for i in path};held|=other_guides
    if (set(old)|set(new)|set(dependent))&held:raise ValueError('Movable set overlaps held points/other guides')
    ids=old+new;index={i:k for k,i in enumerate(ids)};D=2*len(ids);old_count=len(old)
    sampled_ids=ids+dependent_order
    ancestors={i:{i}for i in ids}
    for i in dependent_order:
        a,b=dependent[i]['endpoints'];ancestors[i]=ancestors.get(a,set())|ancestors.get(b,set())
    def parameter_dependencies(i):return ancestors.get(i,set())
    touching={fi for fi,q in enumerate(faces)if set(q)&(set(ids)|set(range(N,n)))}
    if any(len(faces[fi])!=4 for fi in touching):raise ValueError('Changed/birth patch contains a nonquad')
    guarded=set(range(len(faces)))if guard_face_ids is None else set(guard_face_ids)
    if not touching<=guarded:raise ValueError('Guard faces omit changed/birth incidence')
    guarded=sorted(fi for fi in guarded if len(faces[fi])==4);quads=np.asarray([faces[fi]for fi in guarded],int);Q=len(quads)
    if not Q:raise ValueError('No quality guards')
    front_caps=dict(front_caps or {})
    if any(role not in paths or not np.isfinite(cap)or not 0<=cap<=180 for role,cap in front_caps.items()):raise ValueError('Invalid authored front caps')
    station_caps=dict(station_caps or {});targets={int(i):np.asarray(p,float)for i,p in (rail_targets or {}).items()}
    if any(i not in index for i in targets):raise ValueError('Rail target must be movable')
    if any(role not in paths or not np.isfinite(cap)or cap<0 for role,cap in station_caps.items()):raise ValueError('Invalid station cap')
    fixed_rows={int(i):row for i,row in (fixed_birth_rows or {}).items()}
    if set(fixed_rows)!=fixed:raise ValueError('Supply rows for exactly the static fixed births; dependent rows are sampled only after admission')
    seed=base.copy()
    for i,p in (canonical_parameters or {}).items():
        if int(i)not in index:raise ValueError('Canonical seed may move only declared variables')
        seed[int(i)]=p
    # Caller guesses cannot alter a fixed nonneutral-chain birth.
    area=np.sum(base[quads][:,:,0]*np.roll(base[quads][:,:,1],-1,axis=1)-base[quads][:,:,1]*np.roll(base[quads][:,:,0],-1,axis=1),axis=1);winding=np.sign(area)
    if np.any(winding==0):raise ValueError('Author winding undefined; caller must supply nondegenerate prospective face coordinates')
    anchor=base[ids].ravel()*1000;seed_x=seed[ids].ravel()*1000;radii=np.repeat([old_radius_mm if i<N else new_radius_mm for i in ids],2)
    if not np.isfinite(radii).all()or np.any(radii<=0)or np.any(abs(seed_x-anchor)>radii):raise ValueError('Canonical seed outside declared bounded search box')
    E=sum(len(paths[role])-1 for role in station_caps);T=len(targets);hard_targets=T if target_tolerance_mm is not None else 0
    if target_tolerance_mm is not None and (not np.isfinite(target_tolerance_mm)or target_tolerance_mm<0):raise ValueError('Negative target tolerance')
    hard_size=13*Q+old_count+2*E+hard_targets+len(front_caps);residual_size=hard_size+D+2*T
    best={'score':float('inf'),'x':None};attempts=[];calls=0;cache={}
    def evaluate(x):
        p=base.copy();p[ids]=x.reshape(-1,2)/1000
        for i in dependent_order:
            row=dependent[i];a,b=row['endpoints'];fraction=float(row['fraction']);p[i]=(1-fraction)*p[a]+fraction*p[b]
        pos=source_sampler(p[sampled_ids]);delta=field_sampler(p[old])if old else np.empty((0,3))
        if pos is None or delta is None:return None
        pos=np.asarray(pos,float);delta=np.asarray(delta,float)
        if pos.shape!=(len(sampled_ids),3)or delta.shape!=(old_count,3)or not np.isfinite(pos).all()or not np.isfinite(delta).all():return None
        points=v.copy();points[sampled_ids]=pos;a,ratio,length=forehead_birth_quad_quality(points,quads);cross=forehead_birth_chart_cross(p,quads)*winding[:,None]*1e6;drift=np.linalg.norm(delta-oldf[old],axis=1)*1000
        station=[]
        for role,cap in station_caps.items():
            ll=np.linalg.norm(np.diff(points[paths[role]],axis=0),axis=1)*1000;mean=ll.mean();fit_cap=max(0.,cap-station_fit_margin);station.extend((ll-(1-fit_cap)*mean,(1+fit_cap)*mean-ll))
        target_dist=np.array([np.linalg.norm(p[i]-target)*1000 for i,target in targets.items()])
        hard=np.r_[(a-minimum_angle-quality_margin).ravel(),(maximum_angle-quality_margin-a).ravel(),(maximum_ratio-quality_margin-ratio)*5,((cross-parameter_cross_margin_mm2)*20).ravel(),(field_budget_mm-field_margin_mm-drift)*500,np.concatenate(station)*station_weight if station else [],(target_tolerance_mm-target_dist)*10 if hard_targets else [],[(cap-quality_margin-forehead_birth_front_turn(points[paths[role][:2]]))*10 for role,cap in front_caps.items()]]
        return points,p,hard,{'angle_min':float(a.min()),'angle_max':float(a.max()),'ratio_max':float(ratio.max()),'author_cross_min_mm2':float(cross.min()),'moved_old_field_max_mm':float(drift.max())if len(drift)else 0.,'collapsed':bool(np.any(length<=1e-10))}
    def residual(x,phase):
        nonlocal calls
        calls+=1
        if time.monotonic()-start_time>wall_seconds:raise ForeheadBirthBudgetExceeded()
        z=evaluate(x)
        if z is None:return np.full(residual_size,1e4)
        hard=z[2];violation=np.minimum(hard,0);score=float(violation@violation)
        # Always rank with ALL full-strength requested hard violations, in BOTH phases.
        if score<best['score']:
            best.update(score=score,x=x.copy());cache['best_metrics']=z[3]
        weighted=violation.copy()
        if phase==0:weighted[:9*Q]*=.01
        target=np.concatenate([(z[1][i]-p)*1000 for i,p in targets.items()])if targets else np.empty(0)
        return np.r_[weighted,regularization*(x-anchor),target_weight*target]
    sparsity=lil_matrix((residual_size,D),dtype=int)
    for k,q in enumerate(quads):
        columns=sorted({2*index[j]+a for i in q for j in parameter_dependencies(i)for a in (0,1)})
        rows=list(range(4*k,4*k+4))+list(range(4*Q+4*k,4*Q+4*k+4))+[8*Q+k]+list(range(9*Q+4*k,9*Q+4*k+4))
        for row in rows:sparsity[row,columns]=1
    for k in range(old_count):sparsity[13*Q+k,2*k:2*k+2]=1
    offset=13*Q+old_count
    for role in station_caps:
        count=2*(len(paths[role])-1);columns=sorted({2*index[j]+axis for i in paths[role]for j in parameter_dependencies(i)for axis in (0,1)});sparsity[offset:offset+count,columns]=1;offset+=count
    for k,i in enumerate(targets):
        if hard_targets:sparsity[offset+k,2*index[i]:2*index[i]+2]=1
        sparsity[hard_size+D+2*k:hard_size+D+2*k+2,2*index[i]:2*index[i]+2]=1
    for k,role in enumerate(front_caps):
        columns=sorted({2*index[j]+axis for i in paths[role][:2]for j in parameter_dependencies(i)for axis in (0,1)})
        sparsity[offset+hard_targets+k,columns]=1
    for k in range(D):sparsity[hard_size+k,k]=1
    x=seed_x.copy()
    try:
        if D:
            for phase,limit in enumerate(phase_evaluations):
                fit=least_squares(lambda value:residual(value,phase),x,bounds=(anchor-radii,anchor+radii),jac_sparsity=sparsity.tocsr(),max_nfev=int(limit),ftol=1e-10,xtol=1e-10,gtol=1e-9);attempts.append({'phase':phase,'message':str(fit.message),'evaluations':int(fit.nfev)});x=fit.x
        else:residual(x,1)
    except ForeheadBirthBudgetExceeded:attempts.append({'message':'bounded wall budget exhausted'})
    report={'admitted':False,'scope':'Local prospective birth geometry only; topology/UV/physical-turn/arc/independent1mm/native gates remain external','attempts':attempts,'calls':calls,'seconds':time.monotonic()-start_time,'best_all_requested_violation_score':best['score'],'guard_face_ids':guarded,'best_unquantized_metrics':cache.get('best_metrics')}
    if best['x']is None:report['reason']='No source-covered candidate';return report
    chosen=evaluate(best['x']);points,p,hard,metrics=chosen;records={};quant=v.copy();current=fields.copy();errors=[]
    # Reconstruct every changed old point and every birth from authoritative donor arrays.
    for i in sorted(set(ids)|set(range(N,n))):
        row=fixed_rows[i]if i in fixed else row_sampler(p[i].copy())
        if row is None:errors.append('missing source row');break
        donors=np.asarray(row['donors'],int);weights=np.asarray(row['weights'],float)
        if len(donors)!=len(weights)or len(set(donors))!=len(donors)or np.any(donors<0)or np.any(donors>=len(source))or not np.isfinite(weights).all()or np.any(weights<0)or abs(weights.sum()-1)>1e-8:errors.append('invalid source row');break
        qpos=np.asarray(weights@source[donors],np.float32).astype(float);qfield=weights@source_delta[donors]
        if i in fixed and not np.array_equal(qpos,v[i]):errors.append('fixed birth row disagrees with fixed point');break
        if i in dependent and np.linalg.norm(weights@source_parameters_array[donors]-p[i])>1e-8:errors.append('dependent source-row parameter mismatch');break
        if i in dependent:
            sampled_field=field_sampler(p[i:i+1])
            if sampled_field is None or np.linalg.norm(np.asarray(sampled_field)[0]-qfield)>1e-9:errors.append('dependent source-row field mismatch');break
        if np.linalg.norm((weights@source[donors])-points[i])>1e-7:errors.append('source sampler/row disagreement');break
        for key,values in (source_discrete or {}).items():
            if key == '.select_vert':
                continue
            memberships = [values[d] for d, weight in zip(donors, weights) if weight > 0]
            if not memberships or any(value != memberships[0] for value in memberships[1:]):
                errors.append('ambiguous discrete membership')
            if (discrete_reference is not None and key in discrete_reference and i < N
                    and memberships and memberships[0] != discrete_reference[key][i]):
                errors.append('changed old discrete membership')
        records[i]={'donors':donors.tolist(),'weights':weights.tolist()};quant[i]=qpos;current[i]=qfield
    if not errors:
        a,ratio,length=forehead_birth_quad_quality(quant,quads);actual=np.asarray(actual_chart(quant),float)
        if actual.shape!=(n,2)or not np.isfinite(actual).all():errors.append('invalid actual chart')
        else:
            actual_cross=forehead_birth_chart_cross(actual,quads)
            if np.any(actual_cross.min(axis=1)*actual_cross.max(axis=1)<=1e-18):errors.append('quantized actual-chart fold/degeneracy')
        if a.min()<minimum_angle or a.max()>maximum_angle or ratio.max()>maximum_ratio or np.any(length<=1e-10):errors.append('quantized quad quality')
        if np.any(forehead_birth_chart_cross(p,quads)*winding[:,None]<=0):errors.append('author-chart orientation')
        drift=np.linalg.norm(current[:N]-oldf,axis=1)*1000
        if drift.max()>field_budget_mm:errors.append('global OLD frozen field')
        if not np.array_equal(quant[list(held)],v[list(held)]):errors.append('held point moved')
        stations={}
        for role,cap in station_caps.items():
            ll=np.linalg.norm(np.diff(quant[paths[role]],axis=0),axis=1)*1000;deviation=float(np.max(abs(ll/ll.mean()-1)));stations[role]=deviation
            if deviation>cap:errors.append('quantized station cap:'+role)
        front_values={role:forehead_birth_front_turn(quant[paths[role][:2]])for role in front_caps}
        for role,cap in front_caps.items():
            if front_values[role]>cap:errors.append('authored front cap:'+role)
        if hard_targets and any(np.linalg.norm(p[i]-target)*1000>target_tolerance_mm for i,target in targets.items()):errors.append('hard rail target')
        for i,row in dependent.items():
            end_a,end_b=row['endpoints'];fraction=float(row['fraction'])
            if np.linalg.norm(p[i]-((1-fraction)*p[end_a]+fraction*p[end_b]))>1e-12:errors.append('dependent current-chain locus')
        if chain_locus_validator is not None and not chain_locus_validator(p.copy(),records):errors.append('source-chain locus')
        report['quantized_audit']={'minimum_angle':float(a.min()),'maximum_angle':float(a.max()),'maximum_ratio':float(ratio.max()),'global_old_field_max_mm':float(drift.max()),'station_deviation':stations,'held_exact':not bool(set(sampled_ids)&held),'dependent_birth_count':len(dependent),'front_degrees':front_values}
    if errors:report['reason']=sorted(set(errors));return report
    report.update(admitted=True,chosen_parameters={i:p[i].tolist()for i in sorted(set(ids)|set(range(N,n)))},old_current_row_updates={i:records[i]for i in old},new_birth_rows={i:records[i]for i in range(N,n)},positions={i:quant[i].tolist()for i in records},current_field_updates={i:current[i].tolist()for i in records},exact_changed_old_ids=[i for i in old if not np.array_equal(quant[i],v[i])],old_baselines_untouched=True)
    return report


def forehead_birth_spoke_seed_trials(parameters, existing_count, free_births,
                                     spoke_pairs, fractions=(.1,.15,.2)):
    """Bounded scale trials for NEUTRAL removed-spoke midpoint/center pairs only.
    Neutrality must already be proved by the pattern catalogue. Caller provides
    rail-width/station targets separately; this does not invent an absolute width.
    """
    base=np.asarray(parameters,float);allowed=set(free_births);fractions=tuple(fractions)
    if not 1<=len(fractions)<=8 or any(not 0<f<.5 for f in fractions):raise ValueError('Provide1..8 bounded positive cell-width fractions below.5')
    if any(a<existing_count or b<existing_count or a not in allowed or b not in allowed for a,b in spoke_pairs):raise ValueError('Spoke initialization may touch free NEW births only')
    trials=[]
    for fraction in fractions:
        seed={}
        for mid,center in spoke_pairs:
            direction=base[center]-base[mid];seed[mid]=(base[mid]-2*fraction*direction).tolist();seed[center]=(base[center]+2*fraction*direction).tolist()
        trials.append({'cell_width_fraction':float(fraction),'canonical_parameters':seed})
    return trials


def forehead_birth_dependency_order(dependent_births, point_count, existing_count):
    """Normalize affine current-chain births and fail closed on cycles/old resets."""
    normalized={}
    for raw_i,raw in dependent_births.items():
        i=int(raw_i)
        if i<existing_count or i>=point_count:raise ValueError('Dependent row cannot reset an OLD point')
        a,b=raw['endpoints']if isinstance(raw,dict)else raw[:2];fraction=float(raw['fraction']if isinstance(raw,dict)else raw[2]);a,b=int(a),int(b)
        if a==b or a==i or b==i or min(a,b)<0 or max(a,b)>=point_count or not np.isfinite(fraction)or not 0<fraction<1:raise ValueError('Invalid dependent chain endpoints/fraction')
        normalized[i]={'endpoints':[a,b],'fraction':fraction}
    waiting={i:set(row['endpoints'])&set(normalized)for i,row in normalized.items()};order=[]
    while waiting:
        ready=sorted(i for i,parents in waiting.items()if not parents)
        if not ready:raise ValueError('Dependent birth cycle')
        order.extend(ready)
        for i in ready:del waiting[i]
        for parents in waiting.values():parents.difference_update(ready)
    return normalized,order


def forehead_birth_front_turn(pair):
    a,b=np.asarray(pair,float)
    return float(2*np.degrees(np.arctan2(abs(b[2]-a[2]),abs(b[0]-a[0]))))

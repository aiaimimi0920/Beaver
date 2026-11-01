"""Opt-in native-stage consumer. Partial math checkpoints NEVER allocate native data."""
import copy as forehead_consumer_copy
import hashlib as forehead_consumer_hashlib
import json as forehead_consumer_json
import math as forehead_consumer_math
from functools import partial as forehead_consumer_partial
from forehead_seam_growth import forehead_validate_seam_budget
from forehead_pattern_materialize import forehead_pattern_plain_records
from forehead_initial_ready import initialize_forehead_shared
from forehead_shared_fit import fit_forehead_shared_constraints
from forehead_flow_continuation import continue_forehead_flows
from forehead_fairing_continuation import continue_forehead_fairing
from forehead_gap_continuation import continue_forehead_gaps
from forehead_shared_acceptance import measure_forehead_shared_acceptance



class ForeheadSharedPending(ValueError):
    """Detached native staging must unwind; caller may persist this owned checkpoint."""
    def __init__(self, checkpoint):
        self.checkpoint = forehead_consumer_copy.deepcopy(checkpoint)
        super().__init__('Shared forehead pending: '+checkpoint['status'])


def forehead_consumer_canonical(value):
    value = forehead_pattern_plain_records(value)
    if isinstance(value, dict):
        pairs = [[forehead_consumer_canonical(k), forehead_consumer_canonical(v)] for k,v in value.items()]
        return ['map', sorted(pairs, key=lambda row: forehead_consumer_json.dumps(row[0], sort_keys=True))]
    if isinstance(value, (set, frozenset)):
        return ['set', sorted([forehead_consumer_canonical(v) for v in value], key=repr)]
    if isinstance(value, list):
        return ['list', [forehead_consumer_canonical(v) for v in value]]
    if isinstance(value, bytes):
        return ['bytes', value.hex()]
    if isinstance(value, float) and not forehead_consumer_math.isfinite(value):
        raise ValueError('Nonfinite source binding')
    if value is None or isinstance(value, (str, bool, int, float)):
        return value
    raise ValueError('Unsupported source binding value')



def forehead_consumer_checkpoint_bytes(checkpoint):
    return forehead_consumer_json.dumps(forehead_pattern_plain_records(checkpoint),
        separators=(',', ':'), allow_nan=False).encode('utf-8')


def forehead_consumer_binding(raw_source, author, native_capture, pose_context):
    value = forehead_consumer_canonical(dict(raw_source=raw_source, author=author,
        native_capture=native_capture, pose_context=pose_context))
    return forehead_consumer_hashlib.sha256(forehead_consumer_json.dumps(
        value, separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def forehead_consumer_options(options):
    allowed = {'mode','resume','flow_steps','fairing_stages','gap_steps','max_iterations',
               'initialization_options','seam_active_set_growth','seam_wall_seconds','seam_initial_held_seconds',
               'paired_diagonal_guidance'}
    if not isinstance(options, dict) or set(options)-allowed:
        raise ValueError('Unknown shared consumer option')
    result = forehead_consumer_copy.deepcopy(options)
    if result.get('mode') not in ('fresh','resume'):
        raise ValueError('Explicit fresh/resume mode required')
    for key, default, maximum in (('flow_steps',1,10),('fairing_stages',0,5),
                                  ('gap_steps',0,10),('max_iterations',80,200)):
        value = result.setdefault(key,default)
        if type(value) is not int or not (1 if key=='max_iterations' else 0)<=value<=maximum:
            raise ValueError('Invalid bounded consumer budget: '+key)
    if type(result.setdefault('seam_active_set_growth',False)) is not bool:
        raise ValueError('Invalid seam option')
    if type(result.setdefault('paired_diagonal_guidance',False)) is not bool:
        raise ValueError('Invalid paired diagonal guidance option')
    forehead_validate_seam_budget(result.setdefault('seam_wall_seconds',120.),
        result.setdefault('seam_initial_held_seconds',None),result['seam_active_set_growth'])
    if result['mode']=='fresh' and 'resume' in result:
        raise ValueError('Fresh initialization cannot consume a saved proposal')
    if result['mode']=='resume' and 'initialization_options' in result:
        raise ValueError('Resume must not initialize/refreeze')
    return result


def forehead_consumer_retains(before, after):
    if len(before['vertices'])!=len(after['vertices']):
        raise ValueError('Consumer fitting unexpectedly changed topology size')
    for key in ('paths','faces','source_chart','source_faces','uniform_station_reference_length_mm'):
        if forehead_consumer_canonical(before[key]) != forehead_consumer_canonical(after[key]):
            raise ValueError('Consumer stage changed immutable '+key)
    for key in ('vertices','chart','lineage'):
        left,right=before['topology'][key],after['topology'][key]
        if key=='lineage':
            for rows in (left,right):
                if any(isinstance(i,bool) or str(int(i))!=str(i) for i in rows):
                    raise ValueError('Invalid frozen lineage semantic ID')
                if len({int(i) for i in rows})!=len(rows):
                    raise ValueError('Duplicate frozen lineage semantic ID')
            left={int(i):row for i,row in left.items()}
            right={int(i):row for i,row in right.items()}
        if forehead_consumer_canonical(left) != forehead_consumer_canonical(right):
            raise ValueError('Consumer stage reset frozen topology '+key)
    return True


def prepare_forehead_shared_consumer(raw_source, author, native_capture, pose_context,
                                     options, checkpoint=None):
    """Real bounded public API sequencing; no permissive final-admission callback.

    This first consumer slice deliberately cannot return native_ready=True:
    support policy and complete independent final/native gates are not wired yet.
    It is callable from the actual detached native stage, not a saved-case runner.
    """
    opts=forehead_consumer_options(options)
    if not callable(checkpoint):
        raise ValueError('Opt-in consumer requires an owned durable checkpoint sink')
    solve=forehead_consumer_partial(fit_forehead_shared_constraints,
        seam_wall_seconds=opts['seam_wall_seconds'],seam_initial_held_seconds=opts['seam_initial_held_seconds'],
        paired_diagonal_guidance=opts['paired_diagonal_guidance'])
    a=forehead_consumer_copy.deepcopy(author)
    capture=forehead_consumer_copy.deepcopy(native_capture)
    binding=forehead_consumer_binding(raw_source,a,capture,pose_context)
    out=dict(schema_version=1,status='initializing',source_binding=binding,
        baseline_names={'raw':'original native source','author':'regularized author source',
                        'current':'positive current proposal with immutable frozen rows'},
        native_ready=False,full_model_success=False,shape_checked=False,
        author_shape_change=forehead_consumer_copy.deepcopy(raw_source.get('author_shape_change')),
        restoration_policy='optimizing_restore_even_with_zero_progression',
        prior_geometry_evidence_invalidated=True,
        stages=[],pending_contracts=['source-owned support admission','full posed shape reevaluation',
            'final smoothness/visual criteria','bilateral native UV/field/save/readback'],
        positive_proposal=None)
    def publish():
        payload=forehead_consumer_copy.deepcopy(out)
        blob=forehead_consumer_checkpoint_bytes(payload)
        receipt=checkpoint(payload)
        if (not isinstance(receipt,dict) or receipt.get('persisted') is not True
                or receipt.get('sha256')!=forehead_consumer_hashlib.sha256(blob).hexdigest()):
            raise RuntimeError('Checkpoint sink did not confirm exact durable snapshot')

    if opts['mode']=='fresh':
        initial=initialize_forehead_shared(a,capture,**opts.get('initialization_options',{}))
        out['initialization']=initial
        out['positive_proposal']=initial.get('proposal')
        if not initial.get('admitted') or initial.get('status')!='initial_atlas_ready':
            out['status']=initial['status'];publish();return out
        current=initial['proposal']
    else:
        saved=opts.get('resume')
        if not isinstance(saved,dict) or saved.get('schema_version')!=1 or saved.get('source_binding')!=binding:
            raise ValueError('Resume source/raw/author/native/context binding mismatch')
        current=forehead_consumer_copy.deepcopy(saved.get('positive_proposal'))
        if not isinstance(current,dict) or 'guide_paths' in current or 'face_owners' in current:
            raise ValueError('Resume requires an unmirrored positive proposal')
        if not current.get('joint_stage_caps') or len(current.get('matched_gap_caps_v2',{}))!=10:
            raise ValueError('Resume requires retained working caps; no reset/bootstrap')
    # Fresh: bootstrap working caps once. Resume: explicitly OPTIMIZING restore
    # under EXISTING caps, even with zero progression. This may move coordinates;
    # all prior shape/geometry evidence is invalidated. Neither path refreezes arcs.
    try:
        restored=solve(a['vertices'],a['open_delta'],
            forehead_consumer_copy.deepcopy(current),restore=True,physical_stage=True,
            progress_names=[],gap_reduction=0.,objective_mode='restore',minimum_improvement=0.,
            solver_mode='linearized',max_iterations=opts['max_iterations'],author_context=a,
            seam_active_set_growth=opts['seam_active_set_growth'])
        forehead_consumer_retains(current,restored)
    except ValueError as error:
        out.update(status='working_caps_or_restore_pending',positive_proposal=current,reason=str(error))
        publish();return out
    movement=[1000*forehead_consumer_math.sqrt(sum((float(x)-float(y))**2 for x,y in zip(a,b)))
              for a,b in zip(current['vertices'],restored['vertices'])]
    out['restoration']={'mode':'optimizing_restore','target_tightening':False,
        'moved_vertex_count':sum(value>0. for value in movement),
        'maximum_vertex_displacement_mm':max(movement,default=0.),
        'geometry_changed':forehead_consumer_canonical(current['vertices'])!=forehead_consumer_canonical(restored['vertices']),
        'step':'fresh_cap_bootstrap' if opts['mode']=='fresh' else 'resume_restore',
        'input_geometry_sha256':forehead_consumer_hashlib.sha256(forehead_consumer_checkpoint_bytes({'vertices':current['vertices']})).hexdigest(),
        'output_geometry_sha256':forehead_consumer_hashlib.sha256(forehead_consumer_checkpoint_bytes({'vertices':restored['vertices']})).hexdigest(),
        'fresh_cap_bootstrap':opts['mode']=='fresh','all_geometry_evidence_requires_remeasurement':True}
    current=restored;out.update(status='working_checkpoint',positive_proposal=current);publish()
    for stage, count in (('fairing',opts['fairing_stages']),('flow',opts['flow_steps']),('gap',opts['gap_steps'])):
        if not count:continue
        prior=forehead_consumer_copy.deepcopy(current)
        def stage_checkpoint(result):
            candidate=result['proposal'];forehead_consumer_retains(prior,candidate)
            out.update(status='partial_'+stage,positive_proposal=forehead_consumer_copy.deepcopy(candidate))
            publish()
        kwargs=dict(max_iterations=opts['max_iterations'],checkpoint=stage_checkpoint,fitter=solve,
                    seam_active_set_growth=opts['seam_active_set_growth'])
        if stage=='fairing':
            result=continue_forehead_fairing(a['vertices'],a['open_delta'],current,a,max_stages=count,**kwargs)
        elif stage=='flow':result=continue_forehead_flows(a,current,max_steps=count,**kwargs)
        else:result=continue_forehead_gaps(a,current,max_steps=count,**kwargs)
        current=result['proposal'];forehead_consumer_retains(prior,current)
        out['stages'].append({k:forehead_consumer_copy.deepcopy(v) for k,v in result.items() if k!='proposal'})
        out['positive_proposal']=current
    out['shared_acceptance']=measure_forehead_shared_acceptance(a,current)
    out['status']='final_validation_pending' if out['shared_acceptance']['passed'] else 'partial_shared_goals_pending'
    publish();return out

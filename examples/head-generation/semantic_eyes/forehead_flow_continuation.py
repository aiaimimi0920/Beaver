"""Bounded shared flow continuation with immutable accepted-state checkpoints."""
import copy
from collections.abc import Mapping
import numpy as np
from forehead_shared_fit import fit_forehead_shared_constraints
from forehead_flow_acceptance import measure_forehead_flow
from forehead_physical_stage import ForeheadPhysicalFlowStage
from forehead_lower_guard import forehead_lower_guard_retains


def forehead_flow_progress(author, proposal):
    points = np.asarray(proposal['vertices'], float)
    old = np.asarray(author['vertices'], float)
    if points.ndim != 2 or points.shape[1] != 3 or old.ndim != 2 or old.shape[1] != 3:
        raise ValueError('Invalid current/author point arrays')
    goals = dict(zip(('red', 'orange', 'green', 'blue', 'purple'), (18., 20., 25., 30., 33.)))
    if set(proposal['paths']) != set(goals):
        raise ValueError('Flow continuation requires exactly five source-owned paths')
    refs = proposal.get('uniform_station_reference_length_mm')
    if refs is None or set(refs) != set(goals):
        raise ValueError('Flow continuation requires frozen initial length references')
    metrics, active = {}, []
    for name, path in proposal['paths'].items():
        if (len(path) < 3 or len(set(path)) != len(path) or
                any(not isinstance(i, (int, np.integer)) or isinstance(i, (bool, np.bool_))
                    or i < 0 or i >= len(points) for i in path)):
            raise ValueError('Invalid complete guide path')
        if any(i >= len(old) for i in path[:2]):
            raise ValueError('Author front tangent requires original first-two guide nodes')
        if not np.isfinite(old[path[:2]]).all():
            raise ValueError('Nonfinite authored front tangent')
        lengths = np.linalg.norm(np.diff(points[path], axis=0), axis=1)
        if not len(lengths) or lengths.min() <= 0 or not np.isfinite(lengths).all():
            raise ValueError('Invalid current guide edge length')
        if not np.isfinite(refs[name]) or refs[name] <= 0:
            raise ValueError('Invalid frozen initial length reference')
        peak = measure_forehead_flow(points, path)['sampled_peak_degrees']
        deviation = float(np.max(np.abs(lengths/lengths.mean()-1)))
        front = ForeheadPhysicalFlowStage.front_turn(points[path[:2]])
        front_cap = max(6., ForeheadPhysicalFlowStage.front_turn(old[path[:2]]))
        inflation = float(lengths.sum()*1000/refs[name])
        if not np.isfinite([peak, deviation, front, front_cap, inflation]).all():
            raise ValueError('Nonfinite flow progress metric')
        metrics[name] = dict(physical_degrees=peak, station_deviation=deviation,
                             front_degrees=front, front_cap_degrees=front_cap,
                             original_length_inflation=inflation)
        if peak > goals[name] or deviation > .1 or front > front_cap+1e-8 or inflation > 1.05:
            active.append(name)
    return metrics, active



def forehead_continuation_equal(left, right, lineage=False):
    """Exact recursive equality, treating list/tuple/array representations equally.

    Only an explicitly requested lineage mapping normalizes integer/string IDs.
    Ordinary metadata mappings retain their original key identity. Duplicate
    normalized lineage keys fail closed; NaNs are never equal frozen values.
    """
    if isinstance(left, np.ndarray):
        left = left.tolist()
    if isinstance(right, np.ndarray):
        right = right.tolist()
    if isinstance(left, np.generic):
        left = left.item()
    if isinstance(right, np.generic):
        right = right.item()
    if isinstance(left, Mapping) or isinstance(right, Mapping):
        if not isinstance(left, Mapping) or not isinstance(right, Mapping):
            return False
        if lineage:
            normalized = []
            for value in (left, right):
                if any(isinstance(k, (bool, np.bool_)) or not isinstance(k, (str, int, np.integer)) for k in value):
                    return False
                try:
                    rows = {int(k): v for k, v in value.items()}
                except (TypeError, ValueError):
                    return False
                if len(rows) != len(value):
                    return False
                normalized.append(rows)
            left, right = normalized
        return set(left) == set(right) and all(forehead_continuation_equal(left[k], right[k]) for k in left)
    sequence = (list, tuple)
    if isinstance(left, sequence) or isinstance(right, sequence):
        return (isinstance(left, sequence) and isinstance(right, sequence)
                and len(left) == len(right)
                and all(forehead_continuation_equal(a, b) for a, b in zip(left, right)))
    if isinstance(left, bool) != isinstance(right, bool):
        return False
    try:
        equal = left == right
        return isinstance(equal, (bool, np.bool_)) and bool(equal)
    except (TypeError, ValueError):
        return False


def forehead_continuation_retains(before, after, selected):
    """Reject policy/baseline changes even if a solver incorrectly returns them."""
    for key in ('paths', 'faces', 'protected', 'source_chart', 'source_faces',
                'uniform_station_reference_length_mm', 'matched_gap_caps_v2'):
        if key in before and not forehead_continuation_equal(after.get(key), before[key]):
            raise ValueError('Continuation changed frozen ' + key)
    if 'topology' in before:
        old_topology, new_topology = before['topology'], after.get('topology')
        if not isinstance(old_topology, Mapping) or not isinstance(new_topology, Mapping):
            raise ValueError('Continuation changed frozen topology/field baseline')
        old_other = {k: v for k, v in old_topology.items() if k != 'lineage'}
        new_other = {k: v for k, v in new_topology.items() if k != 'lineage'}
        if (not forehead_continuation_equal(old_other, new_other) or
                not forehead_continuation_equal(old_topology.get('lineage'), new_topology.get('lineage'), lineage=True)):
            raise ValueError('Continuation changed frozen topology/field baseline')
    if 'protected' in before:
        ids = before['protected']
        if not np.array_equal(np.asarray(before['vertices'])[ids], np.asarray(after['vertices'])[ids]):
            raise ValueError('Continuation moved protected coordinates')
    forehead_lower_guard_retains(before, after)
    old_caps = before.get('joint_stage_caps')
    if old_caps is not None:
        new_caps = after.get('joint_stage_caps')
        if not isinstance(new_caps, Mapping) or set(new_caps) != set(old_caps):
            raise ValueError('Continuation changed working cap ownership')
        for kind, caps in old_caps.items():
            if not isinstance(new_caps[kind], Mapping) or set(new_caps[kind]) != set(caps):
                raise ValueError('Continuation changed working cap roles')
            for name, value in caps.items():
                proposed = new_caps[kind][name]
                if (np.ndim(value) != 0 or np.ndim(proposed) != 0 or
                        not np.isfinite(value) or not np.isfinite(proposed) or proposed > value or
                        (name not in selected and proposed != value)):
                    raise ValueError('Continuation loosened or changed an unselected working cap')


def forehead_continuation_diagnostic_value(value):
    """Detached JSON-safe metadata; never retain a raw residual vector or mesh."""
    if isinstance(value, np.ndarray):
        value = value.tolist()
    if isinstance(value, np.generic):
        value = value.item()
    if isinstance(value, Mapping):
        excluded = {'raw_constraints', 'constraint_scales', 'vertices', 'faces', 'open_delta', 'proposal'}
        return {str(k): forehead_continuation_diagnostic_value(v)
                for k, v in value.items() if k not in excluded}
    if isinstance(value, (list, tuple)):
        return [forehead_continuation_diagnostic_value(v) for v in value]
    if isinstance(value, float) and not np.isfinite(value):
        return None
    if value is None or isinstance(value, (str, bool, int, float)):
        return value
    return str(value)


def forehead_continuation_failure_diagnostic(payload):
    """Compact solver evidence, not admission; nonfinite rows count as failures."""
    if not isinstance(payload, Mapping):
        raise ValueError('Expected a solver diagnostic mapping')
    keys = ('stage', 'variable_ids', 'start_parameters_mm', 'parameters_mm',
            'best_parameters_mm', 'constraint_layout', 'solver_success', 'solver_message',
            'lower_turn_guard', 'restoration_continuation', 'linearized_restoration', 'geometry_admission_passed',
            'initial_scalar_cost', 'final_scalar_cost', 'minimum_improvement')
    result = {k: forehead_continuation_diagnostic_value(payload[k]) for k in keys if k in payload}
    if 'raw_constraints' not in payload:
        return result
    raw = np.asarray(payload['raw_constraints'], float)
    if raw.ndim != 1:
        raise ValueError('Expected one-dimensional solver residuals')

    def summary(values):
        finite = np.isfinite(values)
        return {'row_count': int(len(values)),
                'minimum_finite': float(values[finite].min()) if finite.any() else None,
                'negative_count': int(np.count_nonzero(finite & (values < 0))),
                'nonfinite_count': int(np.count_nonzero(~finite)),
                'failure_count': int(np.count_nonzero((finite & (values < 0)) | ~finite))}

    groups, spans = {}, {}
    layout = payload.get('constraint_layout', {})
    if isinstance(layout, Mapping):
        for name, span in layout.items():
            if name == 'count':
                continue
            if (not isinstance(span, (list, tuple, np.ndarray)) or len(span) != 2
                    or any(not isinstance(i, (int, np.integer)) for i in span)):
                continue
            a, b = map(int, span)
            if 0 <= a <= b <= len(raw):
                spans[str(name)] = (a, b)
                groups[str(name)] = summary(raw[a:b])
    finite = np.isfinite(raw)
    order = np.argsort(np.where(finite, raw, -np.inf), kind='stable')[:24]
    result['raw_summary'] = summary(raw)
    result['constraint_groups'] = groups
    result['worst_raw_rows'] = [
        {'index': int(i), 'value': float(raw[i]) if finite[i] else None,
         'nonfinite': None if finite[i] else str(raw[i]),
         'groups': [name for name, (a, b) in spans.items() if a <= i < b]}
        for i in order]
    return result


def forehead_continuation_step_record(scale, metrics):
    """Scheduling only; unknown effort never justifies increasing a step.

    selected_feasible_iteration is NOT total solver effort. Use an explicit
    solver_iterations count, or conservatively its full budget; a restoration
    leg prevents a fast-success promotion. No fitter acceptance is changed.
    """
    effort = metrics.get('solver_work_iterations', metrics.get('solver_iterations', metrics.get('solver_iteration_budget')))
    valid = isinstance(effort, (int, np.integer)) and not isinstance(effort, (bool, np.bool_)) and effort > 0
    fast = valid and effort <= 15 and not metrics.get('restoration_continuation')
    levels = (1., .5, .25, .1, .05)
    if scale not in levels:
        raise ValueError('Unknown bounded continuation scale')
    next_scale = levels[max(0, levels.index(scale)-1)] if fast else scale
    return {'next_scale': next_scale, 'last_success_scale': scale,
            'iteration_evidence': int(effort) if valid else None,
            'iteration_evidence_kind': ('solver_work_iterations' if 'solver_work_iterations' in metrics else
                                        'solver_iterations' if 'solver_iterations' in metrics else 'budget_upper_bound'),
            'reason': 'Fast admitted success: increase one level' if fast else
                      'Retain admitted scale: slow, unknown effort, or restoration required'}


def forehead_continuation_step_schedule(proposal):
    """Recover per-role scheduling hints, never caps or geometry references."""
    schedule = {name: {'next_scale': 1., 'reason': 'Unknown history: default full step'}
                for name in proposal['paths']}
    prior = proposal.get('flow_continuation_schedule', {})
    recovered = set()
    if isinstance(prior, Mapping):
        for name, record in prior.items():
            if (name in schedule and isinstance(record, Mapping) and
                    type(record.get('next_scale')) in (int, float) and
                    record['next_scale'] in (1., .5, .25, .1, .05)):
                schedule[name] = copy.deepcopy(record)
                recovered.add(name)
    metrics = proposal.get('metrics', {})
    evidence = metrics.get('physical_flow_stage', {}) if isinstance(metrics, Mapping) else {}
    scale = evidence.get('step_scale') if isinstance(evidence, Mapping) else None
    names = evidence.get('tightened_curves', ()) if isinstance(evidence, Mapping) else ()
    if type(scale) in (int, float) and scale in (1., .5, .25, .1, .05) and isinstance(names, (list, tuple)):
        for name in names:
            if isinstance(name, str) and name in schedule and name not in recovered:
                schedule[name] = forehead_continuation_step_record(scale, metrics)
                schedule[name]['initialized_from'] = 'physical_flow_stage'
    return schedule


def continue_forehead_flows(author, proposal, max_steps=40, max_iterations=120,
                            grouped=True, checkpoint=None, seam_active_set_growth=False):
    """Return admitted progress or a diagnostic stop, never final model success.

    Each rejected trial is isolated. The caller may diagnose capacity and apply a
    source-owned topology transaction before calling again on the accepted state.
    A checkpoint callback receives a detached snapshot, not mutable solver state.
    """
    if type(seam_active_set_growth) is not bool:
        raise ValueError('Invalid optional seam active-set mode')
    if type(max_steps) is not int or not 1 <= max_steps <= 100:
        raise ValueError('Invalid continuation step budget')
    if type(max_iterations) is not int or not 1 <= max_iterations <= 1000:
        raise ValueError('Invalid continuation iteration budget')
    if type(grouped) is not bool or (checkpoint is not None and not callable(checkpoint)):
        raise ValueError('Invalid continuation grouping/checkpoint')
    accepted = copy.deepcopy(proposal)
    frozen_refs = copy.deepcopy(accepted.get('uniform_station_reference_length_mm'))
    metrics, active = forehead_flow_progress(author, accepted)
    schedule = forehead_continuation_step_schedule(accepted)
    result = {'proposal': accepted, 'stages': [], 'attempts': [],
              'flow_goals_passed': False, 'full_model_success': False,
              'scope': 'Along-guide continuation only; transverse spacing, poses, '
                       'native fields, saved mesh, GLB, engine and visual checks remain.'}

    def save():
        result['proposal'] = accepted
        result['metrics'] = metrics
        result['remaining_roles'] = active
        result['step_schedule'] = copy.deepcopy(schedule)
        result['flow_goals_passed'] = not active
        if checkpoint is not None:
            checkpoint(copy.deepcopy(result))

    save()
    blocked_roles = set()
    for step in range(max_steps):
        if not active:
            break
        groups = ([list(group) for group in (('red', 'green'), ('orange',), ('blue', 'purple'))]
                  if grouped else [list(active)])
        candidates = [[name for name in group if name in active] for group in groups]
        candidates = [group for group in candidates if group and not set(group) <= blocked_roles]
        if not candidates:
            result['stop_reason'] = 'All active role groups blocked; accepted state retained'
            break
        selected = candidates[0]
        admitted = False
        initial_scale = min(schedule[name]['next_scale'] for name in selected)
        scheduling = {name: copy.deepcopy(schedule[name]) for name in selected}
        for scale in (value for value in (1., .5, .25, .1, .05) if value <= initial_scale):
            trial_diagnostic = {}

            def capture_solver_diagnostic(payload):
                # Observation must not turn solver success/failure into a different outcome.
                try:
                    trial_diagnostic.update(forehead_continuation_failure_diagnostic(payload))
                except (TypeError, ValueError, OverflowError) as error:
                    trial_diagnostic['capture_error'] = str(error)

            try:
                trial_author = copy.deepcopy(author)
                candidate = fit_forehead_shared_constraints(
                    np.array(trial_author['vertices'], copy=True), np.array(trial_author['open_delta'], copy=True),
                    copy.deepcopy(accepted), restore=True, physical_stage=True,
                    author_context=trial_author, progress_names=list(selected), progress_scale=scale,
                    gap_reduction=0., max_iterations=max_iterations,
                    objective_mode='fairing', minimum_improvement=0., solver_mode='linearized',
                    diagnostic=capture_solver_diagnostic, seam_active_set_growth=seam_active_set_growth)
                if not forehead_continuation_equal(candidate.get('uniform_station_reference_length_mm'), frozen_refs):
                    raise ValueError('Continuation changed frozen initial length references')
                forehead_continuation_retains(accepted, candidate, selected)
                next_metrics, next_active = forehead_flow_progress(author, candidate)
            except ValueError as error:
                attempt = {'step': step, 'scale': scale,
                           'roles': list(selected), 'error': str(error),
                           'scheduling': copy.deepcopy(scheduling)}
                if trial_diagnostic:
                    attempt['solver_diagnostic'] = dict(trial_diagnostic,
                        diagnostic_only=True, admitted=False, full_model_success=False)
                result['attempts'].append(attempt)
                save()
                continue
            for name in selected:
                schedule[name] = forehead_continuation_step_record(scale, candidate.get('metrics', {}))
            candidate['flow_continuation_schedule'] = copy.deepcopy(schedule)
            accepted, metrics, active = candidate, next_metrics, next_active
            result['stages'].append({'step': step, 'scale': scale,
                                     'roles': list(selected), 'metrics': copy.deepcopy(metrics),
                                     'scheduling_before': copy.deepcopy(scheduling),
                                     'scheduling_after': {name: copy.deepcopy(schedule[name]) for name in selected}})
            admitted = True
            blocked_roles.clear()
            save()
            break
        if not admitted:
            blocked_roles.update(selected)
            result['blocked_roles'] = sorted(blocked_roles)
            if set(active) <= blocked_roles:
                result['stop_reason'] = 'All active role groups blocked; source capacity or conditioning requires diagnosis'
                break
    if active and 'stop_reason' not in result:
        result['stop_reason'] = 'Continuation budget exhausted; accepted state retained, not completion'
    save()
    return result

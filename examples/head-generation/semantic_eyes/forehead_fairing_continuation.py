"""Bounded fairing-only stages; no gap/flow target scheduling or model upgrade."""
import copy as forehead_fairing_continuation_copy
import numpy as np
from orbital_support_rule import orbital_turns
from forehead_shared_fit import fit_forehead_shared_constraints


def continue_forehead_fairing(vertices, old_open_delta, proposal, author_context,
                              max_stages=1, reduction=.001, max_iterations=80,
                              role_weights=None, turn_reduction_degrees=0., turn_goals=None,
                              seam_active_set_growth=False, checkpoint=None,
                              diagnostic=None, fitter=None, guard_mode=None, complete_guide_fairing=None):
    if type(max_stages) is not int or not 1 <= max_stages <= 20:
        raise ValueError('Invalid bounded fairing stage count')
    solve = fit_forehead_shared_constraints if fitter is None else fitter
    current = forehead_fairing_continuation_copy.deepcopy(proposal)
    result = {'proposal': current, 'stages': [], 'fit_complete': False, 'component_admitted': False, 'checkpoint_status': 'unchanged_input',
              'attempts': [], 'full_model_success': False, 'scope': 'Fairing components only; dense shape/native/visual gates remain mandatory'}
    schedule = forehead_fairing_continuation_copy.deepcopy(current.get('whole_guide_fairing_schedule', {}))
    levels = (1., .5, .25)
    if any(role not in current.get('paths', {}) or value not in levels for role, value in schedule.items()):
        raise ValueError('Invalid retained fairing fraction schedule')
    previous = current.get('metrics', {}).get('guide_fairing', {})
    ratio = previous.get('requested_reduction', 0.)/reduction if reduction else 0.
    if ratio in levels:
        for role in previous.get('progression_roles', []):
            if role in current.get('paths', {}):
                schedule.setdefault(role, ratio)
    blocked = set()
    while len(result['stages']) < max_stages:
        index = len(result['stages'])
        selected = None
        if turn_goals is not None:
            if set(turn_goals) != set(current['paths']):
                raise ValueError('Local-turn goals must cover all whole paths')
            peaks = {r: float(max(orbital_turns(np.asarray(current['vertices']),
                np.asarray(list(zip(p, p[1:], p[2:])), int)), default=0.)) for r, p in current['paths'].items()}
            violating = [r for r in peaks if peaks[r] > turn_goals[r]]
            available = [r for r in violating if r not in blocked]
            if violating and not available:
                result['stop_reason'] = 'All violating fairing roles blocked; accepted state retained'
                break
            selected = [] if not available else [max(available, key=lambda r: peaks[r]-turn_goals[r])]
        role = selected[0] if selected else None
        hint = schedule.get(role, 1.)
        steps = levels[levels.index(hint):] if turn_reduction_degrees else (1.,)
        candidate = None
        for fraction in steps:
            attempt = {'stage': index, 'fraction': fraction, 'roles': selected,
                       'schedule_reason': 'retained_role_hint' if role in schedule else 'unknown_history_full_step'}
            def trial_diagnostic(value):
                if diagnostic is not None:
                    record = dict(value)
                    record['fairing_attempt'] = dict(attempt, sequence=len(result['attempts']))
                    diagnostic(record)
            try:
                candidate = solve(vertices, old_open_delta,
                    forehead_fairing_continuation_copy.deepcopy(current),
                    restore=True, physical_stage=True, progress_names=[], gap_reduction=0.,
                    author_context=author_context, objective_mode='fairing', minimum_improvement=0.,
                    solver_mode='linearized', max_iterations=max_iterations,
                    seam_active_set_growth=seam_active_set_growth, fairing_reduction=reduction*fraction,
                    fairing_role_weights=role_weights,
                    fairing_turn_reduction=turn_reduction_degrees*fraction,
                    fairing_turn_goals=turn_goals, fairing_turn_roles=selected, diagnostic=trial_diagnostic,
                    fairing_guard_mode=guard_mode, complete_guide_fairing=complete_guide_fairing)
            except ValueError as error:
                result['attempts'].append(dict(attempt, admitted=False, reason=str(error)))
                continue
            result['attempts'].append(dict(attempt, admitted=True))
            break
        if candidate is None:
            if role is not None:
                blocked.add(role)
                continue
            result['stop_reason'] = 'All bounded fairing reductions rejected'
            break
        if role is not None:
            work = candidate.get('metrics', {}).get('solver_work_iterations')
            quick = isinstance(work, (int, float)) and 0 <= work <= 15
            schedule[role] = levels[max(0, levels.index(fraction)-1)] if quick else fraction
        current = candidate
        current['whole_guide_fairing_schedule'] = forehead_fairing_continuation_copy.deepcopy(schedule)
        blocked.clear()
        result['proposal'] = current
        result.update(component_admitted=True, checkpoint_status='admitted_component',
                      fit_complete=True, full_model_success=False)
        result['stages'].append(forehead_fairing_continuation_copy.deepcopy(current['metrics']['guide_fairing']))
        if checkpoint is not None:
            checkpoint(forehead_fairing_continuation_copy.deepcopy(result))
    result['blocked_roles'] = sorted(blocked)
    result['fraction_schedule'] = schedule
    return result

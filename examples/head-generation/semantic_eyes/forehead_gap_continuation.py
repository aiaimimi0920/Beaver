"""Bounded transverse-only continuation; no I/O or full-model acceptance."""
import copy
import numpy as np
from forehead_shared_fit import fit_forehead_shared_constraints
from forehead_shared_acceptance import measure_forehead_shared_acceptance
from forehead_flow_continuation import forehead_continuation_retains


def forehead_gap_continuation_measure(author, proposal, domains):
    """Independent dense measurement, never working-cap-derived acceptance."""
    evidence = measure_forehead_shared_acceptance(author, proposal)
    errors = evidence.get('shared_gap_maximum_relative_deviation')
    if not isinstance(errors, dict) or set(errors) != set(domains):
        raise ValueError('Frozen ten-domain atlas unavailable or ownership changed')
    if any(not np.isfinite(x) or x < 0 for x in errors.values()):
        raise ValueError('Nonfinite/negative dense gap measurement')
    caps = proposal['matched_gap_caps_v2']
    if any(errors[name] > caps[name] + 1e-9 for name in domains):
        raise ValueError('Input or candidate violates retained dense gap caps')
    return evidence, {name: float(value) for name, value in errors.items()}


def forehead_gap_continuation_retains(before, after, selected):
    """Allow one strict gap-cap reduction; retain all other original contracts."""
    old = before['matched_gap_caps_v2']
    new = after.get('matched_gap_caps_v2')
    if not isinstance(new, dict) or set(new) != set(old):
        raise ValueError('Gap cap ownership changed')
    for name, value in new.items():
        if (np.ndim(value) != 0 or not np.isfinite(value) or value < .099
                or value > old[name] or (name != selected and value != old[name])):
            raise ValueError('Gap continuation loosened or changed unrelated cap')
    if new[selected] >= old[selected]:
        raise ValueError('Selected domain did not tighten')
    # The existing guard requires gap caps equal. Omit ONLY that one metadata
    # field from detached comparison views after the explicit check above.
    left, right = copy.deepcopy(before), copy.deepcopy(after)
    left.pop('matched_gap_caps_v2'); right.pop('matched_gap_caps_v2')
    forehead_continuation_retains(left, right, selected=())


def forehead_gap_fraction_schedule(proposal, domains, fractions):
    """Persist scheduling hints only; they never define caps or admission."""
    prior = proposal.get('gap_continuation_schedule', {})
    schedule = {}
    for domain in domains:
        record = prior.get(domain, {}) if isinstance(prior, dict) else {}
        value = record.get('next_fraction') if isinstance(record, dict) else None
        valid = (type(value) in (int, float) and value in fractions)
        schedule[domain] = copy.deepcopy(record) if valid else {
            'next_fraction': fractions[0], 'reason': 'No compatible prior schedule'}
    return schedule


def forehead_gap_fraction_record(fraction, fractions, metrics):
    """Only verified TOTAL work can promote; major LP count is insufficient."""
    work = metrics.get('solver_work_iterations')
    verified = (isinstance(work, (int, np.integer)) and not isinstance(work, (bool, np.bool_))
                and work > 0)
    fast = verified and work <= 15 and not metrics.get('restoration_continuation')
    next_fraction = fractions[max(0, fractions.index(fraction)-1)] if fast else fraction
    budget = metrics.get('solver_iteration_budget')
    return dict(next_fraction=next_fraction, last_success_fraction=fraction,
                solver_work_iterations=int(work) if verified else None,
                budget_hint=int(budget) if type(budget) is int and budget > 0 else None,
                reason='Verified fast total work: promote one level' if fast else
                       'Retain admitted fraction: slow or unverified total work')


def continue_forehead_gaps(author, proposal, max_steps=20, max_iterations=120,
                           reduction=.025, fractions=(1., .5, .25, .1, .05),
                           checkpoint=None, seam_active_set_growth=False, fitter=None):
    """Operate only on an explicitly accepted input with all ten frozen caps.

    The caller owns durable atomic checkpoint writing. Callback receives a
    detached complete snapshot; write atomically and raise on failure. Such an
    exception propagates, never becoming a geometry rejection or success.
    No source/atlas/target clipping, inferred cap reset, or baseline refresh.
    Along-flow final readiness is reported independently, not assumed here.
    """
    solve = fit_forehead_shared_constraints if fitter is None else fitter
    if not callable(solve):
        raise ValueError('Invalid continuation fitter adapter')
    if type(seam_active_set_growth) is not bool:
        raise ValueError('Invalid seam active-set option')
    if type(max_steps) is not int or not 1 <= max_steps <= 100:
        raise ValueError('Invalid bounded gap step count')
    if type(max_iterations) is not int or not 1 <= max_iterations <= 1000:
        raise ValueError('Invalid bounded fitter iteration count')
    if not np.isfinite(reduction) or not 0 < reduction <= .025:
        raise ValueError('Invalid bounded gap reduction')
    fractions = tuple(fractions)
    if (not fractions or len(fractions) > 5
            or any(not np.isfinite(x) or not 0 < x <= 1 for x in fractions)
            or any(a <= b for a, b in zip(fractions, fractions[1:]))):
        raise ValueError('Fractions must be strictly descending positive bounds')
    if checkpoint is not None and not callable(checkpoint):
        raise ValueError('Invalid atomic checkpoint callback')
    frozen_author, accepted = copy.deepcopy(author), copy.deepcopy(proposal)
    caps = accepted.get('matched_gap_caps_v2')
    if (not isinstance(caps, dict) or len(caps) != 10
            or any(not isinstance(k, str) or ':' not in k for k in caps)
            or any(np.ndim(v) != 0 or not np.isfinite(v) or v < .099 for v in caps.values())
            or not accepted.get('joint_stage_caps')
            or not accepted.get('uniform_station_reference_length_mm')):
        raise ValueError('Explicit ten gap caps, along-flow caps and frozen references required')
    domains = tuple(caps)
    schedule = forehead_gap_fraction_schedule(accepted, domains, fractions)
    evidence, errors = forehead_gap_continuation_measure(frozen_author, accepted, domains)
    result = dict(proposal=accepted, stages=[], attempts=[], transverse_goals_passed=False,
                  full_model_success=False, scope='Transverse component only; along-flow, '
                  '1mm posed shape, fields/UV/native, export, engine and visual gates remain.')

    def publish():
        result.update(proposal=accepted, acceptance=evidence, dense_gap_errors=errors,
                      transverse_goals_passed=all(v <= .1 for v in errors.values()),
                      remaining_domains=[n for n in domains if errors[n] > .1],
                      fraction_schedule=copy.deepcopy(schedule))
        if checkpoint is not None:
            checkpoint(copy.deepcopy(result))

    publish()
    for step in range(max_steps):
        if result['transverse_goals_passed']:
            break
        progressed = False
        pending = sorted((n for n in domains if errors[n] > .1),
                         key=lambda n: (-errors[n], n))
        for domain in pending:
            for fraction in (x for x in fractions if x <= schedule[domain]['next_fraction']):
                try:
                    trial_author = copy.deepcopy(frozen_author)
                    candidate = solve(
                        np.array(trial_author['vertices'], copy=True),
                        np.array(trial_author['open_delta'], copy=True),
                        copy.deepcopy(accepted), restore=True, physical_stage=True,
                        author_context=trial_author, progress_names=[],
                        active_gap_pair=domain, gap_reduction=reduction*fraction,
                        gap_sample_count=129, max_iterations=max_iterations,
                        objective_mode='restore', solver_mode='linearized',
                        seam_active_set_growth=seam_active_set_growth)
                    forehead_gap_continuation_retains(accepted, candidate, domain)
                    measured, next_errors = forehead_gap_continuation_measure(
                        frozen_author, candidate, domains)
                except ValueError as error:
                    result['attempts'].append(dict(step=step, domain=domain,
                        reduction=reduction*fraction, admitted=False, error=str(error)))
                    publish()
                    continue
                old_cap = accepted['matched_gap_caps_v2'][domain]
                schedule[domain] = forehead_gap_fraction_record(
                    fraction, fractions, candidate.get('metrics', {}))
                candidate['gap_continuation_schedule'] = copy.deepcopy(schedule)
                accepted, evidence, errors = candidate, measured, next_errors
                result['stages'].append(dict(step=step, domain=domain,
                    reduction=reduction*fraction, prior_cap=old_cap,
                    admitted_cap=accepted['matched_gap_caps_v2'][domain],
                    schedule=copy.deepcopy(schedule[domain]),
                    measured_error=errors[domain], full_model_success=False))
                progressed = True
                publish()
                break
            if progressed:
                break
        if not progressed:
            result['stop_reason'] = 'All remaining domains blocked at bounded reductions'
            break
    if not result['transverse_goals_passed'] and 'stop_reason' not in result:
        result['stop_reason'] = 'Bounded transverse step budget exhausted'
    publish()
    return result

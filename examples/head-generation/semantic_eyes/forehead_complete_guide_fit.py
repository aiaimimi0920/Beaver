"""Integration of supplemental complete-guide obligations into the active fit."""
import copy as forehead_complete_fit_copy
from forehead_complete_guide_stage import forehead_prepare_complete_guide_stage
from forehead_seam_hint import forehead_seam_source_identity


def forehead_install_complete_stage(proposal, initial, source_arrays, layout, config):
    identity = forehead_seam_source_identity(source_arrays, proposal['topology']['faces'])
    stage = forehead_prepare_complete_guide_stage(proposal, initial, identity, config)
    if stage:
        offset = layout['count']
        layout['complete_guide_fairing'] = (offset, offset+stage.count)
        layout['count'] += stage.count
    return stage


def forehead_complete_objective(stage, config, previous, derivative, evaluate, start):
    if stage is None or config is None:
        return previous, derivative
    def objective(values):
        state = evaluate(values)
        if state is None:
            raise ValueError('Complete-guide objective outside original source')
        return previous(values)+stage.objective(state[0])
    def gradient(values):
        stage.charge('prior_objective_derivative')
        old = derivative(values) if derivative is not None else 2*(values-start)/len(start)
        return old+stage.parameter_gradient(values, evaluate)
    return objective, gradient


def forehead_complete_charge(stage, kind):
    if stage:
        stage.charge(kind)


def forehead_complete_saved_accepts(stage, points):
    return stage is None or stage.accepts(points)


def forehead_complete_fit_capture(stage, proposal, points):
    if stage is None:
        return None
    return {'guard': stage.capture(proposal, points),
            'reference': forehead_complete_fit_copy.deepcopy(stage.reference)}


def forehead_complete_fit_publish(proposal, result, stage):
    if result is not None:
        proposal['complete_guide_fairing_guard'] = result['guard']
        proposal['complete_guide_fairing_reference'] = result['reference']
        proposal['metrics']['complete_guide_fairing'] = stage.evidence()

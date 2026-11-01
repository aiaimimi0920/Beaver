"""Monotone owned working caps; final style and uniformity goals never change."""
from copy import deepcopy
import numpy as np
from forehead_physical_stage import ForeheadPhysicalFlowStage
from forehead_station_stage import ForeheadUniformStationStage
from forehead_flow_acceptance import measure_forehead_flow


def make_forehead_working_flow_stage(*args, **kwargs):
    return ForeheadWorkingFlowStage(*args, **kwargs)


class ForeheadWorkingFlowStage:
    def __init__(self, old, initial, paths, reference_length_mm=None,
                 cap_state=None, advance_names=(), step_scale=1.):
        active = set(advance_names)
        if not active <= set(paths) or not 0 < step_scale <= 1:
            raise ValueError('Invalid bounded flow target schedule')
        self.active = sorted(active)
        self.step_scale = float(step_scale) if active else 0.
        self.flow = ForeheadPhysicalFlowStage(old, initial, paths, maximum_reduction=0.)
        self.spacing = ForeheadUniformStationStage(initial, paths, maximum_reduction=0.,
                                                   reference_length_mm=reference_length_mm)
        peaks = {name: measure_forehead_flow(initial, path)['sampled_peak_degrees']
                 for name, path in paths.items()}
        front = {name: self.flow.front_turn(np.asarray(initial)[path[:2]])
                 for name, path in paths.items()}
        deviations = {}
        for name, path in paths.items():
            length = np.linalg.norm(np.diff(np.asarray(initial)[path], axis=0), axis=1)
            deviations[name] = float(np.max(np.abs(length/length.mean()-1)))
        state = deepcopy(cap_state) if cap_state is not None else {
            'physical': {n: max(self.flow.goals[n]-.5, peaks[n]+.05) for n in paths},
            'front': {n: max(self.flow.front_goals[n], front[n]+.05) for n in paths},
            'station': {n: max(self.spacing.goal, deviations[n]+.001) for n in paths}}
        if any(set(state[key]) != set(paths) for key in ('physical', 'front', 'station')):
            raise ValueError('Stale flow target ownership')
        for name in active:
            state['physical'][name] = max(self.flow.goals[name]-.5,
                min(state['physical'][name], peaks[name]+.05)-2*step_scale)
            state['front'][name] = max(self.flow.front_goals[name],
                min(state['front'][name], front[name]+.05)-2*step_scale)
            state['station'][name] = max(self.spacing.goal,
                min(state['station'][name], deviations[name]+.001)-.1*step_scale)
        self.state = state
        self.flow.caps, self.flow.front_caps = state['physical'], state['front']
        self.spacing.caps = state['station']
        self.flow.maximum_reduction = 2*self.step_scale
        self.spacing.reduction = .1*self.step_scale
        self.groups = list(self.flow.groups) + [
            (a+self.flow.count, b+self.flow.count, nodes)
            for a, b, nodes in self.spacing.groups]
        self.count = self.flow.count+self.spacing.count

    def constraints(self, points, interior=True):
        return np.r_[self.flow.constraints(points, interior),
                     self.spacing.constraints(points, interior)]

    def accepts(self, points):
        return self.flow.accepts(points) and self.spacing.accepts(points)

    def evidence(self):
        return {'physical_flow': self.flow.evidence(),
                'uniform_station': self.spacing.evidence(),
                'tightened_curves': self.active, 'step_scale': self.step_scale,
                'all_other_working_caps_retained': True}

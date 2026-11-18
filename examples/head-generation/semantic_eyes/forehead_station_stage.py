"""Whole-guide uniform-station homotopy; no tail or short segment is omitted."""
import numpy as np

def make_forehead_station_stage(*args, **kwargs):
    """Construct a source-owned working constraint stage."""
    return ForeheadUniformStationStage(*args, **kwargs)


class ForeheadUniformStationStage:

    def __init__(self, initial, paths, goal=0.1, maximum_reduction=0.1, reference_length_mm=None):
        self.paths = paths
        self.goal = float(goal)
        self.reduction = float(maximum_reduction)
        self.caps = {}
        self.groups = []
        self.reference = dict(reference_length_mm or {})
        offset = 0
        for name, path in paths.items():
            length = np.linalg.norm(np.diff(np.asarray(initial)[path], axis=0), axis=1)
            if length.min() <= 1e-09:
                raise ValueError('Collapsed station in uniform-stage initialization')
            deviation = float(np.max(np.abs(length / length.mean() - 1)))
            self.caps[name] = max(self.goal, deviation - self.reduction)
            self.reference.setdefault(name, float(length.sum() * 1000))
            if self.reference[name] <= 0:
                raise ValueError('Invalid frozen curve length reference')
            count = 2 * len(length) + 1
            self.groups.append((offset, offset + count, list(path)))
            offset += count
        self.count = offset

    def constraints(self, points, interior=True):
        rows = []
        margin = 0.0001 if interior else 0.0
        for name, path in self.paths.items():
            length = np.linalg.norm(np.diff(np.asarray(points)[path], axis=0), axis=1)
            if length.min() <= 1e-09:
                return np.full(self.count, -1000.0)
            relative = length / length.mean() - 1.0
            cap = self.caps[name] - margin
            rows.extend(cap - relative)
            rows.extend(cap + relative)
            rows.append(1.05 - float(length.sum() * 1000) / self.reference[name])
        return np.asarray(rows)

    def accepts(self, points):
        return bool(np.all(self.constraints(points, interior=False) >= -1e-09))

    def evidence(self):
        return {'whole_path_relative_deviation_caps': self.caps, 'final_relative_deviation_goal': self.goal, 'maximum_reduction_per_stage': self.reduction, 'reference_curve_length_mm': self.reference, 'maximum_length_inflation_ratio': 1.05, 'no_excluded_stations_or_tails': True, 'final_uniform_targets_reached': all((v == self.goal for v in self.caps.values())), 'scope': 'Internal fitting bounds, not user visual acceptance; transverse gaps remain separately checked'}

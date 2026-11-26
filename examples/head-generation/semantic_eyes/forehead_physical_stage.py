"""Progressive physical-flow constraints; dense final acceptance stays separate."""
import numpy as np
from forehead_flow_acceptance import measure_forehead_flow

def make_forehead_physical_stage(*args, **kwargs):
    """Construct a source-owned working constraint stage."""
    return ForeheadPhysicalFlowStage(*args, **kwargs)


class ForeheadPhysicalFlowStage:

    def __init__(self, old, initial, paths, step=0.01, samples=129, maximum_reduction=4.0):
        self.paths = paths
        self.step = step
        self.samples = samples
        self.maximum_reduction = float(maximum_reduction)
        self.goals = {'red': 18.0, 'orange': 20.0, 'green': 25.0, 'blue': 30.0, 'purple': 33.0}
        self.caps = {}
        self.front_caps = {}
        self.front_goals = {}
        self.groups = []
        offset = 0
        for name, path in paths.items():
            peak = measure_forehead_flow(initial, path)['sampled_peak_degrees']
            self.caps[name] = max(self.goals[name] - 0.5, peak - maximum_reduction)
            size = samples + 3 * len(path)
            self.groups.append((offset, offset + size, list(path)))
            offset += size
            assert path[0] < len(old) and path[1] < len(old)
            self.front_goals[name] = max(6.0, self.front_turn(np.asarray(old)[path[:2]]))
            self.front_caps[name] = max(self.front_goals[name], self.front_turn(np.asarray(initial)[path[:2]]) - maximum_reduction)
        for name, path in paths.items():
            self.groups.append((offset, offset + 1, list(path[:2])))
            offset += 1
        self.count = offset

    @staticmethod
    def front_turn(pair):
        a, b = pair
        return float(2 * np.degrees(np.arctan2(abs(b[2] - a[2]), abs(b[0] - a[0]))))

    def constraints(self, points, interior=True):
        result = []
        margin = 0.02 if interior else 0.0
        for name, path in self.paths.items():
            curve = np.asarray(points)[path]
            length = np.r_[0.0, np.cumsum(np.linalg.norm(np.diff(curve, axis=0), axis=1))]
            if np.any(np.diff(length) <= 1e-09) or length[-1] <= 2 * self.step:
                return np.full(self.count, -1000.0)
            centers = np.r_[np.linspace(self.step, length[-1] - self.step, self.samples), np.clip(length, self.step, length[-1] - self.step), np.clip(length - self.step, self.step, length[-1] - self.step), np.clip(length + self.step, self.step, length[-1] - self.step)]
            a, b, c = [np.column_stack([np.interp(centers + d, length, curve[:, k]) for k in range(3)]) for d in (-self.step, 0.0, self.step)]
            u, v = (b - a, c - b)
            den = np.linalg.norm(u, axis=1) * np.linalg.norm(v, axis=1)
            if den.min() <= 1e-20:
                return np.full(self.count, -1000.0)
            angle = np.degrees(np.arccos(np.clip(np.einsum('ij,ij->i', u, v) / den, -1.0, 1.0)))
            result.extend(self.caps[name] - margin - angle)
        result.extend((self.front_caps[name] - margin - self.front_turn(np.asarray(points)[path[:2]]) for name, path in self.paths.items()))
        return np.asarray(result)

    def accepts(self, points):
        return bool(np.all(self.constraints(points, interior=False) >= -1e-08))

    def evidence(self):
        return {'physical_stage_caps_degrees': self.caps, 'final_style_caps_degrees': self.goals, 'front_stage_caps_degrees': self.front_caps, 'front_author_caps_degrees': self.front_goals, 'final_targets_reached': all((abs(self.caps[n] - (self.goals[n] - 0.5)) < 1e-08 and abs(self.front_caps[n] - self.front_goals[n]) < 1e-08 for n in self.paths)), 'maximum_reduction_per_stage_degrees': self.maximum_reduction, 'fit_samples_per_curve': self.samples, 'all_vertex_and_shifted_vertex_chord_centers_included': True, 'scope': 'Progressive fitting constraints only; dense flow,1mm full-pose, native saved mesh/GLB and visual checks remain independent'}

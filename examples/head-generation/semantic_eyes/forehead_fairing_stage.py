"""Persistent whole-path bending/variation reduction and local-turn guards.

Component admission is not dense shape, native-field or visual acceptance.
Reference coefficients persist across stages; topology changes require an explicit
caller-owned reference migration, never an automatic geometric baseline reset.
"""
import copy as forehead_fairing_copy
import numpy as np
from forehead_guide_fairing import forehead_polyline_fairing, forehead_fairing_parameter_gradient
from orbital_support_rule import orbital_turns


def make_forehead_fairing_stage(*args, **kwargs):
    return ForeheadFairingStage(*args, **kwargs)


class ForeheadFairingStage:
    def __init__(self, points, paths, lengths_mm, reference=None, reduction=.001,
                 role_weights=None, turn_reduction_degrees=0., turn_goals=None, turn_roles=None):
        if not np.isfinite(reduction) or not 0 <= reduction < 1 or (reduction == 0 and reference is None):
            raise ValueError('Fairing reduction must be positive and below one')
        if not np.isfinite(turn_reduction_degrees) or turn_reduction_degrees < 0:
            raise ValueError('Invalid local turn reduction')
        points = np.asarray(points, float)
        if points.ndim != 2 or points.shape[1] != 3 or not np.isfinite(points).all() or not paths:
            raise ValueError('Invalid complete guide geometry')
        for path in paths.values():
            ids = np.asarray(path)
            if ids.ndim != 1 or len(ids) < 3 or ids.dtype.kind not in 'iu' or len(set(ids.tolist())) != len(ids) or np.any(ids < 0) or np.any(ids >= len(points)):
                raise ValueError('Invalid whole-guide vertex ownership')
        self.paths = {r: [int(i) for i in p] for r, p in paths.items()}
        if set(paths) != set(lengths_mm):
            raise ValueError('Whole-path length reference mismatch')
        if reference is None:
            weights = {r: 1. for r in paths} if role_weights is None else dict(role_weights)
            if set(weights) != set(paths) or any(not np.isfinite(w) or w <= 0 for w in weights.values()):
                raise ValueError('Positive weights must cover every whole guide')
            terms = {}
            for role, path in paths.items():
                L = float(lengths_mm[role])/1000
                _, _, detail = forehead_polyline_fairing(np.asarray(points)[path], L, 1.)
                terms[role] = {'length_m': L, 'bending': max(detail['dimensionless_bending'], 1e-12),
                               'variation': max(detail['dimensionless_variation'], 1e-12), 'weight': weights[role]}
            reference = {'version': 1, 'paths': self.paths, 'terms': terms}
        self.reference = forehead_fairing_copy.deepcopy(reference)
        if self.reference.get('version') != 1 or self.reference.get('paths') != self.paths:
            raise ValueError('Fairing reference requires explicit topology migration')
        terms = self.reference['terms']
        if set(terms) != set(paths):
            raise ValueError('Missing persistent fairing role')
        for role, row in terms.items():
            if any(not np.isfinite(row[k]) or row[k] <= 0 for k in ('length_m', 'bending', 'variation', 'weight')):
                raise ValueError('Invalid persistent fairing coefficients')
            if row['length_m'] != float(lengths_mm[role])/1000:
                raise ValueError('Frozen whole-path length changed')
            if role_weights is not None and role_weights.get(role) != row['weight']:
                raise ValueError('Cannot silently change persistent role weights')
        self.triples = np.asarray([t for p in paths.values() for t in zip(p, p[1:], p[2:])], int).reshape(-1, 3)
        self.initial_energy = self.energy_gradient(points)[0]
        self.target_energy = (1-reduction)*self.initial_energy
        # Exact nonregression by default; any progression is explicit and bounded at zero.
        raw = orbital_turns(points, self.triples)
        self.turn_caps = raw.copy()
        self.turn_roles = sorted(paths if turn_roles is None else turn_roles)
        if not set(self.turn_roles) <= set(paths):
            raise ValueError('Invalid raw-turn progression roles')
        goals = {r: 0. for r in paths} if turn_goals is None else dict(turn_goals)
        if set(goals) != set(paths) or any(not np.isfinite(g) or g < 0 for g in goals.values()):
            raise ValueError('Invalid explicit local-turn goals')
        offset = 0
        for role, path in paths.items():
            n = len(path)-2
            if role in self.turn_roles:
                self.turn_caps[offset:offset+n] = np.minimum(raw[offset:offset+n], np.maximum(goals[role], raw[offset:offset+n]-turn_reduction_degrees))
            offset += n
        self.initial_metrics = self.role_metrics(points)
        self.count = len(self.triples)+1
        self.nodes = sorted({i for p in paths.values() for i in p})
        self.reduction = reduction

    def energy_gradient(self, points):
        p = np.asarray(points, float); gradient = np.zeros_like(p); energy = 0.
        total = sum(row['weight'] for row in self.reference['terms'].values())
        for role, path in self.paths.items():
            row = self.reference['terms'][role]
            value, g, _ = forehead_polyline_fairing(p[path], row['length_m'], row['bending']/row['variation'])
            factor = row['weight']/row['bending']/total
            energy += factor*value
            np.add.at(gradient, path, factor*g)
        return float(energy), gradient

    def constraints(self, points):
        return np.r_[self.turn_caps-orbital_turns(np.asarray(points, float), self.triples), self.target_energy-self.energy_gradient(points)[0]]

    def accepts(self, points):
        return bool(np.all(self.constraints(points) >= 0))

    def energy_parameter_gradient(self, values, evaluate, batch, ids):
        state = evaluate(values)
        differential = batch.sample_differential(values.reshape(-1, 2)/1000)
        if state is None or differential is None or not np.array_equal(differential[0], state[0][ids]):
            raise ValueError('Fairing primal/differential mismatch')
        _, world = self.energy_gradient(state[0])
        result, flags = forehead_fairing_parameter_gradient(world, ids, differential[1]/1000, differential[2])
        flags &= np.repeat(np.any(world[ids] != 0, axis=1), 2)
        for col in np.flatnonzero(flags):
            plus, minus = values.copy(), values.copy()
            plus[col] += 1e-4; minus[col] -= 1e-4
            a, b = evaluate(plus), evaluate(minus)
            if a is None or b is None:
                raise ValueError('No valid central fairing donor-boundary derivative')
            result[col] = (self.energy_gradient(a[0])[0]-self.energy_gradient(b[0])[0])/2e-4
        return result

    def quantization_targets(self, points, offset, scales):
        smooth = self.constraints(points)
        saved = self.constraints(np.asarray(points).astype(np.float32).astype(float))
        targets = np.zeros(len(scales)); failed = saved < 0
        safety = np.r_[np.full(len(self.triples), .001), max(self.initial_energy*1e-7, 1e-12)]
        targets[offset:offset+self.count][failed] = (np.maximum(smooth[failed]-saved[failed], 0)+safety[failed])/scales[offset:offset+self.count][failed]
        return targets

    def role_metrics(self, points):
        result = {}
        for role, path in self.paths.items():
            row = self.reference['terms'][role]
            _, _, detail = forehead_polyline_fairing(np.asarray(points)[path], row['length_m'], 1.)
            triples = np.asarray(list(zip(path, path[1:], path[2:])), int).reshape(-1, 3)
            result[role] = {'local_peak_degrees': float(max(orbital_turns(np.asarray(points, float), triples), default=0.)),
                            'bending': detail['dimensionless_bending'], 'variation': detail['dimensionless_variation']}
        return result

    def evidence(self, points):
        return {'initial_energy': self.initial_energy, 'final_energy': self.energy_gradient(points)[0],
                'target_energy': self.target_energy, 'requested_reduction': self.reduction,
                'raw_turn_caps_degrees': self.turn_caps.tolist(),
                'persistent_reference': forehead_fairing_copy.deepcopy(self.reference),
                'per_role_before': self.initial_metrics, 'per_role_after': self.role_metrics(points),
                'progression_roles': self.turn_roles, 'full_model_acceptance': False}


def forehead_current_checkpoint(best, values):
    """Historical best availability is not admission of the current iterate."""
    return bool(best and 'values' in best and np.array_equal(np.asarray(best['values']), values))

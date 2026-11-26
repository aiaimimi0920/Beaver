"""Bounded supplemental whole-producer-guide and incident-support fairing.

This never replaces original shape, field, atlas, pose or native gates. References
belong to the saved stage input; topology changes require explicit migration.
"""
import copy as forehead_complete_copy
import hashlib as forehead_complete_hashlib
import json as forehead_complete_json
import numpy as np
from forehead_quantization_targets import forehead_complete_margin_targets
from forehead_complete_curve_metrics import forehead_complete_side_curve
from forehead_finite_rail_gap import forehead_finite_rail_profile


def forehead_complete_guide_identity(proposal):
    data = {'count': len(proposal['vertices']), 'paths': proposal['paths'], 'faces': proposal['faces'],
            'support': proposal['support'], 'protected': proposal['protected']}
    return forehead_complete_hashlib.sha256(forehead_complete_json.dumps(data, sort_keys=True, default=lambda x: np.asarray(x).tolist()).encode()).hexdigest()


def forehead_complete_guide_ids(values, count):
    if not isinstance(values, (list, tuple, np.ndarray)) or np.ndim(values) != 1:
        raise ValueError('Invalid complete-guide integer IDs')
    if any(isinstance(v, (bool, np.bool_)) or not isinstance(v, (int, np.integer))
           or not 0 <= v < count for v in values):
        raise ValueError('Invalid complete-guide integer IDs')
    return [int(v) for v in values]


class ForeheadCompleteGuideFairingStage:
    def __init__(self, proposal, points, source_identity, config=None):
        retained = proposal.get('complete_guide_fairing_guard')
        advance = config is not None
        config = forehead_complete_copy.deepcopy(config if config is not None else retained['config'])
        if type(config.get('version')) is not int or config.get('version') != 1 or type(config.get('max_evaluations')) is not int or not 1 <= config['max_evaluations'] <= 10000:
            raise ValueError('Explicit finite complete-guide evaluation budget required')
        reduction = config.get('reduction', 0.)
        if not np.isfinite(reduction) or not 0 < reduction < 1:
            raise ValueError('Explicit complete-guide reduction must be between zero and one')
        if not isinstance(source_identity, str) or not source_identity:
            raise ValueError('Immutable producer source identity required')
        self.source_identity = source_identity
        self.config = config
        self.calls = {}
        self.total_calls = 0
        self.identity = forehead_complete_guide_identity(proposal)
        self.paths = {k: forehead_complete_guide_ids(v, len(points)) for k, v in proposal['paths'].items()}
        if set(self.paths) != {'red', 'orange', 'green', 'blue', 'purple'}:
            raise ValueError('All five complete producer roles required')
        self.paths = {k: self.paths[k] for k in ('red', 'orange', 'green', 'blue', 'purple')}
        self.points_count = len(points)
        self.faces = [forehead_complete_guide_ids(q, len(points)) for q in proposal['faces']]
        if any(len(q) < 3 or len(set(q)) != len(q) for q in self.faces):
            raise ValueError('Invalid complete-guide source face')
        edges = {frozenset((a, b)) for q in self.faces for a, b in zip(q, q[1:]+q[:1])}
        protected = set(forehead_complete_guide_ids(proposal['protected'], len(points)))
        for path in self.paths.values():
            if len(path) < 4 or len(set(path)) != len(path) or min(path) < 0 or max(path) >= len(points):
                raise ValueError('Invalid complete producer path')
            if not {path[0], path[-1]} <= protected or any(frozenset(e) not in edges for e in zip(path, path[1:])):
                raise ValueError('Unprotected endpoint or non-source guide edge')
        self.frames = {k: np.asarray(v, float) for k, v in config['frames'].items()}
        if set(self.frames) != {'front', 'right45', 'right90'}:
            raise ValueError('All three explicit camera frames required')
        self.frames = {k: self.frames[k] for k in ('front', 'right45', 'right90')}
        if any(v.shape != (2, 3) for v in self.frames.values()):
            raise ValueError('Invalid camera frame')
        owners = set(forehead_complete_guide_ids(proposal['support'], len(points))) | {i for p in self.paths.values() for i in p}
        self.quads = np.array([q for q in self.faces if len(q) == 4 and owners.intersection(q)], int)
        if not len(self.quads):
            raise ValueError('Missing incident support quads')
        self.nodes = sorted(owners | set(self.quads.ravel()))
        self.charge('initial_saved_reference')
        initial = self.measure(np.asarray(points).astype(np.float32).astype(float))
        if retained is not None:
            if type(retained.get('version')) is not int or retained['version'] != 1:
                raise ValueError('Unknown complete-guide guard version')
            if retained['identity'] != self.identity or retained['source_identity'] != source_identity or retained['config'] != config:
                raise ValueError('Complete-guide reference needs explicit migration')
            self.reference = forehead_complete_copy.deepcopy(retained['reference'])
            self.caps = np.asarray(retained['caps'], float)
            self.target = float(retained['energy_cap'])
            if advance:
                self.target = min(self.target, (1-reduction)*self.energy_from(initial))
        else:
            self.reference = initial.copy().tolist()
            self.caps = initial.copy()
            self.target = (1-reduction)*self.energy_from(initial)
        if self.caps.shape != initial.shape or np.asarray(self.reference).shape != initial.shape or not np.isfinite(self.reference).all() or not np.isfinite(self.caps).all() or not np.isfinite(self.target):
            raise ValueError('Invalid retained complete-guide guard')
        self.count = len(initial)+1

    def charge(self, kind):
        if self.total_calls >= self.config['max_evaluations']:
            raise ValueError('Complete-guide shared evaluation budget exhausted')
        self.total_calls += 1
        self.calls[kind] = self.calls.get(kind, 0)+1

    def measure(self, points):
        p = np.asarray(points, float)
        if p.shape != (self.points_count, 3) or not np.isfinite(p).all():
            raise ValueError('Invalid complete-guide geometry')
        out = []
        for path in self.paths.values():
            q = p[path]
            e = np.diff(q, axis=0)
            lengths = np.linalg.norm(e, axis=1)
            if np.min(lengths) <= 0:
                raise ValueError('Collapsed complete-guide station')
            tangent = e/lengths[:, None]
            dual = (lengths[:-1]+lengths[1:])/2
            curvature = np.diff(tangent, axis=0)/dual[:, None]
            variation = np.sum(np.linalg.norm(np.diff(curvature, axis=0), axis=1)**2/((dual[:-1]+dual[1:])/2))*lengths.sum()**3
            peak = np.degrees(np.arccos(np.clip(np.sum(tangent[:-1]*tangent[1:], axis=1), -1, 1))).max()
            out.extend([variation, peak, np.std(lengths)/np.mean(lengths), -lengths.min()*1000, lengths.max()*1000])
            for frame in self.frames.values():
                row = forehead_complete_side_curve(q, frame[0], frame[1])
                out.extend([row['curvature_variation_energy'], max(abs(v) for v in row['signed_turn_degrees'])])
        gap = forehead_finite_rail_profile(p[self.paths['blue']], p[self.paths['purple']])
        out.extend([-gap['minimum_finite_gap_m']*1000, gap['length_weighted_gap_CV']])
        quad = p[self.quads]
        edge = np.linalg.norm(np.roll(quad, -1, axis=1)-quad, axis=2)
        if np.min(edge) <= 0:
            raise ValueError('Collapsed support face')
        cv = np.std(edge, axis=1)/np.mean(edge, axis=1)
        out.extend([cv.max(), cv.mean(), -edge.min()*1000, edge.max()*1000])
        return np.array(out)

    def energy_from(self, measured):
        reference = np.asarray(self.reference)
        indices = [11*r+i for r in range(5) for i in (0, 5, 7, 9)]
        return float(np.sum(measured[indices]/np.maximum(reference[indices], 1e-12))+measured[-5]+measured[-3])

    def objective(self, points):
        self.charge('objective')
        return self.energy_from(self.measure(points))

    def constraints(self, points):
        self.charge('constraints')
        measured = self.measure(points)
        return np.r_[(self.caps-measured)/np.maximum(np.abs(self.reference), 1.), self.target-self.energy_from(measured)]

    def accepts(self, points):
        self.charge('saved_or_final_acceptance')
        return bool(np.all(self.constraints(points) >= 0))

    def quantization_targets(self, points, offset, scales):
        """Count both original residual measurements; retain every hard cap."""
        self.charge('quantization_target_request')
        smooth = self.constraints(points)
        saved = self.constraints(np.asarray(points).astype(np.float32).astype(float))
        return forehead_complete_margin_targets(smooth, saved, offset, scales)

    def parameter_gradient(self, values, evaluate):
        self.charge('objective_derivative')
        result = np.empty(len(values))
        for col in range(len(values)):
            plus, minus = values.copy(), values.copy()
            plus[col] += 1e-4
            minus[col] -= 1e-4
            a, b = evaluate(plus), evaluate(minus)
            if a is None or b is None:
                raise ValueError('Complete-guide derivative leaves original source domain')
            result[col] = (self.objective(a[0])-self.objective(b[0]))/2e-4
        return result

    def capture(self, proposal, points):
        points = np.asarray(points).astype(np.float32).astype(float)
        if forehead_complete_guide_identity(proposal) != self.identity or not self.accepts(points):
            raise ValueError('Cannot capture unadmitted complete-guide geometry')
        measured = self.measure(points)
        return {'version': 1, 'identity': self.identity, 'source_identity': self.source_identity, 'config': forehead_complete_copy.deepcopy(self.config),
                'reference': forehead_complete_copy.deepcopy(self.reference), 'caps': np.minimum(self.caps, measured).tolist(),
                'energy_cap': min(self.target, self.energy_from(measured))}

    def evidence(self):
        return {'counted_evaluations': self.total_calls, 'evaluation_kinds': dict(self.calls),
                'evaluation_limit': self.config['max_evaluations'], 'source_scope': 'complete producer-owned paths and incident support quads',
                'opposite_side_native_proof_required': True, 'original_gates_retained': True,
                'dense_shape_and_native_fields_required_separately': True, 'visual_acceptance': False}


def forehead_prepare_complete_guide_stage(proposal, points, source_identity, config=None):
    guard = proposal.get('complete_guide_fairing_guard')
    reference = proposal.get('complete_guide_fairing_reference')
    if reference is not None and (guard is None or guard.get('reference') != reference):
        raise ValueError('Complete-guide reference lost its retained guard')
    if config is None and guard is None:
        return None
    return ForeheadCompleteGuideFairingStage(proposal, points, source_identity, config)


def forehead_complete_guide_guard_retains(before, after):
    old = before.get('complete_guide_fairing_guard')
    if old is None:
        return
    new = after.get('complete_guide_fairing_guard')
    if new is None:
        raise ValueError('Complete-guide guard dropped by later stage')
    if before.get('complete_guide_fairing_reference') != after.get('complete_guide_fairing_reference'):
        raise ValueError('Complete-guide immutable reference dropped or changed')
    for key in ('version', 'identity', 'source_identity', 'config', 'reference'):
        if old[key] != new[key]:
            raise ValueError('Complete-guide guard needs explicit source/topology migration')
    if np.any(np.asarray(new['caps']) > np.asarray(old['caps'])) or new['energy_cap'] > old['energy_cap']:
        raise ValueError('Complete-guide guard relaxed by later stage')
    stage = forehead_prepare_complete_guide_stage(after, after['vertices'], old['source_identity'])
    if not stage.accepts(np.asarray(after['vertices']).astype(np.float32).astype(float)):
        raise ValueError('Later saved geometry violates complete-guide guard')

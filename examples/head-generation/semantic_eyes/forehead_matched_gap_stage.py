"""Fixed-source shared domains with normal-like bidirectional guide distances.
This fitting diagnostic retains unmatched tails explicitly in its independent atlas.
"""
from forehead_author_atlas import ForeheadFrozenAuthorAtlas
from forehead_author_gap_samples import ForeheadFrozenGapResidual
from forehead_matched_gap_validation import forehead_matched_gap_dense_errors
import copy as forehead_gap_copy
import numpy as np
from forehead_gap_measure import closest_forehead_polyline

def make_forehead_matched_gap_stage(*args, **kwargs):
    """Ordered-package boundary and compatibility construction entry."""
    return ForeheadMatchedGapStage(*args, **kwargs)


class ForeheadMatchedGapStage:

    def __init__(self, author, proposal, params, active_pair=None, reduction=0.025, count=33, atlas=None):
        self.atlas = ForeheadFrozenAuthorAtlas(author) if atlas is None else atlas
        if not np.array_equal(self.atlas.old, np.asarray(author['vertices'])):
            raise ValueError('Atlas author ownership differs')
        self.fast = ForeheadFrozenGapResidual(self.atlas, proposal, count)
        self.paths = proposal['paths']
        self.protected = set(proposal['protected'])
        self.count = count
        self.pairs = [a + '-' + b for a, b in zip(self.fast.names, self.fast.names[1:])]
        self.targets = {a + '-' + b: float(np.linalg.norm(self.atlas.old[self.paths[a][0]] - self.atlas.old[self.paths[b][0]]) * 1000) for a, b in zip(self.fast.names, self.fast.names[1:])}
        self.groups = []
        self.labels = []
        offset = 0
        for a, b, kind, o, d in self.fast.specs:
            for source, target in ((a, b), (b, a)):
                self.groups.append((offset, offset + count, list({i for path in self.paths.values() for i in path})))
                self.labels.extend([a + '-' + b + ':' + kind] * count)
                offset += count
        for a, b, kind, o, d in self.fast.specs:
            for source, target in ((a, b), (b, a)):
                size = len(self.paths[source])
                self.groups.append((offset, offset + size, list({i for path in self.paths.values() for i in path})))
                self.labels.extend([a + '-' + b + ':' + kind] * size)
                offset += size
        self.verify_fast = ForeheadFrozenGapResidual(self.atlas, proposal, 1025)
        self.domains = list(dict.fromkeys(self.labels))
        self.size = offset
        self.target = np.array([self.targets[n.split(':')[0]] for n in self.labels])
        initial = self.measure(np.asarray(proposal['vertices']), params)
        if initial is None:
            raise ValueError('Initial frozen shared-domain atlas is invalid')
        errors = np.abs(initial / self.target - 1)
        caps = forehead_gap_copy.deepcopy(proposal.get('matched_gap_caps_v2'))
        self.newly_covered_domains = []
        if caps is None:
            legacy = proposal.get('matched_gap_caps', {})
            caps = {}
            for domain in self.domains:
                pair = domain.split(':')[0]
                if pair in legacy and domain != 'green-blue:lower':
                    caps[domain] = legacy[pair]
                else:
                    caps[domain] = max(0.099, float(errors[np.array(self.labels) == domain].max()) + 0.001)
                    if domain == 'green-blue:lower':
                        self.newly_covered_domains.append(domain)
        if set(caps) != set(self.domains):
            raise ValueError('Gap domain ownership changed without explicit migration')
        active = [] if reduction == 0 else self.domains if active_pair is None else [n for n in self.domains if n == active_pair or n.split(':')[0] == active_pair]
        for n in active:
            caps[n] = max(0.099, min(caps[n], float(errors[np.array(self.labels) == n].max()) + 0.001) - reduction)
        self.caps = caps
        self.cap = np.array([caps[n] for n in self.labels])
        self.active = active

    def measure(self, points, params):
        result = self.fast.evaluate(points, params, details=True)
        if not result['valid']:
            return None
        rows = []
        for detail in result['details']:
            a, b = (detail['first'], detail['second'])
            wa, wb = detail['world_hits']
            rows.extend((closest_forehead_polyline(wa, points[self.paths[b]])[0] * 1000, closest_forehead_polyline(wb, points[self.paths[a]])[0] * 1000))
        for row in self.vertex_records(points, params, result['details']):
            if row['active']:
                value = closest_forehead_polyline(points[[row['source_id']]], points[self.paths[row['target']]])[0][0] * 1000
            else:
                value = row['target_mm']
            rows.append(np.array([value]))
        return np.concatenate(rows)

    def vertex_records(self, points, params, details=None):
        if details is None:
            result = self.fast.evaluate(points, params, details=True)
            if not result['valid']:
                raise ValueError(result['reason'])
            details = result['details']
        records = []
        for detail in details:
            a, b = (detail['first'], detail['second'])
            pair = a + '-' + b
            for side, (source, target) in enumerate(((a, b), (b, a))):
                locations = detail['curve_parameters'][side]
                lo = float(locations.min())
                hi = float(locations.max())
                for index, node in enumerate(self.paths[source]):
                    records.append({'source_id': node, 'target': target, 'target_mm': self.targets[pair], 'active': lo - 1e-09 <= index <= hi + 1e-09, 'domain_boundary': node not in self.protected and min(abs(index - lo), abs(index - hi)) < 1e-07})
        return records

    def constraints(self, points, params):
        gaps = self.measure(points, params)
        if gaps is None:
            return np.full(self.size, -1000.0)
        return self.cap - 0.002 - np.abs(gaps / self.target - 1)

    def accepts(self, points, params):
        errors = forehead_matched_gap_dense_errors(self, points, params)
        return errors is not None and all((errors[n] <= self.caps[n] + 1e-09 for n in self.domains))

    def evidence(self):
        return {'targets_mm': self.targets, 'domain_version': 2, 'newly_covered_domains': self.newly_covered_domains, 'legacy_existing_domains_keep_their_caps': True, 'working_maximum_relative_deviation': self.caps, 'tightened_pairs': self.active, 'fitting_samples_per_chart_direction': self.count, 'fitting_relative_margin': 0.002, 'saved_admission_sections_per_chart_direction': 1025, 'all_shared_domain_guide_vertex_extrema': True, 'source_vertices_explicit_fitting_rows': True, 'metric': 'bidirectional closest whole-guide distances at fixed-source shared-domain intersections', 'unmatched_tails': 'Always separately reported by independent atlas; never nearest-distance-remasked', 'scope': 'Fitting-only diagnostic. Dense extrema, source-sheet connectivity, shape, native model and visuals remain independent.'}

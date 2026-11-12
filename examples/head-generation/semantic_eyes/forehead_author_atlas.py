"""Read-only frozen-author cross-section measurement library. No fitting or edits.
The polar transition is injective for positive radius; source-face connectivity
is checked separately. Complete curve arc partition is not a uniformity claim.
"""
import numpy as np
from collections import defaultdict, deque
from forehead_flow_bind import bind_forehead_paths
from forehead_flow_rule import forehead_chart

def forehead_atlas_cross(a, b):
    return a[..., 0] * b[..., 1] - a[..., 1] * b[..., 0]

def forehead_atlas_intersections(xy, xyz, origin, direction):
    edge = np.diff(xy, axis=0)
    den = forehead_atlas_cross(edge, direction)
    num = forehead_atlas_cross(origin - xy[:-1], direction)
    ts = np.divide(num, den, out=np.full(len(den), np.nan), where=abs(den) > 1e-14)
    hits = []
    for k, t in enumerate(ts):
        if np.isfinite(t) and -1e-09 <= t <= 1 + 1e-09:
            s = k + float(np.clip(t, 0, 1))
            pos = xyz[k] + float(np.clip(t, 0, 1)) * (xyz[k + 1] - xyz[k])
            q = xy[k] + float(np.clip(t, 0, 1)) * edge[k]
            if not hits or abs(s - hits[-1]['s']) > 1e-07:
                hits.append({'s': s, 'point': pos, 'xy': q})
    return hits

def forehead_atlas_at(array, s):
    k = min(int(s), len(array) - 2)
    t = s - k
    return array[k] * (1 - t) + array[k + 1] * t

def forehead_atlas_sub(array, a, b):
    return np.asarray([forehead_atlas_at(array, a)] + [array[k] for k in range(int(np.floor(a)) + 1, int(np.ceil(b)))] + [forehead_atlas_at(array, b)])

def forehead_atlas_event(xy, xyz, entry, exit_h):
    en = forehead_atlas_intersections(xy, xyz, np.array([entry, 0]), np.array([0.0, 1.0]))
    en = [z for z in en if xy[min(int(z['s']) + 1, len(xy) - 1), 0] >= xy[int(z['s']), 0]]
    ex = forehead_atlas_intersections(xy, xyz, np.array([0, exit_h]), np.array([1.0, 0.0]))
    ex = [z for z in ex if xy[min(int(z['s']) + 1, len(xy) - 1), 1] < xy[int(z['s']), 1]]
    if len(en) != 1 or len(ex) != 1 or en[0]['s'] >= ex[0]['s']:
        return (None, {'entry_crossings': len(en), 'descending_exit_crossings': len(ex), 'entry_s': [z['s'] for z in en], 'exit_s': [z['s'] for z in ex]})
    return ((en[0]['s'], ex[0]['s']), None)

def forehead_atlas_summary(values):
    a = np.asarray(values)
    return {'min_mm': float(a.min()), 'max_mm': float(a.max()), 'mean_mm': float(a.mean()), 'CV': float(a.std() / a.mean())} if len(a) else None

class ForeheadFrozenAuthorAtlas:

    def __init__(self, author, reference_paths=None):
        r = author
        old = np.asarray(r['vertices'])
        sc = forehead_chart(old)
        refs = bind_forehead_paths(old, r['faces'], r['regions'], r['report'])['positive_paths'] if reference_paths is None else {name: list(path) for name, path in reference_paths.items()}
        names = ('blue', 'purple')
        peaks = {}
        for n in names:
            path = refs[n]
            x = sc[path, 0]
            diff = np.diff(x)
            k = next((k for k in range(1, len(x) - 1) if diff[k - 1] > 1e-08 and diff[k] < -1e-08))
            peaks[n] = {'vertex': path[k], 'index': k, 'u': float(x[k]), 'height': float(old[path[k], 2])}
        edge = tuple(sorted((z['vertex'] for z in peaks.values())))
        incident = [i for i, q in enumerate(r['faces']) if any((tuple(sorted(e)) == edge for e in zip(q, q[1:] + q[:1])))]
        assert len(incident) == 2
        entry = 0.5 * min((z['u'] for z in peaks.values()))
        hpeak = np.mean([z['height'] for z in peaks.values()])
        semantic = np.asarray(r['report']['semantic_rows']) + r['report']['head_assembly_y_offset_m']
        source_row = int(np.argmin(abs(semantic - hpeak)))
        assert source_row > 0
        exit_h = float(semantic[source_row - 1])
        refparts = {}
        refevents = {}
        for n in names:
            xy = sc[refs[n]][:, [0, 2]]
            xyz = old[refs[n]]
            ev, err = forehead_atlas_event(xy, xyz, entry, exit_h)
            assert err is None
            refevents[n] = ev
            refparts[n] = forehead_atlas_sub(xy, *ev)

        def arc_sample(xy, t):
            ar = np.r_[0, np.cumsum(np.linalg.norm(np.diff(xy, axis=0), axis=1))]
            return np.column_stack([np.interp(t * ar[-1], ar, xy[:, k]) for k in range(2)])
        t = np.linspace(0, 1, 129)
        pivot = np.array([entry, exit_h])
        angle = (1 - t) * np.pi / 2
        left = np.tile(pivot, (len(t), 1))
        right = left + np.column_stack((np.cos(angle), np.sin(angle)))
        rulings = [(a, b - a) for a, b in zip(left, right)]
        crossings = []
        meta = {'author_transition': peaks, 'shared_original_edge': edge, 'shared_original_faces': incident, 'fixed_upper_core_u_end_mm': entry * 1000, 'fixed_lower_chart_start_height_mm': exit_h * 1000, 'transition_rule': 'noncrossing radial rays in author chart from the intersection of fixed u-entry and source-row exit; angles pi/2 down to0. Positive-radius sector has Jacobian r>0. No current errors used', 'transition_reference_ruling_crossings': crossings, 'scope': 'Author-owned transition frame; complete five-guide shared domains are measured separately. No geodesic or model acceptance claim.'}
        self.author = r
        self.sc = sc
        self.old = old
        self.refs = refs
        self.names = names
        self.entry = entry
        self.exit_h = exit_h
        self.left = left
        self.right = right
        self.rulings = rulings
        self.evidence = meta

    def check_source_connectivity(self, proposal, measurement):
        """Prove each sampled section connects across shared ORIGINAL donor edges.
  Does not accept projected overlaps as anatomical continuity. Returns holes,
  ambiguous endpoint interiors and disconnected sheet warnings explicitly.
  """
        donors = np.asarray(proposal['source_faces'], int)
        xy = self.sc[donors][:, :, [0, 2]]
        edges = np.roll(xy, -1, axis=1) - xy
        adj = defaultdict(set)
        inc = defaultdict(list)
        for i, q in enumerate(donors):
            for a, b in zip(q, np.roll(q, -1)):
                inc[tuple(sorted((int(a), int(b))))].append(i)
        for fs in inc.values():
            for i in fs:
                adj[i].update(set(fs) - {i})
        result = {}
        for name, chart in measurement.get('charts', {}).items():
            failures = []
            ambiguous = []
            passed = 0
            for row in chart['section_records']:
                a, b = np.asarray(row['source_chart_endpoints'])
                direction = b - a
                A = forehead_atlas_cross(edges, a - xy)
                B = forehead_atlas_cross(edges, direction)
                lo = np.zeros(len(xy))
                hi = np.ones(len(xy))
                valid = np.ones(len(xy), bool)
                for k in range(4):
                    pos = B[:, k] > 1e-14
                    neg = B[:, k] < -1e-14
                    flat = ~(pos | neg)
                    valid &= ~(flat & (A[:, k] < -1e-12))
                    lo[pos] = np.maximum(lo[pos], -A[pos, k] / B[pos, k])
                    hi[neg] = np.minimum(hi[neg], -A[neg, k] / B[neg, k])
                valid &= lo <= hi + 1e-09
                starts = np.flatnonzero(valid & (lo <= 1e-09))
                ends = set(np.flatnonzero(valid & (hi >= 1 - 1e-09)).tolist())
                inner_start = np.sum(np.all(A > 1e-12, axis=1))
                inner_end = np.sum(np.all(A + B > 1e-12, axis=1))
                if inner_start > 1 or inner_end > 1:
                    ambiguous.append({'section': row['section'], 'endpoint_interior_counts': [int(inner_start), int(inner_end)]})
                visited = set(starts.tolist())
                queue = deque(visited)
                while queue:
                    i = queue.popleft()
                    for j in adj[i] - visited:
                        if valid[j] and max(lo[i], lo[j]) <= min(hi[i], hi[j]) + 1e-09:
                            visited.add(j)
                            queue.append(j)
                if visited & ends:
                    passed += 1
                else:
                    failures.append({'section': row['section'], 'start_candidates': len(starts), 'end_candidates': len(ends), 'visited_faces': len(visited)})
            result[name] = {'connected_sections': passed, 'tested': len(chart['section_records']), 'disconnected_sections': failures, 'ambiguous_endpoint_sections': ambiguous, 'all_tested_sections_connected_unambiguous': not failures and (not ambiguous)}
        return result

    def measure_all(self, proposal, count=129, terminal_ids=None):
        """Eight directed, fixed-source pair records with raw gaps and honest tails.
  First call freezes terminal IDs; later calls cannot silently replace roles.
  Radial domains stop at max(author endpoint angles,0). Only blue-purple also
  uses the lower height chart. Role-unmatched remainders are always reported.
  """
        if not isinstance(count, int) or count < 3:
            raise ValueError('Need at least three fixed sections')
        names = ('red', 'orange', 'green', 'blue', 'purple')
        paths = proposal['paths']
        xyz = np.asarray(proposal['vertices'])
        roles = {n: int(paths[n][-1]) for n in names} if terminal_ids is None else dict(terminal_ids)
        if set(roles) != set(names) or any((not 0 <= i < len(self.old) for i in roles.values())):
            raise ValueError('Terminal roles require original author vertices')
        if hasattr(self, 'terminal_roles') and self.terminal_roles != roles:
            raise ValueError('Frozen terminal ownership changed')
        self.terminal_roles = roles.copy()
        if any((roles[n] not in set(proposal['protected']) or paths[n][-1] != roles[n] or (not np.array_equal(xyz[roles[n]], self.old[roles[n]])) for n in names)):
            raise ValueError('Current terminal differs from frozen author role')
        params = self.sc.tolist() + [None] * (len(xyz) - len(self.sc))
        for k, z in proposal.get('lineage', {}).items():
            params[int(k)] = (np.asarray(z['weights']) @ self.sc[z['donors']]).tolist()
        params = np.asarray(params)
        pivot = np.array([self.entry, self.exit_h])
        curves = {}
        issues = []
        terminal_angles = {n: float(np.arctan2(self.sc[i, 2] - self.exit_h, self.sc[i, 0] - self.entry)) for n, i in roles.items()}

        def partition(coords, points, path):
            xy = coords[path][:, [0, 2]]
            world = points[path]
            hits = forehead_atlas_intersections(xy, world, np.array([self.entry, 0]), np.array([0.0, 1.0]))
            hits = [z for z in hits if xy[min(int(z['s']) + 1, len(xy) - 1), 0] > xy[int(z['s']), 0]]
            if len(hits) != 1:
                return {'error': 'nonunique fixed angular entry', 'count': len(hits)}
            en = hits[0]['s']
            end = len(path) - 1
            if xy[-1, 1] < self.exit_h - 1e-10:
                hh = forehead_atlas_intersections(xy, world, np.array([0, self.exit_h]), np.array([1.0, 0.0]))
                hh = [z for z in hh if z['s'] > en and xy[min(int(z['s']) + 1, len(xy) - 1), 1] < xy[int(z['s']), 1]]
                if len(hh) != 1:
                    return {'error': 'nonunique fixed height exit', 'count': len(hh)}
                end = hh[0]['s']
            core = forehead_atlas_sub(xy, 0, en)
            rad = forehead_atlas_sub(xy, en, end)
            dc = np.diff(core, axis=0)
            dr = np.diff(rad, axis=0)
            radius = np.linalg.norm(rad - pivot, axis=1)
            signed = -forehead_atlas_cross(rad[:-1] - pivot, dr)
            certificate = {'upper_strict': bool(np.all(dc[:, 0] > 1e-14)), 'radial_strict': bool(np.all(signed > 1e-14)), 'radial_nonpositive_segments': np.flatnonzero(signed <= 1e-14).tolist(), 'minimum_radius_m': float(radius.min()), 'within_positive_quadrant': bool(np.all(rad >= pivot - 1e-10))}
            return {'xy': xy, 'world': world, 'entry': en, 'radial_end': end, 'core': (core, forehead_atlas_sub(world, 0, en)), 'radial': (rad, forehead_atlas_sub(world, en, end)), 'certificate': certificate}
        for n in names:
            curves[n] = partition(params, xyz, paths[n])
            if 'error' in curves[n]:
                issues.append({'guide': n, **curves[n]})
        reference = {n: partition(self.sc, self.old, self.refs[n]) for n in names}
        reference_audit = {n: {k: z for k, z in reference[n].items() if k in ('error', 'count')} if 'error' in reference[n] else reference[n]['certificate'] for n in names}
        out = {'samples_per_chart': count, 'frozen_terminal_roles': roles, 'frozen_terminal_angles_radians': terminal_angles, 'original_author_binding_endpoints': {n: self.refs[n][-1] for n in names}, 'author_binding_order_audit': reference_audit, 'current_order_audit': {n: z.get('certificate') for n, z in curves.items()}, 'partition_errors': issues, 'pairs': [], 'coverage_scope': 'Each guide remainder is explicit per neighbor. Unmatched does not mean uniform or approved. All-five acceptance is not implied.'}
        if issues:
            return out

        def arc(world, a, b):
            return float(np.linalg.norm(np.diff(forehead_atlas_sub(world, a, b), axis=0), axis=1).sum() * 1000) if b > a + 1e-10 else 0.0

        def sections(first, second, kind, levels):
            record = {'chart': kind, 'levels': list(map(float, levels)), 'gaps_mm': [], 'signed_chart_separation': [], 'intersection_counts': [], 'section_records': []}
            for j, value in enumerate(levels):
                if kind == 'upper':
                    origin = np.array([value, 0.0])
                    direction = np.array([0.0, 1.0])
                    parts = {n: curves[n]['core'] for n in (first, second)}
                elif kind == 'radial':
                    origin = pivot
                    direction = np.array([np.cos(value), np.sin(value)])
                    parts = {n: curves[n]['radial'] for n in (first, second)}
                else:
                    origin = np.array([0.0, value])
                    direction = np.array([1.0, 0.0])
                    parts = {n: (forehead_atlas_sub(curves[n]['xy'], curves[n]['radial_end'], len(paths[n]) - 1), forehead_atlas_sub(curves[n]['world'], curves[n]['radial_end'], len(paths[n]) - 1)) for n in (first, second)}
                hits = {n: forehead_atlas_intersections(*parts[n], origin, direction) for n in (first, second)}
                counts = {n: len(hits[n]) for n in (first, second)}
                record['intersection_counts'].append(counts)
                if any((c != 1 for c in counts.values())):
                    record['gaps_mm'].append(None)
                    record['signed_chart_separation'].append(None)
                    continue
                aa, bb = [hits[n][0] for n in (first, second)]
                record['gaps_mm'].append(float(np.linalg.norm(aa['point'] - bb['point']) * 1000))
                record['signed_chart_separation'].append(float(np.dot(aa['xy'] - bb['xy'], direction)))
                record['section_records'].append({'section': j, 'source_chart_endpoints': [aa['xy'].tolist(), bb['xy'].tolist()]})
            vals = [x for x in record['gaps_mm'] if x is not None]
            record['diagnostic_summary'] = forehead_atlas_summary(vals)
            record['unresolved_sections'] = sum((x is None for x in record['gaps_mm']))
            record['correct_transverse_order'] = all((x is not None and x > 0 for x in record['signed_chart_separation']))
            return record
        for a, b in zip(names, names[1:]):
            end_angle = max(terminal_angles[a], terminal_angles[b], 0.0)
            charts = [sections(a, b, 'upper', np.linspace(0, self.entry, count)), sections(a, b, 'radial', np.linspace(np.pi / 2, end_angle, count))]
            lower_end = None
            if max(self.old[roles[a], 2], self.old[roles[b], 2]) < self.exit_h - 1e-10:
                lower_end = max(self.old[roles[a], 2], self.old[roles[b], 2])
                charts.append(sections(a, b, 'lower', np.linspace(self.exit_h, lower_end, count)))
            coverage = {}
            for n in (a, b):
                c = curves[n]
                direction = np.array([np.cos(end_angle), np.sin(end_angle)])
                hh = forehead_atlas_intersections(*c['radial'], pivot, direction)
                terminal_s = None
                if len(hh) == 1:
                    local_s = hh[0]['s']
                    index = int(local_s)
                    local_fraction = local_s - index
                    indices = [c['entry']] + list(range(int(np.floor(c['entry'])) + 1, int(np.ceil(c['radial_end'])))) + [c['radial_end']]
                    terminal_s = indices[index] * (1 - local_fraction) + indices[min(index + 1, len(indices) - 1)] * local_fraction
                if lower_end is not None:
                    hh = forehead_atlas_intersections(c['xy'], c['world'], np.array([0.0, lower_end]), np.array([1.0, 0.0]))
                    hh = [z for z in hh if z['s'] >= c['radial_end'] - 1e-08]
                    terminal_s = hh[0]['s'] if len(hh) == 1 else None
                total = arc(c['world'], 0, len(paths[n]) - 1)
                matched = arc(c['world'], 0, terminal_s) if terminal_s is not None else None
                coverage[n] = {'total_arc_mm': total, 'matched_arc_mm': matched, 'unmatched_arc_mm': total - matched if matched is not None else None, 'unmatched_curve_start_parameter': terminal_s, 'all_arc_accounted': terminal_s is not None}
            crossing = []
            aa, bb = (curves[a]['xy'], curves[b]['xy'])
            for ia, (pa, qa) in enumerate(zip(aa, aa[1:])):
                da = qa - pa
                for ib, (pb, qb) in enumerate(zip(bb, bb[1:])):
                    db = qb - pb
                    den = forehead_atlas_cross(da, db)
                    if abs(den) < 1e-14:
                        continue
                    ta = forehead_atlas_cross(pb - pa, db) / den
                    tb = forehead_atlas_cross(pb - pa, da) / den
                    if 1e-09 < ta < 1 - 1e-09 and 1e-09 < tb < 1 - 1e-09:
                        crossing.append([ia, ib])
            pair = {'first': a, 'second': b, 'fixed_radial_endpoint_angle': end_angle, 'charts': charts, 'coverage': coverage, 'adjacent_source_chart_crossings': crossing}
            out['pairs'].append(pair)
        return out

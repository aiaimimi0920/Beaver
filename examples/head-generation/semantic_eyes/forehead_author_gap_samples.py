"""Scratch fixed-source gap residuals. No nearest-neighbor remasking or fitting.
Rebuild only after an authorized topology change. Invalid means all rows invalid.
External final verification must run slow atlas and source-sheet connectivity.
"""
from forehead_author_atlas import forehead_atlas_cross, forehead_atlas_sub
import numpy as np

def make_forehead_gap_residual(*args, **kwargs):
    """Ordered-package boundary and compatibility construction entry."""
    return ForeheadFrozenGapResidual(*args, **kwargs)


class ForeheadFrozenGapResidual:

    def __init__(self, atlas, proposal, count=33, cache_limit=64):
        if not isinstance(cache_limit, int) or cache_limit < 0:
            raise ValueError('Invalid exact gap-cache capacity')
        self.cache_limit = cache_limit
        self.cache = {}
        if not isinstance(count, int) or count < 3:
            raise ValueError('Invalid section count')
        self.atlas = atlas
        self.paths = {n: tuple(p) for n, p in proposal['paths'].items()}
        self.names = ('red', 'orange', 'green', 'blue', 'purple')
        self.count = count
        self.entry = atlas.entry
        self.height = atlas.exit_h
        self.pivot = np.array([self.entry, self.height])
        self.roles = {n: p[-1] for n, p in self.paths.items()}
        if any((i >= len(atlas.old) or i not in proposal['protected'] for i in self.roles.values())):
            raise ValueError('Frozen target terminals must be protected original points')
        self.endpoint_world = {n: atlas.old[i].copy() for n, i in self.roles.items()}
        self.endpoint_chart = {n: atlas.sc[i, [0, 2]].copy() for n, i in self.roles.items()}
        self.groups = []
        self.specs = []
        offset = 0
        angles = {n: np.arctan2(x[1] - self.height, x[0] - self.entry) for n, x in self.endpoint_chart.items()}
        for first, second in zip(self.names, self.names[1:]):
            end = max(angles[first], angles[second], 0.0)
            u = np.linspace(0, self.entry, count)
            origins = np.column_stack((u, np.zeros(count)))
            directions = np.tile([0.0, 1.0], (count, 1))
            self.specs.append((first, second, 'upper', origins, directions))
            phi = np.linspace(np.pi / 2, end, count)
            self.specs.append((first, second, 'radial', np.tile(self.pivot, (count, 1)), np.column_stack((np.cos(phi), np.sin(phi)))))
            if max(self.endpoint_chart[first][1], self.endpoint_chart[second][1]) < self.height - 1e-10:
                hs = np.linspace(self.height, max(self.endpoint_chart[first][1], self.endpoint_chart[second][1]), count)
                self.specs.append((first, second, 'lower', np.column_stack((np.zeros(count), hs)), np.tile([1.0, 0.0], (count, 1))))
        for first, second, kind, o, d in self.specs:
            self.groups.append((offset, offset + count, list(self.paths[first] + self.paths[second])))
            offset += count
        self.row_count = offset

    def _invalid(self, reason):
        return {'valid': False, 'reason': reason, 'gaps_mm': np.full(self.row_count, np.nan)}

    @staticmethod
    def _hits(xy, world, origins, directions):
        edges = np.diff(xy, axis=0)
        den = forehead_atlas_cross(edges[:, None, :], directions[None, :, :])
        num = forehead_atlas_cross(origins[None, :, :] - xy[:-1, None, :], directions[None, :, :])
        t = np.divide(num, den, out=np.full_like(den, np.nan), where=np.abs(den) > 1e-14)
        valid = np.isfinite(t) & (t >= -1e-09) & (t <= 1 + 1e-09)
        tc = np.clip(t, 0, 1)
        s = np.arange(len(edges))[:, None] + tc
        for k in range(1, len(edges)):
            duplicate = valid[k] & valid[k - 1] & (np.abs(s[k] - s[k - 1]) <= 1e-07)
            valid[k, duplicate] = False
        counts = valid.sum(axis=0)
        if np.any(counts != 1):
            return (None, counts)
        seg = valid.argmax(axis=0)
        cols = np.arange(len(origins))
        fraction = tc[seg, cols]
        points = world[seg] + fraction[:, None] * (world[seg + 1] - world[seg])
        chart = xy[seg] + fraction[:, None] * edges[seg]
        return ((points, chart, seg, fraction), counts)

    def evaluate(self, world, author_params, paths=None, details=False):
        if paths is not None and {n: tuple(p) for n, p in paths.items()} != self.paths:
            return self._invalid('Frozen paths changed')
        world = np.asarray(world)
        params = np.asarray(author_params)
        params = params[:, [0, 2]] if params.shape[1] == 3 else params
        if not np.isfinite(world).all() or not np.isfinite(params).all():
            return self._invalid('Nonfinite input')
        key = (world.dtype.str, params.dtype.str, tuple(((world[list(self.paths[n])].tobytes(), params[list(self.paths[n])].tobytes()) for n in self.names)))
        if not details and key in self.cache:
            return {'valid': True, 'reason': None, 'gaps_mm': self.cache[key], 'details': []}
        pieces = {}
        piece_indices = {}
        for name in self.names:
            path = np.asarray(self.paths[name])
            xy = params[path]
            ww = world[path]
            if np.any(np.linalg.norm(np.diff(ww, axis=0), axis=1) <= 1e-09):
                return self._invalid(name + ': collapsed physical guide segment')
            if not np.array_equal(ww[-1], self.endpoint_world[name]) or not np.array_equal(xy[-1], self.endpoint_chart[name]):
                return self._invalid('Frozen terminal coordinates changed')
            du = np.diff(xy[:, 0])
            cand = np.flatnonzero((du > 0) & (xy[:-1, 0] <= self.entry + 1e-12) & (xy[1:, 0] >= self.entry - 1e-12))
            en = []
            for k in cand:
                t = (self.entry - xy[k, 0]) / du[k]
                if -1e-09 <= t <= 1 + 1e-09:
                    value = k + float(np.clip(t, 0, 1))
                    if not en or abs(value - en[-1]) > 1e-07:
                        en.append(value)
            if len(en) != 1:
                return self._invalid(name + ': nonunique fixed angular entry')
            en = en[0]
            end = len(path) - 1
            if xy[-1, 1] < self.height - 1e-10:
                dh = np.diff(xy[:, 1])
                cand = np.flatnonzero((dh < 0) & (xy[:-1, 1] >= self.height - 1e-12) & (xy[1:, 1] <= self.height + 1e-12))
                ex = []
                for k in cand:
                    t = (self.height - xy[k, 1]) / dh[k]
                    value = k + float(np.clip(t, 0, 1))
                    if -1e-09 <= t <= 1 + 1e-09 and value > en and (not ex or abs(value - ex[-1]) > 1e-07):
                        ex.append(value)
                if len(ex) != 1:
                    return self._invalid(name + ': nonunique fixed height exit')
                end = ex[0]
            core = (forehead_atlas_sub(xy, 0, en), forehead_atlas_sub(ww, 0, en))
            radial = (forehead_atlas_sub(xy, en, end), forehead_atlas_sub(ww, en, end))
            signed = -forehead_atlas_cross(radial[0][:-1] - self.pivot, np.diff(radial[0], axis=0))
            if np.any(np.diff(core[0][:, 0]) <= 1e-14) or np.any(signed <= 1e-14) or np.any(radial[0] < self.pivot - 1e-10):
                return self._invalid(name + ': nonmonotone or out-of-sector curve')
            pieces[name] = {'upper': core, 'radial': radial}

            def indices(a, b):
                return np.asarray([a] + list(range(int(np.floor(a)) + 1, int(np.ceil(b)))) + [b])
            piece_indices[name] = {'upper': indices(0, en), 'radial': indices(en, end)}
            if self.endpoint_chart[name][1] < self.height - 1e-10:
                lower = (forehead_atlas_sub(xy, end, len(path) - 1), forehead_atlas_sub(ww, end, len(path) - 1))
                if np.any(np.diff(lower[0][:, 1]) >= -1e-14):
                    return self._invalid(name + ': nonmonotone lower-height curve')
                pieces[name]['lower'] = lower
                piece_indices[name]['lower'] = indices(end, len(path) - 1)
        allgaps = []
        records = []
        for first, second, kind, origins, directions in self.specs:
            a, ca = self._hits(*pieces[first][kind], origins, directions)
            b, cb = self._hits(*pieces[second][kind], origins, directions)
            if a is None or b is None:
                return self._invalid(first + '-' + second + ' ' + kind + ': nonunique/missing section')
            signed = np.sum((a[1] - b[1]) * directions, axis=1)
            if np.any(signed <= 0):
                return self._invalid(first + '-' + second + ' ' + kind + ': transverse order failure')
            gaps = np.array([np.linalg.norm(row) * 1000 for row in a[0] - b[0]])
            allgaps.append(gaps)
            if details:
                tangents = []
                global_parameters = []
                for name, hits in [(first, a), (second, b)]:
                    w = pieces[name][kind][1]
                    t = w[hits[2] + 1] - w[hits[2]]
                    t /= np.linalg.norm(t, axis=1)[:, None]
                    tangents.append(t)
                    ix = piece_indices[name][kind]
                    global_parameters.append(ix[hits[2]] * (1 - hits[3]) + ix[hits[2] + 1] * hits[3])
                sep = a[0] - b[0]
                unit = sep / np.linalg.norm(sep, axis=1)[:, None]
                sines = [np.sqrt(np.maximum(0, 1 - np.sum(unit * t, axis=1) ** 2)) for t in tangents]
                records.append({'first': first, 'second': second, 'chart': kind, 'gaps_mm': gaps, 'section_sine_to_rails': sines, 'world_hits': [a[0], b[0]], 'chart_hits': [a[1], b[1]], 'curve_parameters': global_parameters})
        gaps = np.concatenate(allgaps)
        gaps.setflags(write=False)
        if not details and self.cache_limit:
            self.cache[key] = gaps
            while len(self.cache) > self.cache_limit:
                del self.cache[next(iter(self.cache))]
        return {'valid': True, 'reason': None, 'gaps_mm': gaps, 'details': records}

    @staticmethod
    def nearest(samples, curve):
        edges = np.diff(curve, axis=0)
        square = np.sum(edges * edges, axis=1)
        relative = samples[:, None, :] - curve[None, :-1, :]
        fraction = np.clip(np.sum(relative * edges[None, :, :], axis=2) / square[None, :], 0, 1)
        dist2 = np.sum((relative - fraction[:, :, None] * edges[None, :, :]) ** 2, axis=2)
        selected = dist2.argmin(axis=1)
        rows = np.arange(len(samples))
        return (np.sqrt(dist2[rows, selected]) * 1000, selected + fraction[rows, selected])

    def evaluate_nearest(self, world, author_params, paths=None, final_extrema=False):
        """Same fixed SOURCE sections; nearest search never defines their domain.
  Report both whole-target and union-of-all-pair-matched-charts distances.
  Whole-target projection outside the pair's interval is recorded, not hidden.
  """
        result = self.evaluate(world, author_params, paths, details=True)
        if not result['valid']:
            return result
        world = np.asarray(world)
        ends = {}
        records = []
        raw = []
        restricted = []
        for row in result['details']:
            key = (row['first'], row['second'])
            ends.setdefault(key, {})
            for n, ss in zip(key, row['curve_parameters']):
                ends[key][n] = max(ends[key].get(n, 0), float(ss.max()))
        for row in result['details']:
            key = (row['first'], row['second'])
            for side, (source, target) in enumerate((key, key[::-1])):
                curve = world[list(self.paths[target])]
                samples = row['world_hits'][side]
                gap, where = self.nearest(samples, curve)
                matched = forehead_atlas_sub(curve, 0, ends[key][target])
                limited, _ = self.nearest(samples, matched)
                raw.append(gap)
                restricted.append(limited)
                rec = {'source': source, 'target': target, 'chart': row['chart'], 'whole_target_gaps_mm': gap, 'matched_union_target_gaps_mm': limited, 'whole_target_curve_parameters': where, 'source_curve_parameters': row['curve_parameters'][side], 'matched_target_end_parameter': ends[key][target], 'outside_matched_target': where > ends[key][target] + 1e-08, 'paired_section_lengths_mm': row['gaps_mm']}
                records.append(rec)
        extrema = []
        if final_extrema:
            for key, limits in ends.items():
                for source, target in (key, key[::-1]):
                    nodes = np.arange(int(np.floor(limits[source] + 1e-09)) + 1)
                    samples = world[np.asarray(self.paths[source])[nodes]]
                    curve = world[list(self.paths[target])]
                    gap, where = self.nearest(samples, curve)
                    limited, _ = self.nearest(samples, forehead_atlas_sub(curve, 0, limits[target]))
                    extrema.append({'source': source, 'target': target, 'source_vertex_ids': np.asarray(self.paths[source])[nodes], 'whole_target_gaps_mm': gap, 'matched_union_target_gaps_mm': limited, 'outside_matched_target': where > limits[target] + 1e-08})
        return {'valid': True, 'reason': None, 'section_gaps_mm': result['gaps_mm'], 'nearest_gaps_mm': np.concatenate(raw), 'matched_union_nearest_gaps_mm': np.concatenate(restricted), 'records': records, 'vertex_extrema': extrema, 'matched_end_parameters': ends}

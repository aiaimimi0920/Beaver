"""Immutable, content-bound source packet and explicit side lookup.

This is a bounded rail-point/corner proposal API, NOT full face-domain coverage.
"""
import hashlib
import json
import math
from .geometry import metrics, cross, norm, sub, finite_vector, validate_row, evaluate

def encode(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()

def digest(value):
    return hashlib.sha256(encode(value)).hexdigest()

class Source:

    def __init__(self, payload, expected_hash):
        raw = encode(payload)
        if hashlib.sha256(raw).hexdigest() != expected_hash:
            raise ValueError('stale source hash')
        self._raw = raw
        self.sha256 = expected_hash
        try:
            self.validate()
        except (KeyError, TypeError, IndexError) as exc:
            raise ValueError('malformed source packet') from exc

    @property
    def data(self):
        return json.loads(self._raw)

    def validate(self):
        p = self.data
        vertices = p['points']
        faces = p['faces']
        rails = p['rails']
        if p['schema'] != 'IMMUTABLE_RAIL_SOURCE_V1':
            raise ValueError('schema')
        if len(p['keys']) != 6 or len(set(p['keys'])) != 6:
            raise ValueError('six explicit source keys required')
        if len(p['uv_layers']) != 3 or len(set(p['uv_layers'])) != 3:
            raise ValueError('three explicit UV layers required')
        roles = p['baseline_roles']
        if set(roles) != {'raw', 'author'}:
            raise ValueError('baseline role records')
        for name, record in roles.items():
            if record['role'] != name or not isinstance(record['capture_id'],
                str) or (not record['capture_id']):
                raise ValueError('baseline role binding')
            value = record['content_sha256']
            if not isinstance(value,
                str) or len(value) != 64 or any((x not in '0123456789abcdef' for x in value)):
                raise ValueError('baseline content hash')
        if not vertices or not faces or (not rails) or (not p['transverse']):
            raise ValueError('empty source domain')
        if set(p['frozen_fields']) != set(vertices):
            raise ValueError('baseline coverage')
        for uid, point in vertices.items():
            if len(point) != 3:
                raise ValueError('point')
            finite_vector(point)
            if set(p['frozen_fields'][uid]) != set(p['keys']):
                raise ValueError('key coverage')
            for value in p['frozen_fields'][uid].values():
                if len(value) != 3:
                    raise ValueError('field')
                finite_vector(value)
                finite_vector((x + y for x, y in zip(point, value)))
        edges = set()
        for owner, f in faces.items():
            v = f['vertices']
            c = f['corners']
            if len(v) != 4 or len(set(v)) != 4 or len(c) != 4 or (not set(v) <= set(vertices)):
                raise ValueError('quad')
            if len(f['diagonal']) != 2 or set(f['diagonal']) not in ({v[0], v[2]}, {v[1], v[3]}):
                raise ValueError('fixed diagonal')
            start = next((j for j in range(4) if {v[j], v[(j + 2) % 4]} == set(f['diagonal'])))
            w = v[start:] + v[:start]
            for a, b, third in [(w[0], w[1], w[2]), (w[0], w[2], w[3])]:
                if norm(cross(sub(vertices[b], vertices[a]),
                    sub(vertices[third], vertices[a]))) == 0:
                    raise ValueError('degenerate fixed triangle')
            for j, uid in enumerate(v):
                edges.add(frozenset((uid, v[(j + 1) % 4])))
                if c[j]['vertex'] != uid or not c[j]['side'] or set(c[j]['uv']) != set(p['uv_layers']):
                    raise ValueError('corner side/layer')
                for value in c[j]['uv'].values():
                    if len(value) != 2:
                        raise ValueError('UV')
                    finite_vector(value)
        for name, pairs in p['transverse'].items():
            if not name or not pairs or any((len(e) != 2 or len(set(e)) != 2 or frozenset(e) not in edges for e in pairs)) or (len({frozenset(e) for e in pairs}) != len(pairs)):
                raise ValueError('transverse edge domain')
        seen = set()
        for name, path in rails.items():
            if not name or len(path) < 3 or len(set(path)) != len(path):
                raise ValueError('ambiguous role')
            if seen.intersection(path):
                raise ValueError('ambiguous role')
            seen.update(path)
            if not {path[0], path[-1]} <= set(p['protected']):
                raise ValueError('unprotected endpoint')
            if any((frozenset((a, b)) not in edges for a, b in zip(path, path[1:]))):
                raise ValueError('rail not actual edges')
            metrics(vertices, path)
        if not set(p['protected']) <= set(vertices):
            raise ValueError('unknown protected identity')
        rail_of = {uid: name for name, path in rails.items() for uid in path}
        supplied = set()
        declared_pairs = set()
        for pairs in p['transverse'].values():
            pair_roles = set()
            for a, b in pairs:
                if a not in rail_of or b not in rail_of or rail_of[a] == rail_of[b]:
                    raise ValueError('transverse role mismatch')
                edge = frozenset((a, b))
                if edge in supplied:
                    raise ValueError('duplicate transverse domain')
                supplied.add(edge)
                pair_roles.add(frozenset((rail_of[a], rail_of[b])))
            if len(pair_roles) != 1:
                raise ValueError('mixed transverse rail pair')
            pair = next(iter(pair_roles))
            if pair in declared_pairs:
                raise ValueError('split transverse rail-pair domain')
            declared_pairs.add(pair)
        expected = {edge for edge in edges if all((uid in rail_of for uid in edge)) and len({rail_of[uid] for uid in edge}) == 2}
        if supplied != expected:
            raise ValueError('incomplete transverse source edge domain')

    def transverse_domains(self):
        """Derive complete actual edge domains; caller names are display labels only.

        Validation permits exactly one declared group per unordered rail pair.
        Neither its listed ordering nor a singleton split controls metric scope.
        """
        p = self.data
        role = {uid: name for name, path in p['rails'].items() for uid in path}
        station = {uid: i for path in p['rails'].values() for i, uid in enumerate(path)}
        by_pair = {}
        for f in p['faces'].values():
            v = f['vertices']
            for a, b in zip(v, v[1:] + v[:1]):
                if a not in role or b not in role or role[a] == role[b]:
                    continue
                if role[a] > role[b]:
                    a, b = (b, a)
                pair = (role[a], role[b])
                by_pair.setdefault(pair, set()).add((a, b))
        result = {}
        for label, declared in p['transverse'].items():
            a, b = declared[0]
            pair = tuple(sorted((role[a], role[b])))
            result[label] = [list(e) for e in sorted(by_pair[pair],
                key=lambda e: (station[e[0]], station[e[1]]))]
        return result

    def readset(self):
        p = self.data
        return {'source': self.sha256,
            'baseline': digest(p['frozen_fields']),
            'baseline_roles': digest(p['baseline_roles']),
            'faces': {uid: digest(f) for uid, f in p['faces'].items()},
            'points': {uid: digest(v) for uid, v in p['points'].items()}}

    def check_binding(self, binding):
        if binding != self.readset():
            raise ValueError('stale readset or rebased fields')

def corner_value(source, row, side, category):
    """Requires an explicit same-side original edge owner, never XYZ-nearest."""
    p = source.data
    candidates = []
    validate_row(row, p['points'])
    if not isinstance(side, str) or not side or (not isinstance(category, str)) or (not category):
        raise ValueError('missing corner side/category')
    ids = [uid for uid, _ in row]
    if len(ids) == 2 and (not any((any(({f['vertices'][j],
        f['vertices'][(j + 1) % 4]} == set(ids) for j in range(4))) for f in p['faces'].values()))):
        raise ValueError('nonadjacent source donors')
    for owner, f in p['faces'].items():
        if f['category'] != category:
            continue
        by_id = {c['vertex']: c for c in f['corners'] if c['side'] == side}
        ids = [uid for uid, _ in row]
        if not all((uid in by_id for uid in ids)):
            continue
        if len(ids) == 2 and (not any(({f['vertices'][j],
            f['vertices'][(j + 1) % 4]} == set(ids) for j in range(4)))):
            continue
        uv = {layer: evaluate({uid: c['uv'][layer] for uid, c in by_id.items()},
            row) for layer in p['uv_layers']}
        candidates.append((owner, uv))
    if not candidates:
        raise ValueError('missing corner side/source owner')
    first = candidates[0][1]
    if any((uv != first for _, uv in candidates[1:])):
        raise ValueError('ambiguous corner side')
    return {'side': side,
        'category': category,
        'source_owners': sorted((x[0] for x in candidates)),
        'uv': first}

"""Synthetic source construction only. No private character data or adapters."""
import copy
import hashlib
import random
from ..packet import Source, digest

def grid(rows, cols):
    points = {}
    fields = {}
    faces = {}
    rails = {}
    keys = ['open', 'blink_left', 'blink_right', 'round', 'spread', 'pucker']
    uv_layers = ['base', 'secondary', 'detail']
    protected = []
    for r in range(rows):
        path = []
        for c in range(cols):
            uid = f'point-{r}-{c}'
            t = (c / (cols - 1)) ** 1.7
            points[uid] = [0.024 * t, 0.004 * r, 0.003 * t * t + 0.0002 * r * t]
            fields[uid] = {key: [4e-05 * (j + 1) * t, 2e-05 * j * r / rows, 3e-05 * (j + 1) * t * t] for j,
                key in enumerate(keys)}
            path.append(uid)
            if r in (0, rows - 1) or c in (0, cols - 1):
                protected.append(uid)
        rails[f'rail-{r}'] = path
    for r in range(rows - 1):
        for c in range(cols - 1):
            v = [f'point-{r}-{c}',
                f'point-{r}-{c + 1}',
                f'point-{r + 1}-{c + 1}',
                f'point-{r + 1}-{c}']
            corners = []
            for uid in v:
                rr = int(uid.split('-')[1])
                x, y, z = points[uid]
                side = f'row-{rr}:toward-{r}'
                uv = {layer: [x * 20 + k * 0.1 + r % 2 * 0.2, y * 20 - k * 0.05] for k,
                    layer in enumerate(uv_layers)}
                corners.append({'vertex': uid, 'side': side, 'uv': uv})
            faces[f'owner-{r}-{c}'] = {'vertices': v,
                'diagonal': [v[0], v[2]],
                'corners': corners,
                'category': f'band-{r}'}
    payload = {'schema': 'IMMUTABLE_RAIL_SOURCE_V1',
        'baseline_roles': {name: {'role': name, 'capture_id': name + '-fixture', 'content_sha256': hashlib.sha256((name + ' fixture').encode()).hexdigest()} for name in ('raw', 'author')},
        'points': points,
        'frozen_fields': fields,
        'faces': faces,
        'rails': rails,
        'transverse': {f'gap-{r}': [[f'point-{r}-{c}', f'point-{r + 1}-{c}'] for c in range(cols)] for r in range(rows - 1)},
        'protected': protected,
        'keys': keys,
        'uv_layers': uv_layers}
    return Source(payload, digest(payload))

def permute(source):
    p = source.data
    rng = random.Random(624)
    ids = list(p['points'])
    shuffled = ids[:]
    rng.shuffle(shuffled)
    vm = {uid: f'v{j}' for j, uid in enumerate(shuffled)}
    owners = list(p['faces'])
    rng.shuffle(owners)
    fm = {uid: f'f{j}' for j, uid in enumerate(owners)}
    p['points'] = {vm[uid]: p['points'][uid] for uid in reversed(ids)}
    p['frozen_fields'] = {vm[uid]: p['frozen_fields'][uid] for uid in reversed(ids)}
    p['protected'] = [vm[uid] for uid in p['protected']]
    p['transverse'] = {name: [[vm[a], vm[b]] for a, b in pairs] for name,
        pairs in p['transverse'].items()}
    p['rails'] = {name: [vm[uid] for uid in path] for name,
        path in reversed(list(p['rails'].items()))}
    out = {}
    for owner in owners:
        f = p['faces'][owner]
        shift = rng.randrange(4)
        f['vertices'] = [vm[v] for v in f['vertices']]
        f['diagonal'] = [vm[v] for v in f['diagonal']]
        for corner in f['corners']:
            corner['vertex'] = vm[corner['vertex']]
        f['vertices'] = f['vertices'][shift:] + f['vertices'][:shift]
        f['corners'] = f['corners'][shift:] + f['corners'][:shift]
        out[fm[owner]] = f
    p['faces'] = out
    return (Source(p, digest(p)), vm, fm)

def joint_grid(rows, cols):
    """Different explicit feature binding: vertical end interfaces are protected.

    All horizontal source rails are movable; this is not the outer-pinned case.
    A source-owned localized transverse bulge supplies a genuine joint defect.
    """
    s = grid(rows, cols)
    p = s.data
    p['protected'] = [uid for uid in p['points'] if int(uid.split('-')[2]) in (0, cols - 1)]
    for uid, point in p['points'].items():
        r, c = map(int, uid.split('-')[1:])
        point[2] = 0.003 * (point[0] / 0.024) ** 2
        if c == 1:
            point[1] += 0.00055 * (-1) ** r
    return Source(p, digest(p))

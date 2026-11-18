"""Bind a current-build lower-mouth chart through native mesh assembly."""
import numpy as np


def chin_oriented_cycle(face):
    face = tuple(face)
    return min(face[k:] + face[:k] for k in range(len(face)))


def capture_chin_source_roles(vertices, faces, rows, units, outer, mouth, recipe):
    """Capture semantic positions before compaction, never historical mesh IDs."""
    if list(map(len, rows)) != [5, 7, 9, 13] or len(units) != 4:
        raise ValueError('Changed lower-chin row or transition contract')
    if any(len(unit) != 3 for unit in units) or len(mouth) != 1:
        raise ValueError('Ambiguous lower-chin transitions or mouth owner')
    roles = {f'chin_row_{r}_station_{k}': index
             for r, row in enumerate(rows) for k, index in enumerate(row)}
    for n, unit in enumerate(units):
        roles.update({f'chin_transition_{n}_{name}': index
                      for name, index in zip(('left', 'middle', 'right'), unit)})
    lower = outer['lower_transition']['vertex_ids']
    if len(lower) != 3 or len(mouth[0]['rings']) != 5:
        raise ValueError('Changed outer-mouth source role contract')
    roles.update({f'outer_lower_transition_{name}': index
                  for name, index in zip(('left', 'middle', 'right'), lower)})
    roles.update({f'mouth_ring_3_station_{k}': index
                  for k, index in enumerate(mouth[0]['rings'][3])})
    required = {role for key in ('old_faces', 'replacement_faces')
                for face in recipe[key] for role in face}
    if len(required) != 51 or not set(recipe['movable_roles']) <= required:
        raise ValueError('Changed bounded lower-mouth recipe domain')
    selected = {name: roles[name] for name in sorted(required)}
    if not np.isfinite(np.asarray(vertices)[list(selected.values())]).all():
        raise ValueError('Nonfinite lower-mouth source capture')
    if len(set(selected.values())) != len(selected):
        raise ValueError('Lower-mouth roles alias different source stations')
    lookup = {}
    for face in faces:
        key = chin_oriented_cycle(face)
        lookup[key] = lookup.get(key, 0) + 1
    for face in recipe['old_faces']:
        if lookup.get(chin_oriented_cycle([selected[name] for name in face])) != 1:
            raise ValueError('Lower-mouth recipe is not the current oriented source disk')
    return {'version': 1, 'positions': {name: list(vertices[index])
            for name, index in selected.items()}, 'face_count': len(recipe['old_faces'])}


def bind_chin_source_roles(vertices, faces, regions, authored, recipe, assembly_offset):
    """Resolve both actual skin halves with unique matches and oriented cycles."""
    points = np.asarray(vertices, dtype=float)
    if not np.isfinite(points).all():
        raise ValueError('Nonfinite assembled lower-mouth source')
    required = {role for key in ('old_faces', 'replacement_faces')
                for face in recipe[key] for role in face}
    if (authored.get('version') != 1 or set(authored['positions']) != required
            or authored['face_count'] != len(recipe['old_faces'])):
        raise ValueError('Lower-mouth source capture does not match its recipe')
    skin = sorted({i for face, region in zip(faces, regions) if region == 1 for i in face})
    skin_points = points[skin]
    result = []
    lookup = {}
    for index, (face, region) in enumerate(zip(faces, regions)):
        if region == 1:
            lookup.setdefault(chin_oriented_cycle(face), []).append(index)
    for side in (1, -1):
        bound = {}
        for name, value in authored['positions'].items():
            expected = np.asarray(value, dtype=np.float32).astype(float)
            if expected.shape != (3,) or not np.isfinite(expected).all():
                raise ValueError('Invalid lower-mouth authored station')
            expected[2] = np.float32(expected[2] + assembly_offset)
            expected[0] *= side
            distances = np.linalg.norm(skin_points - expected, axis=1)
            ordered = np.argsort(distances)[:2]
            if len(ordered) != 2 or distances[ordered[0]] > 2e-6 or distances[ordered[1]] < 1e-5:
                raise ValueError('Missing or ambiguous current lower-mouth source role: ' + name)
            bound[name] = skin[int(ordered[0])]
        if len(set(bound.values())) != len(bound):
            raise ValueError('Lower-mouth source roles collapsed during assembly')
        selected = []
        for face in recipe['old_faces']:
            cycle = [bound[name] for name in face]
            if side < 0:
                cycle.reverse()
            hits = lookup.get(chin_oriented_cycle(cycle), [])
            if len(hits) != 1:
                raise ValueError('Current lower-mouth skin connectivity changed')
            selected.append(hits[0])
        result.append({'side': side, 'roles': bound, 'face_ids': selected})
    if set(result[0]['face_ids']) & set(result[1]['face_ids']):
        raise ValueError('Lower-mouth half domains overlap')
    return result

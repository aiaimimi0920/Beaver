"""Exact float32 affine transport of a fresh, identity-node Face-only GLB."""
import copy
import itertools
import json
import math
import struct
try:
    from general_face_controls import validate, vector
except ModuleNotFoundError:
    from controls import validate, vector


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_glb(raw):
    require(len(raw) >= 28, "Truncated GLB")
    magic, version, length = struct.unpack_from('<III', raw)
    require((magic, version, length) == (0x46546C67, 2, len(raw)), "Invalid GLB header")
    cursor, chunks = 12, []
    while cursor < len(raw):
        require(cursor + 8 <= len(raw), "Truncated chunk header")
        size, kind = struct.unpack_from('<II', raw, cursor)
        cursor += 8
        require(size % 4 == 0 and cursor + size <= len(raw), "Invalid chunk length")
        chunks.append((kind, raw[cursor:cursor + size]))
        cursor += size
    require([kind for kind, _ in chunks] == [0x4E4F534A, 0x004E4942], "Expected JSON and BIN only")
    doc, binary = json.loads(chunks[0][1]), chunks[1][1]
    require(len(doc.get('buffers', [])) == 1 and 'uri' not in doc['buffers'][0], "Expected embedded buffer")
    size = doc['buffers'][0]['byteLength']
    require(0 <= len(binary) - size <= 3, "Invalid embedded buffer length")
    return doc, binary[:size]


def write_glb(doc, binary):
    doc['buffers'][0]['byteLength'] = len(binary)
    source = json.dumps(doc, separators=(',', ':')).encode()
    source += b' ' * (-len(source) % 4)
    binary = bytes(binary) + b'\0' * (-len(binary) % 4)
    return (struct.pack('<III', 0x46546C67, 2, 28 + len(source) + len(binary))
            + struct.pack('<II', len(source), 0x4E4F534A) + source
            + struct.pack('<II', len(binary), 0x004E4942) + binary)


def read_rows(doc, binary, view_id, offset, count, fmt, packed=False):
    require(isinstance(view_id, int) and 0 <= view_id < len(doc['bufferViews']), "Invalid view")
    view = doc['bufferViews'][view_id]
    require(view.get('buffer') == 0 and not view.get('extensions'), "Unsupported buffer view")
    size = struct.calcsize(fmt)
    stride = view.get('byteStride', size)
    require(not packed or 'byteStride' not in view, "Sparse data must be tightly packed")
    require(stride >= size and (stride == size or stride % 4 == 0), "Invalid byte stride")
    start, length = view.get('byteOffset', 0), view['byteLength']
    component_size = struct.calcsize('<' + fmt[1])
    require((start + offset) % component_size == 0, "Misaligned accessor")
    require('byteStride' not in view or 4 <= stride <= 252, "Invalid interleaved stride")
    require(isinstance(count, int) and count >= 0 and offset >= 0 and start >= 0,
            "Invalid accessor range")
    require(start + length <= len(binary) and offset + (count - 1) * stride + size <= length,
            "Accessor outside buffer view")
    return [struct.unpack_from(fmt, binary, start + offset + i * stride) for i in range(count)]


def accessor_values(doc, binary, index, dimensions):
    require(isinstance(index, int) and 0 <= index < len(doc['accessors']), "Invalid accessor")
    accessor = doc['accessors'][index]
    require(accessor.get('componentType') == 5126 and accessor.get('type') == f'VEC{dimensions}'
            and not accessor.get('normalized', False) and not accessor.get('extensions'),
            "Only unnormalized float32 vectors are supported")
    count = accessor['count']
    require(isinstance(count, int) and count > 0, "Empty/invalid accessor")
    fmt = '<' + 'f' * dimensions
    rows = (read_rows(doc, binary, accessor['bufferView'], accessor.get('byteOffset', 0), count, fmt)
            if 'bufferView' in accessor else [(0.0,) * dimensions for _ in range(count)])
    if 'sparse' in accessor:
        sparse = accessor['sparse']
        require(not sparse.get('extensions') and 0 < sparse['count'] <= count, "Invalid sparse accessor")
        indices, values = sparse['indices'], sparse['values']
        require(indices['componentType'] in (5121, 5123, 5125), "Invalid sparse index type")
        fmt_index = {5121: '<B', 5123: '<H', 5125: '<I'}[indices['componentType']]
        ids = [v[0] for v in read_rows(doc, binary, indices['bufferView'], indices.get('byteOffset', 0),
                                     sparse['count'], fmt_index, packed=True)]
        require(ids == sorted(set(ids)) and ids[-1] < count, "Sparse indices must be unique, ordered and in range")
        replacements = read_rows(doc, binary, values['bufferView'], values.get('byteOffset', 0),
                                 sparse['count'], fmt, packed=True)
        for i, value in zip(ids, replacements):
            rows[i] = value
    require(all(math.isfinite(v) for row in rows for v in row), "Nonfinite accessor value")
    return rows


def face_primitives(doc):
    require(not doc.get('skins') and not doc.get('animations'), "Skinned/animated nodes are unsupported")
    require(len(doc.get('nodes', [])) == 1 and len(doc.get('meshes', [])) == 1, "Expected a Face-only GLB")
    node, mesh = doc['nodes'][0], doc['meshes'][0]
    require(node.get('mesh') == 0 and mesh.get('name') == 'Face', "Expected Face mesh")
    require(not node.get('children') and 'skin' not in node and not node.get('extensions'), "Unsupported node")
    require(all(value == 0 for value in node.get('weights', [0]))
            and all(value == 0 for value in mesh.get('weights', [0])), "Expected neutral morph weights")
    for key, identity in [('matrix', [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
                          ('translation', [0, 0, 0]), ('rotation', [0, 0, 0, 1]), ('scale', [1, 1, 1])]:
        require(key not in node or node[key] == identity, "Nonidentity Face node transform")
    require(mesh.get('extras', {}).get('targetNames') == ['MouthOpen'], "Expected MouthOpen target")
    require(not mesh.get('extensions'), "Unsupported mesh extension")
    for primitive in mesh['primitives']:
        require(primitive.get('mode', 4) == 4 and not primitive.get('extensions'), "Unsupported primitive")
        require({'POSITION', 'NORMAL'} <= primitive['attributes'].keys(), "Missing position or normal")
        require(all(key in ('POSITION', 'NORMAL', 'TANGENT') or key.startswith(('TEXCOORD_', 'COLOR_'))
                    for key in primitive['attributes']), "Unsupported vertex semantic")
        require(len(primitive.get('targets', [])) == 1, "Expected one morph target")
        require(set(primitive['targets'][0]) <= {'POSITION', 'NORMAL', 'TANGENT'}
                and 'POSITION' in primitive['targets'][0], "Unsupported morph semantic")
    return mesh['primitives']


def unit(values):
    length = math.sqrt(sum(x * x for x in values))
    require(math.isfinite(length) and length > 1e-20, "Invalid direction")
    return tuple(x / length for x in values)


def transport_glb(raw, proportions, pivot_blender):
    config, pivot_blender = validate(proportions), vector(pivot_blender)
    scales = config['width'], config['height'], config['depth']
    pivot = pivot_blender[0], pivot_blender[2], -pivot_blender[1]
    doc, binary = read_glb(raw)
    primitives = face_primitives(doc)
    transformed = {}
    frozen_accessors = {index for primitive in primitives
                        for semantic, index in primitive['attributes'].items()
                        if semantic not in ('POSITION', 'NORMAL', 'TANGENT')}
    frozen_accessors.update(primitive['indices'] for primitive in primitives if 'indices' in primitive)

    def linear_direction(row, semantic):
        factors = tuple(1 / s for s in scales) if semantic == 'NORMAL' else scales
        return tuple(v * s for v, s in zip(row[:3], factors))

    def direction(row, semantic):
        return unit(linear_direction(row, semantic))

    def remember(index, semantic, rows):
        require(index not in frozen_accessors, "Transformed accessor aliases a preserved semantic")
        if index in transformed:
            require(transformed[index] == (semantic, rows), "Conflicting shared accessor semantics")
        transformed[index] = (semantic, rows)

    for primitive in primitives:
        originals = {}
        for semantic in ('POSITION', 'NORMAL', 'TANGENT'):
            if semantic not in primitive['attributes']:
                continue
            index = primitive['attributes'][semantic]
            rows = accessor_values(doc, binary, index, 4 if semantic == 'TANGENT' else 3)
            originals[semantic] = rows
            if semantic == 'POSITION':
                mapped = [tuple(v if s == 1 else c + s * (v - c)
                                for v, c, s in zip(row, pivot, scales)) for row in rows]
            else:
                if semantic == 'TANGENT':
                    require(all(row[3] in (-1, 1) for row in rows), "Invalid tangent handedness")
                mapped = [direction(row, semantic) + (row[3:] if semantic == 'TANGENT' else ()) for row in rows]
            remember(index, semantic, mapped)
        require(all(len(rows) == len(originals['POSITION']) for rows in originals.values()), "Vertex count mismatch")
        for semantic, index in primitive['targets'][0].items():
            require(semantic in originals, "Morph has no matching base semantic")
            rows = accessor_values(doc, binary, index, 3)
            require(len(rows) == len(originals[semantic]), "Morph count mismatch")
            if semantic == 'POSITION':
                mapped = [tuple(v * s for v, s in zip(row, scales)) for row in rows]
            else:
                mapped = []
                for base, delta in zip(originals[semantic], rows):
                    transformed_base = linear_direction(base, semantic)
                    base_length = math.sqrt(sum(v * v for v in transformed_base))
                    require(math.isfinite(base_length) and base_length > 1e-20, "Invalid direction")
                    # Keep the existing endpoint validity guard, but never use
                    # an independently normalized endpoint to form the delta.
                    direction(tuple(a + b for a, b in zip(base[:3], delta)), semantic)
                    mapped.append(tuple(v / base_length for v in linear_direction(delta, semantic)))
                    # Base + w*delta is L(base + w*delta)/|L(base)|.
                    # Runtime normalization therefore preserves every weight.
            remember(index, 'MORPH_' + semantic, mapped)
    # Validate the full supported geometry even for the byte-exact identity path.
    if scales == (1.0, 1.0, 1.0):
        return raw
    result = bytearray(binary)
    for index, (_, rows) in transformed.items():
        result.extend(b'\0' * (-len(result) % 4))
        offset = len(result)
        width = len(rows[0])
        packed = b''.join(struct.pack('<' + 'f' * width, *row) for row in rows)
        rounded = list(struct.iter_unpack('<' + 'f' * width, packed))
        require(all(math.isfinite(v) for row in rounded for v in row), "float32 overflow")
        result.extend(packed)
        view_id = len(doc['bufferViews'])
        doc['bufferViews'].append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(packed)})
        accessor = copy.deepcopy(doc['accessors'][index])
        accessor.pop('sparse', None)
        accessor.update(bufferView=view_id, byteOffset=0,
                        min=[min(row[i] for row in rounded) for i in range(width)],
                        max=[max(row[i] for row in rounded) for i in range(width)])
        doc['accessors'][index] = accessor
    return write_glb(doc, result)


def assert_blender_positions(transported_raw, blender_raw, tolerance=3e-7):
    """Compare paired neutral/opened records, preserving morph correspondence."""
    from collections import defaultdict
    require(math.isfinite(tolerance) and tolerance > 0, "Invalid position tolerance")

    def records(raw):
        doc, binary = read_glb(raw)
        result = []
        for primitive in face_primitives(doc):
            base = accessor_values(doc, binary, primitive['attributes']['POSITION'], 3)
            delta = accessor_values(doc, binary, primitive['targets'][0]['POSITION'], 3)
            require(len(base) == len(delta), "Morph position count mismatch")
            result.extend(tuple(p) + tuple(a + b for a, b in zip(p, d)) for p, d in zip(base, delta))
        return result

    maximum = 0.0
    left, right = records(transported_raw), records(blender_raw)
    for source, target in ((left, right), (right, left)):
        buckets = defaultdict(list)
        for record in target:
            # Index only neutral coordinates, but compare all six paired values.
            buckets[tuple(math.floor(v / tolerance) for v in record[:3])].append(record)
        for record in source:
            cell = tuple(math.floor(v / tolerance) for v in record[:3])
            candidates = (q for offset in itertools.product((-1, 0, 1), repeat=3)
                          for q in buckets.get(tuple(a + b for a, b in zip(cell, offset)), ()))
            distance = min((max(abs(a - b) for a, b in zip(record, q)) for q in candidates), default=math.inf)
            require(distance <= tolerance, "Transported/Blender paired positions disagree")
            maximum = max(maximum, distance)
    return {'base_and_mouthopen_endpoint_max_component_error_m': maximum,
            'tolerance_m': tolerance,
            'scope': 'Bidirectional paired neutral/opened record distance; identical export split duplicates allowed'}

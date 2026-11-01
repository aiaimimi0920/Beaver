"""Inactive source-field recovery receipts, separate from rendering precision.

These receipts bind an actual serialized asset and original source identities.
They do not confer geometry, shading, pose, or whole-model acceptance.
"""
import hashlib
import re
import numpy as np
from source_field_accessor import decode_accessor
from source_field_serialization import recover_color_channels


def _hashes(hashes):
    if not hashes or any(not isinstance(key, str) or not isinstance(value, str)
                         or re.fullmatch('[0-9a-f]{64}', value) is None
                         for key, value in hashes.items()):
        raise ValueError('Expected named immutable SHA256 identities')
    return dict(hashes)


def _indices(values, count, label):
    if len(values) != count or any(type(value) is not int or value < 0 for value in values):
        raise ValueError('Invalid ' + label)
    return list(values)


def _consistent_identity(ids, loops, vertices, normals, colors):
    corners, native_loops = {}, {}
    for row, (identity, loop, vertex) in enumerate(zip(ids, loops, vertices)):
        bits = normals[row].tobytes() + colors[row].tobytes()
        corner_record = (loop, vertex, bits)
        loop_record = (vertex, bits)
        if identity in corners and corners[identity] != corner_record:
            raise ValueError('Conflicting identity or fields for source corner')
        if loop in native_loops and native_loops[loop] != loop_record:
            raise ValueError('Conflicting vertex or fields for native loop')
        corners[identity] = corner_record
        native_loops[loop] = loop_record


def recover_source_fields(document, binary, attributes, *, source_corner_loops,
                          source_corner_vertices, source_hashes):
    """Read owned actual-accessor fields and bind each corner to native identity.

    A source corner may appear in multiple exported triangle corners. Repeated
    IDs must carry exactly identical authoritative fields; never guess a map
    from nearest geometry. The maps themselves come from the bound export
    sidecar and must cover every staging corner. No categorical interpolation.
    """
    hashes = _hashes(source_hashes)
    count = len(source_corner_loops)
    loops = _indices(source_corner_loops, count, 'native loop map')
    vertices = _indices(source_corner_vertices, count, 'native vertex map')
    loop_vertices = {}
    for loop, vertex in zip(loops, vertices):
        if loop in loop_vertices and loop_vertices[loop] != vertex:
            raise ValueError('Native loop maps to multiple vertices')
        loop_vertices[loop] = vertex
    index = attributes['_SOURCE_CORNER']
    values = decode_accessor(document, binary, index)
    accessor = document['accessors'][index]
    if accessor['componentType'] != 5126 or accessor['type'] != 'SCALAR':
        raise ValueError('Source corner IDs require FLOAT SCALAR')
    ids = values[:, 0]
    if np.any(ids < 0) or np.any(ids >= count) or np.any(ids != np.floor(ids)):
        raise ValueError('Nonintegral or out-of-range source corner ID')
    ids = ids.astype(np.int64)
    normal_index = attributes['NORMAL']
    normal_accessor = document['accessors'][normal_index]
    if normal_accessor['componentType'] != 5126 or normal_accessor['type'] != 'VEC3':
        raise ValueError('Authoritative normal requires FLOAT VEC3')
    normals = decode_accessor(document, binary, normal_index).astype('<f4')
    colors = recover_color_channels(document, binary, attributes)
    if len(normals) != len(ids) or len(colors) != len(ids):
        raise ValueError('Source field count mismatch')
    _consistent_identity(ids, [loops[i] for i in ids],
                         [vertices[i] for i in ids], normals, colors)
    return {
        'schema': 'SOURCE_FIELD_RECOVERY_V1', 'source_hashes': hashes,
        'source_corner_ids': ids.tolist(),
        'native_loop_ids': [loops[i] for i in ids],
        'native_vertex_ids': [vertices[i] for i in ids],
        'normal_float32_hex': normals.tobytes().hex(),
        'color_float32_hex': colors.tobytes().hex(),
        'normal_sha256': hashlib.sha256(normals.tobytes()).hexdigest(),
        'color_sha256': hashlib.sha256(colors.tobytes()).hexdigest(),
        'metadata_is_not_shader_binding': True,
    }


def verify_recovered_fields(expected, actual):
    """Require exact source identities and serialized float32 words on reload."""
    if expected != actual:
        raise ValueError('Source-field recovery mismatch')
    if (actual.get('schema') != 'SOURCE_FIELD_RECOVERY_V1'
            or actual.get('metadata_is_not_shader_binding') is not True):
        raise ValueError('Invalid source-field receipt contract')
    _hashes(actual['source_hashes'])
    count = len(actual['source_corner_ids'])
    for name in ('source_corner_ids', 'native_loop_ids', 'native_vertex_ids'):
        _indices(actual[name], count, name)
    fields = {}
    for field, width in (('normal', 3), ('color', 4)):
        data = bytes.fromhex(actual[field + '_float32_hex'])
        if (len(data) != count * width * 4
                or not np.isfinite(np.frombuffer(data, dtype='<f4')).all()
                or hashlib.sha256(data).hexdigest() != actual[field + '_sha256']):
            raise ValueError('Invalid recovered field digest')
        fields[field] = np.frombuffer(data, dtype='<f4').reshape(count, width)
    _consistent_identity(actual['source_corner_ids'], actual['native_loop_ids'],
                         actual['native_vertex_ids'], fields['normal'], fields['color'])
    return True

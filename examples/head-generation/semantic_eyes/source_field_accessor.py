"""Strict scalar/vector GLB accessors with Khronos glTF2 normalization."""
import numpy as np

MAX_DECODED_BYTES = 64 * 1024 * 1024


def decode_accessor(g, binary, index):
    """Decode one owned embedded-buffer scalar/vector accessor, failing closed.

    No compressed or extension-defined representation is accepted. Sparse
    substitutions precede Khronos integer normalization.
    """

    required = g.get('extensionsRequired', [])
    # Texture transforms operate on textureInfo, not stored accessor words.
    # Allowing this declaration does not validate texture/material semantics.
    if (type(required) is not list
            or any(type(name) is not str or name != 'KHR_texture_transform'
                   for name in required)
            or len(set(required)) != len(required)):
        raise ValueError('Unsupported or invalid required glTF extensions')

    def integer(v, name, minimum=0):
        if type(v) is not int or v < minimum:
            raise ValueError('Invalid ' + name)
        return v

    def no_extensions(value, label):
        if 'extensions' in value and value['extensions'] != {}:
            raise ValueError('Unsupported ' + label + ' extensions')
        if 'extensions' in value and type(value['extensions']) is not dict:
            raise ValueError('Invalid ' + label + ' extensions')

    def row(table, i):
        i = integer(i, 'index')
        values = g.get(table, [])
        if i >= len(values):
            raise ValueError('Out-of-range ' + table)
        return values[i]

    a = row('accessors', index)
    no_extensions(a, 'accessor')
    count = integer(a.get('count'), 'accessor count', 1)
    normalized = a.get('normalized', False)
    if type(normalized) is not bool:
        raise ValueError('normalized must be boolean')
    types = {5120: 'i1', 5121: 'u1', 5122: '<i2', 5123: '<u2', 5125: '<u4', 5126: '<f4'}
    ct = integer(a.get('componentType'), 'component type')
    dtype = types.get(ct)
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}.get(a.get('type'))
    if dtype is None or width is None:
        raise ValueError('Unsupported component/accessor type')
    if normalized and ct not in (5120, 5121, 5122, 5123):
        raise ValueError('Normalization forbidden for FLOAT/UNSIGNED_INT')
    dt = np.dtype(dtype)
    output_itemsize = 8 if normalized else dt.itemsize
    if count * width * output_itemsize > MAX_DECODED_BYTES:
        raise ValueError('Accessor exceeds decoded-byte budget')

    def read(viewid, offset, n, w, dtype, sparse=False):
        v = row('bufferViews', viewid)
        no_extensions(v, 'bufferView')
        if integer(v.get('buffer', 0), 'buffer') != 0:
            raise ValueError('Only GLB buffer0 supported')
        buffers = g.get('buffers', [])
        if len(buffers) != 1 or buffers[0].get('uri') is not None:
            raise ValueError('Expected one embedded GLB buffer')
        no_extensions(buffers[0], 'buffer')
        declared = integer(buffers[0].get('byteLength'), 'buffer byteLength')
        if declared > len(binary):
            raise ValueError('Truncated binary buffer')
        start = integer(v.get('byteOffset', 0), 'view byteOffset')
        length = integer(v.get('byteLength'), 'view byteLength', 1)
        off = integer(offset, 'accessor byteOffset')
        if start + length > declared:
            raise ValueError('BufferView exceeds buffer')
        dtype = np.dtype(dtype)
        size = w * dtype.itemsize
        if off % dtype.itemsize or (start + off) % dtype.itemsize:
            raise ValueError('Misaligned accessor')
        if sparse and 'target' in v:
            raise ValueError('Sparse bufferView must not define target')
        if 'target' in v and (type(v['target']) is not int
                              or v['target'] not in (34962, 34963)):
            raise ValueError('Invalid bufferView target')
        if sparse and 'byteStride' in v:
            raise ValueError('Sparse payload must be tightly packed')
        stride = integer(v.get('byteStride', size), 'stride', 1)
        if 'byteStride' in v and (not 4 <= stride <= 252 or stride % 4):
            raise ValueError('Invalid bufferView stride')
        if stride > length or stride < size or stride % dtype.itemsize:
            raise ValueError('Invalid element stride')
        if off + (n - 1) * stride + size > length:
            raise ValueError('Accessor exceeds bufferView')
        return np.ndarray(
            (n, w), dtype=dtype, buffer=binary, offset=start + off,
            strides=(stride, dtype.itemsize)).copy()

    if 'bufferView' in a:
        out = read(a['bufferView'], a.get('byteOffset', 0), count, width, dt)
    else:
        if 'byteOffset' in a:
            raise ValueError('Offset without base bufferView')
        out = np.zeros((count, width), dtype=dt)
    if 'sparse' in a:
        s = a['sparse']
        no_extensions(s, 'sparse accessor')
        no_extensions(s['indices'], 'sparse indices')
        no_extensions(s['values'], 'sparse values')
        n = integer(s.get('count'), 'sparse count', 1)
        if n > count:
            raise ValueError('Sparse count exceeds accessor')
        indices = s['indices']
        it = integer(indices.get('componentType'), 'sparse index type')
        if it not in (5121, 5123, 5125):
            raise ValueError('Invalid sparse index type')
        ids = read(indices['bufferView'], indices.get('byteOffset', 0),
                   n, 1, types[it], True).reshape(-1).astype(np.int64)
        if ids[-1] >= count or np.any(ids[1:] <= ids[:-1]):
            raise ValueError('Sparse indices not strictly increasing/in bounds')
        values = s['values']
        out[ids] = read(values['bufferView'], values.get('byteOffset', 0),
                        n, width, dt, True)
    if normalized:
        denominator = {5120: 127.0, 5121: 255.0, 5122: 32767.0, 5123: 65535.0}[ct]
        out = out.astype(np.float64)
        if ct in (5120, 5122):
            out = np.maximum(out, -denominator)
        out = out / denominator
    if not np.isfinite(out).all():
        raise ValueError('Nonfinite accessor')
    return out


def decode_position_target(g, binary, target, vertex_count):
    if type(vertex_count) is not int or vertex_count <= 0:
        raise ValueError('Invalid morph vertex count')
    if vertex_count * 3 * 4 > MAX_DECODED_BYTES:
        raise ValueError('Morph target exceeds decoded-byte budget')
    if 'POSITION' not in target:
        return np.zeros((vertex_count, 3), dtype=np.float32)
    index = target['POSITION']
    values = decode_accessor(g, binary, index)
    a = g['accessors'][index]
    if a['componentType'] != 5126 or a['type'] != 'VEC3' or a['count'] != vertex_count:
        raise ValueError('Invalid morph POSITION type/count')
    return values

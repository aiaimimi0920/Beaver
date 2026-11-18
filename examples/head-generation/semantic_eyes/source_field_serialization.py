"""Inactive lossless owned-field representation, without GLB geometry rewriting.

Four custom FLOAT SCALAR accessors avoid mixed-width custom-attribute importer
ambiguity. This does not bind source color to a renderer's standard COLOR_0.
"""
import copy
import numpy as np
from source_field_accessor import decode_accessor

COLOR_CHANNELS = ('_SOURCE_COLOR_R', '_SOURCE_COLOR_G',
                  '_SOURCE_COLOR_B', '_SOURCE_COLOR_A')


def split_color_channels(document, binary, semantic='_SOURCE_COLOR'):
    """Return a new document and receipt; entire original BIN remains unchanged.

    Only the owned custom semantic and appended accessor/view declarations may
    change. All custom attributes must then be homogeneous FLOAT SCALAR. The
    caller separately packs the document using its existing GLB serializer.
    """
    if not isinstance(semantic, str) or not semantic.startswith('_'):
        raise ValueError('Only an explicitly owned custom semantic may be split')
    result = copy.deepcopy(document)
    records = []
    for mesh_index, mesh in enumerate(result['meshes']):
        for primitive_index, primitive in enumerate(mesh['primitives']):
            attributes = primitive['attributes']
            if semantic not in attributes or any(n in attributes for n in COLOR_CHANNELS):
                raise ValueError('Missing source color or conflicting channels')
            source_index = attributes[semantic]
            expected = decode_accessor(document, binary, source_index)
            accessor = result['accessors'][source_index]
            if (accessor['componentType'] != 5126 or accessor['type'] != 'VEC4'
                    or accessor.get('normalized', False) or 'sparse' in accessor):
                raise ValueError('Expected dense unnormalized source FLOAT VEC4')
            for name, index in attributes.items():
                if not isinstance(name, str):
                    raise ValueError('Invalid attribute semantic name')
                if name.startswith('_') and name != semantic:
                    # Validate against ORIGINAL tables before appending accessors.
                    values = decode_accessor(document, binary, index)
                    row = document['accessors'][index]
                    if (row['type'] != 'SCALAR' or row['componentType'] != 5126
                            or row.get('normalized', False)
                            or len(values) != len(expected)):
                        raise ValueError('Custom FLOAT SCALAR type/count mismatch')
            view = copy.deepcopy(result['bufferViews'][accessor['bufferView']])
            view['byteStride'] = view.get('byteStride', 16)
            view_index = len(result['bufferViews'])
            result['bufferViews'].append(view)
            del attributes[semantic]
            indices = []
            for channel, name in enumerate(COLOR_CHANNELS):
                index = len(result['accessors'])
                result['accessors'].append({
                    'bufferView': view_index,
                    'byteOffset': accessor.get('byteOffset', 0) + 4 * channel,
                    'componentType': 5126, 'count': accessor['count'], 'type': 'SCALAR'})
                attributes[name] = index
                indices.append(index)
            for name, index in attributes.items():
                if name.startswith('_'):
                    row = result['accessors'][index]
                    if row['type'] != 'SCALAR' or row['componentType'] != 5126:
                        raise ValueError('Custom attributes are not homogeneous FLOAT SCALAR')
            actual = recover_color_channels(result, binary, attributes)
            if actual.tobytes() != expected.astype('<f4').tobytes():
                raise ValueError('Color source float32 bits changed')
            records.append({'mesh': mesh_index, 'primitive': primitive_index,
                            'source_accessor': source_index, 'scalar_accessors': indices})
    if not records:
        raise ValueError('No primitives to rewrite')
    return result, {'representation': 'FLOAT_SCALAR_RGBA4', 'records': records,
                    'binary_unchanged': True, 'metadata_is_not_shader_binding': True}


def recover_color_channels(document, binary, attributes):
    """Recover exact RGBA float32 without consulting standard COLOR_n fields."""
    channels = []
    for name in COLOR_CHANNELS:
        index = attributes[name]
        values = decode_accessor(document, binary, index)
        accessor = document['accessors'][index]
        if (accessor['componentType'] != 5126 or accessor['type'] != 'SCALAR'
                or accessor.get('normalized', False)):
            raise ValueError('Source channel is not unnormalized FLOAT SCALAR')
        channels.append(values[:, 0])
    if len({len(channel) for channel in channels}) != 1:
        raise ValueError('Source channel counts disagree')
    return np.column_stack(channels).astype('<f4')

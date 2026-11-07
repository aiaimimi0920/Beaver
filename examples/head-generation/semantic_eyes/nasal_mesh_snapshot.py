"""Read-only Blender Mesh snapshots; imports no Blender and accepts fake objects.

Compares payload, not datablock identities/names. Capture before and after without
changing modes. Geometry order must be identical (intended for no-edit transfers
and rollback); source-to-retopology comparisons need an explicit lineage map.
"""
import hashlib
import json
import math


def nasal_mesh_snapshot(data):
    """Hash topology, fields and key payloads exactly; reject unsupported public attrs.

    System-derived normals/caches, UI selection and hidden internal attributes are
    excluded. Material references use RNA identity (same live session), not names.
    Shape-key animation/driver datablocks are not modified by these mesh writes;
    their IDs/metadata are intentionally not a claim of this payload comparison.
    """
    def plain(value):
        if value is None or isinstance(value, (str, bool, int)):
            return value
        if isinstance(value, float):
            if not math.isfinite(value):
                raise ValueError('Nonfinite native mesh payload')
            return value.hex()
        if isinstance(value, dict):
            return {str(k): plain(v) for k, v in sorted(value.items())}
        return [plain(x) for x in value]

    def digest(value):
        raw = json.dumps(plain(value), sort_keys=True, separators=(',', ':')).encode()
        return hashlib.sha256(raw).hexdigest()

    def optional(obj, names):
        return {name: getattr(obj, name) for name in names if hasattr(obj, name)}

    def identity(block):
        return None if block is None else int(block.as_pointer())

    out = {'counts': [len(data.vertices), len(data.edges), len(data.polygons), len(data.loops)]}
    out['positions'] = digest([tuple(v.co) for v in data.vertices])
    out['edges'] = digest([tuple(e.vertices) for e in data.edges])
    out['polygons'] = digest([tuple(p.vertices) for p in data.polygons])
    out['loops'] = digest([(int(x.vertex_index), int(x.edge_index)) for x in data.loops])
    out['materials'] = digest({
        'slots': [identity(m) for m in data.materials],
        'indices': [int(p.material_index) for p in data.polygons]})
    out['face_smooth'] = digest([bool(p.use_smooth) for p in data.polygons])
    out['edge_flags'] = digest([optional(e, ('use_seam', 'use_edge_sharp')) for e in data.edges])
    out['deform'] = digest([sorted((int(g.group), float(g.weight)) for g in v.groups)
                            for v in data.vertices])
    out['uv'] = {}
    for layer in data.uv_layers:
        out['uv'][layer.name] = digest({
            'settings': optional(layer, ('active_render', 'active_clone')),
            'values': [tuple(x.uv) for x in layer.data],
            'pins': [bool(x.pin_uv) for x in layer.data]})
    active_uv = getattr(data.uv_layers, 'active', None)
    out['active_uv'] = None if active_uv is None else active_uv.name
    out['color_settings'] = optional(data.color_attributes,
                                      ('active_color_name', 'default_color_name'))
    # These builtins are captured above, or are ephemeral UI state. They are not
    # custom schema; BMesh may remove an implicit-default builtin representation.
    builtins = {'position', '.edge_verts', '.corner_vert', '.corner_edge',
                'material_index', 'sharp_face', 'sharp_edge', 'uv_seam'}
    value_fields = {'FLOAT': 'value', 'INT': 'value', 'INT8': 'value',
                    'BOOLEAN': 'value', 'STRING': 'value',
                    'FLOAT_VECTOR': 'vector', 'FLOAT2': 'vector',
                    'INT32_2D': 'value', 'FLOAT_COLOR': 'color',
                    'BYTE_COLOR': 'color', 'QUATERNION': 'value',
                    'FLOAT4X4': 'value'}
    out['attributes'] = {}
    for attr in data.attributes:
        if attr.name in builtins or attr.name.startswith('.') or getattr(attr, 'is_internal', False):
            continue
        if attr.domain not in ('POINT', 'EDGE', 'FACE', 'CORNER'):
            raise ValueError('Unsupported native attribute domain: ' + attr.name)
        prop = value_fields.get(attr.data_type)
        if prop is None or any(not hasattr(x, prop) for x in attr.data):
            raise ValueError('Unsupported native attribute payload: ' + attr.name + '/' + attr.data_type)
        out['attributes'][attr.name] = {'domain': attr.domain, 'type': attr.data_type,
            'count': len(attr.data), 'sha256': digest([getattr(x, prop) for x in attr.data])}
    keys = data.shape_keys
    out['shape_keys'] = None
    if keys is not None:
        blocks = list(keys.key_blocks)
        ordinal = {identity(k): i for i, k in enumerate(blocks)}
        records = []
        for key in blocks:
            settings = optional(key, ('name', 'value', 'slider_min', 'slider_max',
                                      'mute', 'interpolation', 'vertex_group', 'frame'))
            relative = getattr(key, 'relative_key', None)
            settings['relative_key_ordinal'] = None if relative is None else ordinal[identity(relative)]
            records.append({'settings': plain(settings), 'count': len(key.data),
                            'sha256': digest([tuple(p.co) for p in key.data])})
        reference = getattr(keys, 'reference_key', None)
        out['shape_keys'] = {'settings': plain(optional(keys, ('use_relative', 'eval_time'))),
            'reference_key_ordinal': None if reference is None else ordinal[identity(reference)],
            'blocks': records}
    return out


def assert_nasal_mesh_snapshot_equal(expected, actual, label='Native nasal mesh payload'):
    """Fail with differing sections, never with bulky/sensitive entire field payloads."""
    if expected != actual:
        changed = sorted(k for k in set(expected) | set(actual) if expected.get(k) != actual.get(k))
        raise ValueError(label + ' differs: ' + ', '.join(changed))
    return {'equal': True, 'sections': sorted(expected)}

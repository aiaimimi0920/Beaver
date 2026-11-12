"""Inactive native adapter: UV coordinates with verified unchanged false flags.

No bpy import or implicit scene operation. Call only on an owned staging mesh.
This narrow writer does not implement true pins or UI selection: reject those
requirements rather than silently losing them or allocating legacy lazy flags.
"""
import math
import struct


def _float32(value):
    number = float(value)
    if not math.isfinite(number):
        raise ValueError('Nonfinite UV coordinate')
    return struct.unpack('<f', struct.pack('<f', number))[0]


def write_uv_coordinates(mesh, layers):
    """Write {name: loop-ordered [u,v]} and verify coordinates and all flags.

    Narrow staging limits: at most eight layers, names <=63 UTF-8 bytes.
    Validate every payload before mutation. Allocate all layers before fetching
    any layer/data handles. Never write pin_uv/select/select_edge to False:
    these legacy setters may lazily allocate CustomData. Original pinned data
    belongs on the immutable source carrier, not this false-flag adapter.
    """
    if mesh.is_editmode:
        raise ValueError('UV staging requires object mode')
    prepared = {}
    for name, rows in layers.items():
        if (not isinstance(name, str) or not name or '\0' in name
                or len(name.encode('utf-8')) > 63
                or len(rows) != len(mesh.loops)):
            raise ValueError('Invalid UV layer name or loop count')
        values = []
        for row in rows:
            if len(row) != 2:
                raise ValueError('UV requires two coordinates')
            values.append([_float32(value) for value in row])
        prepared[name] = values
    missing = [name for name in prepared if mesh.uv_layers.get(name) is None]
    if len(mesh.uv_layers) + len(missing) > 8:
        raise ValueError('UV layer capacity exceeded')
    for name in prepared:
        attribute = mesh.attributes.get(name)
        if name in missing and attribute is not None:
            raise ValueError('UV name collides with an existing attribute')
        if name not in missing and (attribute is None
                or attribute.data_type != 'FLOAT2' or attribute.domain != 'CORNER'):
            raise ValueError('Existing UV attribute has unexpected type/domain')
    # Reject existing nonfalse flags before coordinates are changed.
    for name in prepared:
        if mesh.uv_layers.get(name) is not None:
            _verify_false_flags(mesh, name)
    created = []
    try:
        for name in missing:
            layer = mesh.uv_layers.new(name=name, do_init=False)
            if layer is None:
                raise ValueError('UV allocation failed')
            actual_name = layer.name
            created.append(actual_name)
            attribute = mesh.attributes.get(actual_name)
            if (actual_name != name or attribute is None
                    or attribute.data_type != 'FLOAT2' or attribute.domain != 'CORNER'):
                raise ValueError('UV allocation identity/type mismatch')
    except Exception:
        # Roll back only our new layers, before any existing coordinates change.
        for name in reversed(created):
            mesh.uv_layers.remove(mesh.uv_layers[name])
        raise
    for name, values in prepared.items():
        mesh.uv_layers[name].data.foreach_set(
            'uv', [coordinate for row in values for coordinate in row])
    for name, values in prepared.items():
        _verify_false_flags(mesh, name)
        for index, expected in enumerate(values):
            actual = mesh.uv_layers[name].data[index].uv
            if struct.pack('<2f', *actual) != struct.pack('<2f', *expected):
                raise ValueError('UV float32 readback mismatch')


def _verify_false_flags(mesh, name):
    for index in range(len(mesh.loops)):
        for flag in ('pin_uv', 'select', 'select_edge'):
            if getattr(mesh.uv_layers[name].data[index], flag) is not False:
                raise ValueError('Nonfalse UV flag outside this adapter contract')

"""Frozen field transport for a same-vertex lower-mouth retile."""
from math import isfinite


def chin_snapshot_point_fields(bm, parents):
    fields = []
    for kind in ('float', 'float_vector', 'color', 'float_color', 'shape', 'int', 'string'):
        accessor = getattr(bm.verts.layers, kind, None)
        for layer in accessor.values() if accessor is not None else []:
            values = [v[layer] if kind in ('float', 'int', 'string') else tuple(v[layer])
                      for v in parents]
            fields.append((kind, layer, values))
    deforms = [(layer, [dict(v[layer]) for v in parents])
               for layer in bm.verts.layers.deform.values()]
    return fields, deforms


def chin_interpolate_point_fields(index, weights, fields):
    """Return staged values; the seam flag is structural, not a sampled field."""
    if (not weights or not all(isfinite(w) for w in weights.values())
            or min(weights.values()) < 0
            or abs(sum(weights.values()) - 1) > 1e-8):
        raise ValueError('Invalid lower-mouth frozen donor weights')
    result = []
    for kind, layer, snapshots in fields:
        first = snapshots[next(iter(weights))]
        if kind == 'int' and layer.name == 'SmoothChinJoin':
            # Only interior stations move. The real pinned underside seam
            # retains its existing membership even when it is a donor corner.
            if snapshots[index] != 0:
                raise ValueError('Lower-mouth retile tries to move a chin-join member')
            value = snapshots[index]
        elif kind in ('int', 'string'):
            if any(snapshots[k] != first for k in weights):
                raise ValueError('Lower-mouth move crosses discrete source ownership')
            value = first
        elif kind == 'float':
            value = sum(w * snapshots[k] for k, w in weights.items())
        else:
            value = tuple(sum(w * snapshots[k][c] for k, w in weights.items())
                          for c in range(len(first)))
        result.append((layer, value))
    return result


def chin_apply_point_fields(parents, lineage, fields, deforms):
    staged = {i: chin_interpolate_point_fields(i, weights, fields)
              for i, weights in lineage.items()}
    for index, weights in lineage.items():
        for layer, value in staged[index]:
            parents[index][layer] = value
        for layer, snapshots in deforms:
            parents[index][layer].clear()
            keys = set().union(*(snapshots[k].keys() for k in weights))
            for key in keys:
                parents[index][layer][key] = sum(
                    w * snapshots[k].get(key, 0) for k, w in weights.items())


def chin_validate_face_fields(bm, selected):
    if len({face.material_index for face in selected}) != 1:
        raise ValueError('Lower-mouth retile crosses source materials')
    for kind in ('int', 'float', 'string', 'float_vector', 'color', 'float_color'):
        accessor = getattr(bm.faces.layers, kind, None)
        for layer in accessor.values() if accessor is not None else []:
            values = [face[layer] if kind in ('int', 'float', 'string')
                      else tuple(face[layer]) for face in selected]
            if any(value != values[0] for value in values[1:]):
                raise ValueError('Lower-mouth retile crosses face-field provenance')
    for kind in ('int', 'float', 'string', 'float_vector'):
        accessor = getattr(bm.loops.layers, kind, None)
        if accessor is not None and list(accessor.values()):
            raise ValueError('Unsupported lower-mouth corner field')

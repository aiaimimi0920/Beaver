"""Frozen point, loop-pin and edge provenance for appended source vertices."""
from math import isfinite


def stage_forehead_point_fields(old_count, lineage, fields, deforms):
    staged, staged_deforms, weights_by_vertex = {}, {}, {}
    for index, row in lineage.items():
        donors, shares = row['donors'], row['weights']
        if (len(donors) != len(shares) or len(set(donors)) != len(donors)
                or not all(isfinite(w) and w >= 0 for w in shares)
                or abs(sum(shares)-1) > 1e-8):
            raise ValueError('Invalid forehead field interpolation weights')
        weights = {int(i): float(w) for i, w in zip(donors, shares) if w > 0}
        if (not weights or any(i < 0 or i >= old_count for i in weights)
                or not all(isfinite(w) for w in weights.values())
                or abs(sum(weights.values())-1) > 1e-8):
            raise ValueError('Invalid frozen forehead field donor')
        weights_by_vertex[index] = weights
        staged[index], staged_deforms[index] = [], []
        for kind, layer, values in fields:
            first = values[next(iter(weights))]
            if kind in ('int', 'string'):
                if any(values[i] != first for i in weights):
                    raise ValueError('Forehead retile crosses a discrete point field')
                if index < old_count and values[index] != first:
                    raise ValueError('Forehead retile changes original discrete membership')
                value = first
            elif kind == 'float':
                value = sum(w*values[i] for i, w in weights.items())
            else:
                value = tuple(sum(w*values[i][j] for i, w in weights.items())
                              for j in range(len(first)))
            staged[index].append((layer, value))
        for layer, values in deforms:
            first = values[next(iter(weights))]
            if any(values[i] != first for i in weights) or (index < old_count and values[index] != first):
                raise ValueError('Forehead retile crosses semantic deform membership')
            staged_deforms[index].append((layer, dict(first)))
    return staged, staged_deforms, weights_by_vertex


def apply_forehead_point_fields(vertices, staged, deforms):
    for index, values in staged.items():
        for layer, value in values:
            vertices[index][layer] = value
        for layer, value in deforms[index]:
            vertices[index][layer].clear()
            for group, weight in value.items():
                vertices[index][layer][group] = weight


def snapshot_forehead_uv_pins(faces, indices, uv_layers):
    values = {}
    for face in faces:
        for loop in face.loops:
            index = indices[loop.vert]
            current = {layer: bool(loop[layer].pin_uv) for layer in uv_layers}
            if index in values and values[index] != current:
                raise ValueError('Forehead donor has conflicting UV pin ownership')
            values[index] = current
    return values


def assign_forehead_uv_pins(loop, weights, pins, uv_layers):
    for layer in uv_layers:
        values = {pins[i][layer] for i, weight in weights.items() if weight > 0}
        if len(values) != 1:
            raise ValueError('Forehead interpolation crosses a UV pin boundary')
        loop[layer].pin_uv = next(iter(values))


def snapshot_forehead_edges(bm):
    supported = {'crease_edge', 'FacialCreaseGroup'}
    for kind in ('float', 'int', 'string', 'float_vector', 'color', 'float_color'):
        accessor = getattr(bm.edges.layers, kind, None)
        if any(layer.name not in supported for layer in (accessor.values() if accessor is not None else [])):
            raise ValueError('Unsupported forehead edge-field schema')
    crease = bm.edges.layers.float.get('crease_edge')
    group = bm.edges.layers.int.get('FacialCreaseGroup')
    if crease is None or group is None:
        raise ValueError('Missing forehead edge provenance layers')
    values = {tuple(sorted(v.index for v in edge.verts)):
              (float(edge[crease]), int(edge[group]), bool(edge.seam), bool(edge.smooth))
              for edge in bm.edges}
    return values, crease, group


def restore_forehead_edges(bm, provenance, original, crease, group):
    parents = {tuple(row['edge']): row['parent'] for row in provenance}
    seen = set()
    inherited = 0
    for edge in bm.edges:
        key = tuple(sorted(v.index for v in edge.verts))
        if key not in parents:
            raise ValueError('Forehead edge lacks explicit provenance')
        parent = parents[key]
        if parent is None:
            expected = (0., 0, False, True)
        else:
            parent = tuple(parent)
            if parent not in original:
                raise ValueError('Forehead child edge refers to a non-original parent')
            expected = original[parent]
            inherited += int(key != parent and (expected[0] != 0 or expected[1] != 0))
        edge[crease], edge[group], edge.seam, edge.smooth = expected
        actual = (float(edge[crease]), int(edge[group]), bool(edge.seam), bool(edge.smooth))
        if actual != expected:
            raise ValueError('Forehead native edge transport changed its value')
        seen.add(key)
    if seen != set(parents):
        raise ValueError('Forehead native edge topology differs from proposal')
    return {'all_edges_bound': True, 'nonzero_inherited_child_edges': inherited,
            'original_seam_and_sharp_flags_inherited': True}


def forehead_named_field(values, name):
    matches = [value for layer, value in values if layer.name == name]
    if len(matches) != 1:
        raise ValueError('Missing or ambiguous staged forehead field: '+name)
    return matches[0]

"""Opt-in generalized source transport validation; never a surface binding.

No geometry mutation, source rebasing, categorical averaging or API fallback.
"""
from collections import Counter
from fractions import Fraction
import hashlib
import json
import re

import numpy as np


def _generalized_ids(values, *, unique=True):
    ids = list(values)
    if (any(type(v) is not int or v < 0 for v in ids)
            or (unique and len(set(ids)) != len(ids))):
        raise ValueError('Expected distinct nonnegative integer identities')
    return ids


def _generalized_numeric(values):
    # Preserve raw list types until boolean/string rejection is complete.
    raw = np.asarray(values, dtype=object)
    if any(isinstance(v, (bool, np.bool_)) or not isinstance(
            v, (int, float, np.integer, np.floating)) for v in raw.flat):
        raise ValueError('Numeric nonboolean values required')
    return np.asarray(values)


def generalized_source_weights(rows):
    """Validate exact rational partition of unity; do not normalize or prune."""
    if not 1 <= len(rows) <= 4:
        raise ValueError('Expected one to four explicit source donors')
    ids, weights = [], []
    for donor, ratio in rows:
        if (len(ratio) != 2 or any(type(v) is not int for v in ratio)
                or ratio[0] < 0 or ratio[1] <= 0):
            raise ValueError('Invalid nonnegative rational source weight')
        ids.append(donor)
        weights.append(Fraction(*ratio))
    _generalized_ids(ids)
    if sum(weights) != 1:
        raise ValueError('Source weights must sum exactly to one')
    return ids, weights


def generalized_source_binding(rows, *, source_hashes, frozen_field_identity):
    """Bind current weights separately from the immutable cumulative baseline."""
    generalized_source_weights(rows)
    if (not isinstance(frozen_field_identity, str) or not frozen_field_identity
            or not {'raw', 'author', 'frozen_fields'} <= source_hashes.keys()
            or any(not isinstance(k, str) or not isinstance(v, str)
                   or re.fullmatch('[0-9a-f]{64}', v) is None
                   for k, v in source_hashes.items())):
        raise ValueError('Missing immutable source or frozen-field identity')
    return json.loads(json.dumps({
        'schema': 'GENERALIZED_NONNEGATIVE_SOURCE_V1',
        'source_hashes': source_hashes,
        'frozen_field_identity': frozen_field_identity,
        'source_weights_rational': rows,
        'not_single_triangle_or_bilinear_binding': True,
    }))


def generalized_source_values(rows, source_values, *, categorical=False):
    """Continuous affine transport, or exact unanimous active categorical value.

    This does not project the weighted XYZ onto the authored surface. Normals
    are not normalized implicitly. Native deltas and saved keys stay distinct.
    """
    ids, weights = generalized_source_weights(rows)
    # Object conversion preserves caller list scalar types before NumPy can
    # coerce mixed integers/strings to apparently agreeing string labels.
    values = np.asarray(source_values, dtype=object) if categorical else _generalized_numeric(source_values)
    if values.ndim < 1 or max(ids) >= len(values):
        raise ValueError('Missing donor values')
    selected = values[ids]
    active = [i for i, w in enumerate(weights) if w > 0]
    if categorical:
        def category_signature(row):
            kinds = []
            for value in np.asarray(row, dtype=object).flat:
                if isinstance(value, (bool, np.bool_)):
                    kinds.append('bool')
                elif isinstance(value, (int, np.integer)):
                    kinds.append('int')
                elif isinstance(value, (str, np.str_)):
                    kinds.append('str')
                else:
                    raise ValueError('Explicit categorical values required')
            return kinds
        signature = category_signature(selected[active[0]])
        if any(category_signature(selected[i]) != signature
               or not np.array_equal(selected[i], selected[active[0]]) for i in active):
            raise ValueError('Categorical donor conflict')
        return np.asarray(selected[active[0]], dtype=object).copy()
    if values.dtype.kind != 'f' or not np.isfinite(selected).all():
        raise ValueError('Finite floating continuous source values required')
    with np.errstate(over='ignore', invalid='ignore'):
        result = np.tensordot(np.array([float(w) for w in weights]), selected, axes=(0, 0))
    if not np.isfinite(result).all():
        raise ValueError('Nonfinite derived source transport')
    return result


def generalized_corner_values(rows, *, owner, layer, loop_vertices,
                              loop_owners, loop_values, loop_ids):
    """Transport ONE explicit CORNER side; do not infer it from point weights.

    A verified continuous seam may be represented by a different explicit owner
    in a separate record. This function never silently switches or merges sides.
    """
    ids, _ = generalized_source_weights(rows)
    _generalized_ids([owner])
    loops = _generalized_ids(loop_ids)
    loop_vertices = _generalized_ids(loop_vertices, unique=False)
    loop_owners = _generalized_ids(loop_owners, unique=False)
    if (not isinstance(layer, str) or not layer or len(loops) != len(ids)
            or any(i >= len(loop_vertices) or i >= len(loop_owners) for i in loops)
            or [loop_vertices[i] for i in loops] != ids
            or any(loop_owners[i] != owner for i in loops)):
        raise ValueError('Explicit owner/layer/loop side mismatch')
    loop_rows = [[loop, ratio] for loop, (_, ratio) in zip(loops, rows)]
    return generalized_source_values(loop_rows, loop_values)


def generalized_sparse_readset(expected, actual, operations):
    """Check exact JSON source records and immutable identities, without applying.

    Existing records include caller-owned XYZ, weights and frozen baseline data.
    Every replace has a readset record; births cannot overwrite an existing key.
    Caller must include every consumed boundary/face record, not only mutations.
    """
    def strict_json(value):
        if type(value) is dict:
            if any(type(key) is not str for key in value):
                raise ValueError('JSON record keys must be strings')
            for item in value.values():
                strict_json(item)
        elif type(value) is list:
            for item in value:
                strict_json(item)
        elif type(value) not in (type(None), bool, int, float, str):
            raise ValueError('Expected plain JSON record values')

    def canonical(value):
        strict_json(value)
        return json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False)
    for records in (expected, actual):
        if type(records) is not dict or any(type(k) is not str or not k for k in records):
            raise ValueError('Sparse identities must be nonempty strings')
    canonical(expected)
    for key, record in expected.items():
        if key not in actual or canonical(record) != canonical(actual[key]):
            raise ValueError('Stale sparse source readset: ' + str(key))
    seen = set()
    for op in operations:
        canonical(op)  # Validate the WHOLE supplied payload, not just its binding.
        key = op['identity']
        if not isinstance(key, str) or not key or key in seen:
            raise ValueError('Duplicate or invalid sparse operation identity')
        seen.add(key)
        record = op.get('record', {})
        generalized_source_binding(
            record.get('source_weights_rational', []),
            source_hashes=record.get('source_hashes', {}),
            frozen_field_identity=record.get('frozen_field_identity'))
        if op['operation'] == 'append':
            if key in actual:
                raise ValueError('Birth identity already exists')
        elif op['operation'] == 'replace':
            if key not in expected:
                raise ValueError('Replacement missing readset')
            old = expected[key]
            new = op['record']
            for field in ('frozen_field_identity', 'source_hashes'):
                if field not in old or old[field] != new.get(field):
                    raise ValueError('Replacement rebases immutable source fields')
        else:
            raise ValueError('Unsupported sparse operation')
    return hashlib.sha256(canonical(expected).encode()).hexdigest()


def generalized_fixed_quad_triangles(quads, triangles, triangle_faces):
    """Check complete oriented two-triangle partition; geometry remains separate."""
    if len(triangles) != 2 * len(quads) or len(triangle_faces) != len(triangles):
        raise ValueError('Incomplete fixed triangle coverage')
    _generalized_ids(triangle_faces, unique=False)
    if len({tuple(sorted(t)) for t in triangles}) != len(triangles):
        raise ValueError('Duplicate fixed triangles')
    groups = [[] for _ in quads]
    for tri, face in zip(triangles, triangle_faces):
        if face >= len(quads) or len(tri) != 3:
            raise ValueError('Invalid triangle owner')
        groups[face].append(_generalized_ids(tri))
    for quad, group in zip(quads, groups):
        q = _generalized_ids(quad)
        if len(q) != 4 or len(group) != 2 or any(not set(t) <= set(q) for t in group):
            raise ValueError('Invalid quad partition')
        edges = Counter((t[i], t[(i + 1) % 3]) for t in group for i in range(3))
        for i in range(4):
            edge = (q[i], q[(i + 1) % 4])
            if edges[edge] != 1:
                raise ValueError('Partition changes boundary winding')
            del edges[edge]
        if len(edges) != 2 or any(n != 1 or edges[(b, a)] != 1
                                  for (a, b), n in list(edges.items())):
            raise ValueError('Invalid fixed internal diagonal')
    return True


def generalized_fixed_triangle_poses(neutral, poses, triangles):
    """Check supplied states only, with fixed triangles and original direction gate.

    Caller supplies all required states/representations; this is not a claim
    that two poses imply 729 validity, paired export, dense shape or acceptance.
    """
    if _generalized_numeric(neutral).dtype.kind not in 'fiu':
        raise ValueError('Numeric nonboolean geometry required')
    neutral = np.asarray(neutral, dtype=float)
    if neutral.ndim != 2 or neutral.shape[1] != 3 or not np.isfinite(neutral).all():
        raise ValueError('Invalid neutral geometry')
    for tri in triangles:
        if len(tri) != 3 or max(_generalized_ids(tri)) >= len(neutral):
            raise ValueError('Invalid fixed triangle')
    if not len(triangles) or not len(poses):
        raise ValueError('Missing triangles or pose states')
    def normals(points):
        t = points[np.asarray(triangles)]
        with np.errstate(over='ignore', invalid='ignore'):
            result = np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0])
        if not np.isfinite(result).all():
            raise ValueError('Nonfinite derived triangle normals')
        return result
    reference = normals(neutral)
    minimum = 1.
    for points in poses:
        if _generalized_numeric(points).dtype.kind not in 'fiu':
            raise ValueError('Numeric nonboolean pose geometry required')
        points = np.asarray(points, dtype=float)
        if points.shape != neutral.shape or not np.isfinite(points).all():
            raise ValueError('Invalid pose geometry')
        n = normals(points)
        with np.errstate(over='ignore', invalid='ignore'):
            reference_length = np.linalg.norm(reference, axis=1)
            length = np.linalg.norm(n, axis=1)
            product = reference_length * length
            dot = np.einsum('ij,ij->i', reference, n)
        if any(not np.isfinite(v).all() for v in
               (reference_length, length, product, dot)):
            raise ValueError('Nonfinite derived triangle orientation')
        if np.any(product <= 1e-20) or np.any(dot <= 0):
            raise ValueError('Fixed triangle degeneracy or reversal')
        cosine = dot / product
        if not np.isfinite(cosine).all() or np.any(cosine <= 0):
            raise ValueError('Nonpositive or nonfinite derived relative cosine')
        minimum = min(minimum, float(np.min(cosine)))
    return {'states_checked': len(poses), 'minimum_relative_cosine': minimum}

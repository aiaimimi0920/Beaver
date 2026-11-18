"""Private prototype: exact-vector, immutable-epoch, bounded Jacobian cache."""
from collections import OrderedDict as ForeheadExactOrderedDict
import hashlib as forehead_exact_hashlib
import json as forehead_exact_json
import numpy as forehead_exact_np
from scipy.sparse import csr_matrix as forehead_exact_csr

def forehead_exact_epoch_digest(value):
    """Canonical payload plus typed array paths, with no retained digest cache.

    JSON's C encoder avoids the second 400k-node tagged tree. A plain
    container snapshot preserves traversal-time semantics. Arrays become null
    only in the payload; their exact paths,
    dtype, shape and byte hashes are recorded separately, so ordinary nulls
    or crafted dictionaries cannot impersonate an array. Every dependency is
    visited anew on every request. Graph/source validation is unchanged.
    """
    import math
    arrays = []
    path = []

    def normalize(v):
        if isinstance(v, forehead_exact_np.ndarray):
            if (type(v) is not forehead_exact_np.ndarray or v.dtype.metadata is not None
                    or v.dtype.hasobject or v.dtype.fields is not None
                    or v.dtype.kind not in 'biufc'):
                raise ValueError('Only plain numeric epoch arrays are supported')
            # ascontiguousarray promotes 0-D arrays to shape (1,). Preserve the
            # source shape separately so a scalar-to-vector change invalidates.
            shape = list(v.shape)
            a = forehead_exact_np.ascontiguousarray(v)
            arrays.append([list(path), a.dtype.str, shape,
                           forehead_exact_hashlib.sha256(a.tobytes()).hexdigest()])
            return None
        if isinstance(v, dict):
            if type(v) is not dict or not all(type(k) is str for k in v):
                raise ValueError('Epoch mappings require plain dictionaries and string keys')
            out = {}
            keys = sorted(v)
            for key in keys:
                path.append(key)
                child = v[key]
                encoded = normalize(child)
                path.pop()
                out[key] = encoded
            return out
        if isinstance(v, (list, tuple)):
            if type(v) not in (list, tuple):
                raise ValueError('Epoch sequences require plain lists or tuples')
            out = []
            for index, item in enumerate(v):
                path.append(index)
                encoded = normalize(item)
                path.pop()
                out.append(encoded)
            return out
        if v is None or type(v) in (str, bool, int):
            return v
        if type(v) is float:
            if not math.isfinite(v):
                raise ValueError('Nonfinite epoch scalar')
            return v
        raise ValueError('Unsupported epoch state')

    payload = forehead_exact_json.dumps(normalize(value),
        separators=(',', ':'), allow_nan=False)
    typed_arrays = forehead_exact_json.dumps(arrays, separators=(',', ':'), allow_nan=False)
    # The version prefix distinguishes this in-memory epoch format. Existing
    # fits never cross formats or share caches, so old persisted IDs are unused.
    digest = forehead_exact_hashlib.sha256(b'forehead-epoch-v2\0')
    digest.update(payload.encode())
    digest.update(b'\0')
    digest.update(typed_arrays.encode())
    return digest.hexdigest()

def forehead_exact_fit_cache(derivative, epoch, charge, rows, columns, capacity=2):
    if type(capacity) is not int or capacity != 2:
        raise ValueError('Invalid bounded cache capacity')
    frozen = forehead_exact_epoch_digest(epoch())
    cache = ForeheadExactOrderedDict()
    metadata = {'requests': 0, 'hits': 0, 'misses': 0, 'capacity': capacity, 'epoch': frozen, 'invalidated': False}

    def require_epoch():
        try:
            valid = not metadata['invalidated'] and forehead_exact_epoch_digest(epoch()) == frozen
        except Exception:
            cache.clear()
            metadata['invalidated'] = True
            raise
        if not valid:
            cache.clear()
            metadata['invalidated'] = True
            raise ValueError('Per-fit derivative epoch changed; new fit required')

    def request(values):
        charge('exact_fit_jacobian_request')
        metadata['requests'] += 1
        require_epoch()
        x = forehead_exact_np.asarray(values, dtype=forehead_exact_np.float64)
        if x.shape != (columns,) or not forehead_exact_np.isfinite(x).all():
            raise ValueError('Invalid derivative parameters')
        key = forehead_exact_np.ascontiguousarray(x).tobytes()
        if key in cache:
            metadata['hits'] += 1
            cache.move_to_end(key)
            return cache[key].copy()
        metadata['misses'] += 1
        try:
            result = forehead_exact_csr(derivative(x.copy()), copy=True)
            require_epoch()
            if result.shape != (rows, columns) or not forehead_exact_np.isfinite(result.data).all():
                raise ValueError('Invalid cached derivative')
        except Exception:
            cache.clear()
            metadata['invalidated'] = True
            raise
        cache[key] = result.copy()
        if len(cache) > capacity:
            cache.popitem(last=False)
        return result
    return (request, metadata)

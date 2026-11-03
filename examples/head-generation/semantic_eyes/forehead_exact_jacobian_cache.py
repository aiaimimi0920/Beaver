"""Private prototype: exact-vector, immutable-epoch, bounded Jacobian cache."""
from collections import OrderedDict as ForeheadExactOrderedDict
import hashlib as forehead_exact_hashlib
import json as forehead_exact_json
import numpy as forehead_exact_np
from scipy.sparse import csr_matrix as forehead_exact_csr

def forehead_exact_epoch_digest(value):
    """Only explicit owned state; callers must enumerate all derivative dependencies."""

    def pack(v):
        if isinstance(v, forehead_exact_np.ndarray):
            if v.dtype.hasobject or v.dtype.fields is not None or v.dtype.kind not in 'biufc':
                raise ValueError('Only plain numeric epoch arrays are supported')
            a = forehead_exact_np.ascontiguousarray(v)
            return ['array', a.dtype.str, list(a.shape), forehead_exact_hashlib.sha256(a.tobytes()).hexdigest()]
        if isinstance(v, dict):
            if not all((isinstance(k, str) for k in v)):
                raise ValueError('Epoch keys must be strings')
            return ['dict', [[k, pack(v[k])] for k in sorted(v)]]
        if isinstance(v, (list, tuple)):
            return ['sequence', [pack(x) for x in v]]
        if v is None or type(v) in (str, bool, int, float):
            return [type(v).__name__, v]
        raise ValueError('Unsupported epoch state')
    return forehead_exact_hashlib.sha256(forehead_exact_json.dumps(pack(value), separators=(',', ':'), allow_nan=False).encode()).hexdigest()

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

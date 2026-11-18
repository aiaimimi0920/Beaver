"""Fixed positive row scaling from the existing unscaled hybrid Jacobian."""
import numpy as np
from scipy.sparse import csr_matrix, diags


def forehead_prepare_hybrid_scaling(derivative, start, scales):
    """Bind scaling once, without changing raw constraints or admission gates.

    The derivative closure must reference the supplied all-one scale array.
    Compute its initial unscaled matrix once, then freeze positive row norms.
    Reuse that initial matrix only at the identical parameter vector; returned
    cache copies cannot corrupt later optimizer requests. No old fit is reused.
    """
    start = np.asarray(start, float).copy()
    if (start.ndim != 1 or not len(start) or not np.isfinite(start).all()
            or not isinstance(scales, np.ndarray) or scales.ndim != 1
            or not len(scales) or not np.issubdtype(scales.dtype, np.floating)
            or not scales.flags.writeable or not np.all(scales == 1.)):
        raise ValueError('Hybrid scaling requires a finite start and owned unit scales')
    raw = csr_matrix(derivative(start), copy=True)
    if raw.shape != (len(scales), len(start)) or not np.isfinite(raw.data).all():
        raise ValueError('Invalid initial unscaled hybrid Jacobian')
    squared = np.asarray(raw.multiply(raw).sum(axis=1)).ravel()
    proposed = np.maximum(1., np.sqrt(squared)).astype(scales.dtype)
    if not np.isfinite(proposed).all() or np.any(proposed <= 0):
        raise ValueError('Nonfinite or nonpositive fixed row scales')
    initial = (diags(1/proposed) @ raw).tocsr()
    scales[:] = proposed
    scales.setflags(write=False)
    key = start.tobytes()
    metadata = {'method': 'initial unscaled hybrid Jacobian row norms',
                'fixed_positive_scaling': True, 'raw_constraints_unchanged': True,
                'row_norm_floor': 1., 'constraint_count': len(scales),
                'parameter_count': len(start), 'initial_nonzeros': raw.nnz,
                'minimum_scale': float(scales.min()),
                'maximum_scale': float(scales.max()), 'initial_cache_hits': 0}

    def scaled_derivative(values):
        values = np.asarray(values, float)
        if values.shape != start.shape or not np.isfinite(values).all():
            raise ValueError('Invalid scaled-Jacobian parameter vector')
        if values.tobytes() == key:
            metadata['initial_cache_hits'] += 1
            return initial.copy()
        return derivative(values)

    return scaled_derivative, metadata

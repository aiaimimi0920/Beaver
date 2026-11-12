"""Same constraints, hybrid derivative: base sparse FD plus verified smooth gap blocks."""
from forehead_matched_gap_jacobian import forehead_gap_source_differential, forehead_matched_gap_jacobian
import numpy as np
from scipy.sparse import vstack as forehead_gap_vstack, diags as forehead_gap_diags
from scipy.optimize._numdiff import approx_derivative as forehead_gap_approx_derivative, group_columns as forehead_gap_group_columns

def forehead_make_hybrid_gap_jacobian(base_constraints, evaluate, author_parameters, batch, ids, stage, offset, scales, sparsity, base_derivative=None):
    base_pattern = sparsity[:offset].tocsr()
    base_groups = forehead_gap_group_columns(base_pattern) if base_derivative is None else None
    gap_pattern = sparsity[offset:].tocsr()
    stats = {'base_colors': int(max(base_groups) + 1) if base_groups is not None else None, 'complete_split_base': base_derivative is not None, 'calls': 0, 'fallback_rows_total': 0, 'analytic_rows_total': 0, 'invalid_atlas_fallbacks': 0, 'flag_reasons': {}}

    def gap_values(values):
        result = evaluate(values)
        if result is None and base_derivative is not None:
            raise ValueError('Invalid source probe in complete-guide fit')
        return np.full(stage.size, -1000.0) if result is None else stage.constraints(result[0], author_parameters(values))

    def derivative(values):
        stats['calls'] += 1
        base = (base_derivative(values) if base_derivative is not None else
                forehead_gap_approx_derivative(lambda x: base_constraints(x) / scales[:offset], values, method='2-point', rel_step=1e-08, sparsity=(base_pattern, base_groups)))
        try:
            result = evaluate(values)
            if result is None:
                raise ValueError('Invalid author sample')
            points = result[0]
            params = author_parameters(values)
            coords = np.asarray(values).reshape(-1, 2) / 1000
            sampled, world_jac, boundary = forehead_gap_source_differential(batch, coords)
            if not np.array_equal(sampled, points[ids]):
                raise ValueError('Derivative sampler differs from primal')
            block, gaps, fallback, reasons = forehead_matched_gap_jacobian(stage, points, params, ids, world_jac, boundary)
            actual = stage.measure(points, params)
            if actual is None or not np.allclose(actual, gaps, rtol=0, atol=1e-09):
                raise ValueError('Analytic metric differs from primal')
            for reason, count in reasons.items():
                stats['flag_reasons'][reason] = stats['flag_reasons'].get(reason, 0) + count
        except (ValueError, KeyError, np.linalg.LinAlgError):
            if base_derivative is not None:
                raise
            stats['invalid_atlas_fallbacks'] += 1
            fallback = np.ones(stage.size, bool)
            block = None
        selected = np.flatnonzero(fallback)
        stats['fallback_rows_total'] += len(selected)
        stats['analytic_rows_total'] += stage.size - len(selected)
        if len(selected):
            pattern = gap_pattern[selected]
            groups = forehead_gap_group_columns(pattern)
            numerical = forehead_gap_approx_derivative(lambda x: gap_values(x)[selected], values, method='2-point', rel_step=1e-08, sparsity=(pattern, groups))
            if block is None:
                block = numerical.tocsr()
            else:
                block = block.tolil()
                block[selected] = numerical
                block = block.tocsr()
        return forehead_gap_vstack((base, forehead_gap_diags(1 / scales[offset:]) @ block), format='csr')
    return (derivative, stats)

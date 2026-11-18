"""Separate old rows and complete-guide fallback; never color partial masks."""
import numpy as np
from scipy.sparse import csr_matrix as forehead_complete_csr
from scipy.optimize._numdiff import approx_derivative as forehead_complete_fd, group_columns as forehead_complete_colors
from forehead_complete_derivative_adapter import forehead_complete_analytic_constraints


def forehead_complete_fallback_block(stage, values, evaluate, analytic, fallback, dependencies):
    block = np.asarray(analytic).copy()
    rows = np.flatnonzero(fallback.any(axis=1))
    columns = np.flatnonzero(fallback.any(axis=0))
    if not len(rows):
        return block
    # Full dependencies within the perturbed union prevent analytic-column cross-talk.
    pattern = forehead_complete_csr(dependencies[np.ix_(rows, columns)])
    groups = forehead_complete_colors(pattern)
    def residual(reduced):
        stage.charge('complete_constraint_fallback_probe')
        sample = np.asarray(values).copy()
        sample[columns] = reduced
        state = evaluate(sample)
        if state is None:
            raise ValueError('Complete-guide constraint probe outside source')
        return stage.constraints(state[0])[rows]
    numerical = forehead_complete_fd(residual, np.asarray(values)[columns], method='2-point',
                                     rel_step=1e-8, sparsity=(pattern, groups)).toarray()
    for i, row in enumerate(rows):
        selected = fallback[row, columns]
        block[row, columns[selected]] = numerical[i, selected]
    if not np.isfinite(block).all():
        raise ValueError('Nonfinite complete-guide fallback differential')
    return block


def forehead_complete_base_derivative(stage, base_without_complete, evaluate, batch, ids,
                                       interval, scales, base_pattern):
    if stage is None:
        return None
    first, last = interval
    other = np.r_[np.arange(first), np.arange(last, base_pattern.shape[0])]
    def derivative(values):
        stage.charge('complete_split_base_derivative_wrapper')
        # Rebuild every call. Supplemental masks may change with source/active branches.
        pattern = base_pattern[other].tocsr()
        groups = forehead_complete_colors(pattern)
        def residual(sample):
            stage.charge('other_base_constraint_probe')
            if evaluate(sample) is None:
                raise ValueError('Other constraint probe outside source')
            return base_without_complete(sample)[other]/scales[other]
        numerical = forehead_complete_fd(residual, values, method='2-point', rel_step=1e-8,
                                        sparsity=(pattern, groups))
        _, analytic, fallback, dependencies, _ = forehead_complete_analytic_constraints(stage, values, evaluate, batch, ids)
        supplemental = forehead_complete_fallback_block(stage, values, evaluate, analytic, fallback, dependencies)
        out = forehead_complete_csr((len(scales), len(values))).tolil()
        out[other] = numerical
        out[first:last] = supplemental/scales[first:last, None]
        return out.tocsr()
    return derivative

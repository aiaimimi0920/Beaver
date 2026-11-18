"""Nonzero boundary provenance before numerical target-weight pruning."""


def target_has_boundary_dependency(path, fractions, contenders, index, boundary):
    """Use the caller's existing contender set, without changing any tolerances.

    The analytic row may retain its original tiny-weight approximation. This
    predicate separately tells the full-primal numerical fallback consumer that
    a boundary variable remains a true nonzero endpoint dependency.
    """
    for edge in contenders:
        fraction = fractions[edge]
        for node, weight in ((int(path[edge]), 1-fraction),
                             (int(path[edge+1]), fraction)):
            column = index.get(node)
            if weight != 0 and column is not None and boundary[column]:
                return True
    return False

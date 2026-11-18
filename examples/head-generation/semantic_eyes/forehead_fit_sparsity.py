"""Complete source-parameter incidence for the active shared-fit Jacobian.

Dense supplemental guide/support rows include every relevant source variable;
existing analytical fairing rows remain separately installed by the fit.
"""


def forehead_fit_constraint_sparsity(layout, parameter_count, ids, quads, lower_triples,
                                     flow_stage, fairing_stage, complete_stage,
                                     paired_stage, anchor_domain, gap_stage):
    from scipy.sparse import lil_matrix
    nc = layout['count']
    sparse = lil_matrix((nc + parameter_count, parameter_count), dtype=int)
    index = {i: j for j, i in enumerate(ids)}

    def mark(rows, nodes):
        columns = [2 * index[i] + d for i in nodes if i in index for d in (0, 1)]
        for row in rows:
            sparse[row, columns] = 1
    Q = len(quads)
    for k, quad in enumerate(quads):
        rows = list(range(layout['aspect'][0] + 12*k, layout['aspect'][0] + 12*k + 12))
        for name in ('angle_lower', 'angle_upper', 'orientation'):
            rows.extend(range(layout[name][0] + 4 * k, layout[name][0] + 4 * k + 4))
        mark(rows, quad)
    for k, i in enumerate(ids):
        mark([layout['field'][0] + k], [i])
    for k, triple in enumerate(lower_triples):
        mark([layout['lower_turn'][0] + k], triple)
    if flow_stage:
        for a, b, nodes in flow_stage.groups:
            mark(range(layout['physical_flow'][0] + a, layout['physical_flow'][0] + b), nodes)
    if fairing_stage:
        for k, triple in enumerate(fairing_stage.triples):
            mark([layout['whole_guide_fairing'][0]+k], triple)
        # Dense energy row is replaced analytically, not finite-difference colored.
    if complete_stage:
        first, last = layout['complete_guide_fairing']
        mark(range(first, last), complete_stage.nodes)
    for a, b, nodes in paired_stage.groups:
        mark(range(layout['paired_diagonal'][0]+a, layout['paired_diagonal'][0]+b), nodes)
    for row, (_, _, first, last) in enumerate(anchor_domain.specs):
        mark([layout['anchor_domain'][0] + row], [first, last])
    for a, b, nodes in gap_stage.groups:
        mark(range(layout['matched_gap'][0] + a, layout['matched_gap'][0] + b), nodes)
    for k in range(parameter_count):
        sparse[nc + k, k] = 1
    return sparse

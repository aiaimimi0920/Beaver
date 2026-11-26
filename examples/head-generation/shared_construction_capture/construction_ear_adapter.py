"""Opt-in real ear allocation and bounded joined attachment consumer wiring."""
import ast
from construction_instrument import replace
from ear_adapter import instrument_ears
from ear_lower_adapter import instrument_lower


def instrument_ear_sources(sources):
    """Keep all producer math and change only reviewed observation/ID boundaries."""
    out = dict(sources)
    out['ears.py'] = instrument_ears(out['ears.py'])
    source = out['ear_boundary.py']
    nodes = [n for n in ast.parse(source).body
             if isinstance(n, ast.FunctionDef) and n.name == 'fair_lower_ear_boundaries']
    if len(nodes) != 1:
        raise ValueError('Unique original lower ear fairing required')
    function, proof = instrument_lower(source)
    if not proof['reverse_source_exact']:
        raise ValueError('Lower ear identity adapter changed numeric source')
    node = nodes[0]
    lines = source.splitlines(keepends=True)
    out['ear_boundary.py'] = ''.join(lines[:node.lineno-1]) + function + '\n' + ''.join(lines[node.end_lineno:])
    return out


def attachment_boundaries(pipeline):
    """Bounded verified join/ownership/fairing; all later geometry stays unadmitted."""
    pipeline = replace(pipeline,
        '        report["oral_skin_point_ownership"] = restore_oral_skin_donors(members[0],_oral_skin_donors)',
        '        from ear_join import verify_combined_oral_transfer, verify_ear_membership\n'
        '        report["oral_skin_point_ownership"] = verify_combined_oral_transfer(\n'
        '            members[0], _construction_joined, _oral_skin_donors, restore_oral_skin_donors)')
    pipeline = replace(pipeline,
        '        report["ear_shell_membership"] = restore_ear_shell_membership(members[0],_ear_shell_membership)',
        '        report["ear_shell_membership"] = verify_ear_membership(\n'
        '            members[0], _construction_joined, _ear_shell_membership, restore_ear_shell_membership)')
    return replace(pipeline,
        '        report["ear_lower_boundary_fairing"] = fair_lower_ear_boundaries(members[0].data, report["ear_root_fit"])',
        '        from ear_lower_context import prepare_reports\n'
        '        if len(_construction_joined) != 1:\n'
        '            raise ValueError("Exactly one bound ear shell required")\n'
        '        _construction_ear_reports, _construction_ear_context = prepare_reports(\n'
        '            _construction_joined[0], report["ear_root_fit"])\n'
        '        report["ear_lower_boundary_fairing"] = fair_lower_ear_boundaries(members[0].data, _construction_ear_reports)\n'
        '        report["ear_root_fit"] = [{k: v for k, v in row.items()\n'
        '            if k not in ("_identity_bridge_triangle", "_identity_root_indices", "_identity_lower")}\n'
        '            for row in _construction_ear_reports]\n'
        '        from construction_native import invalidate_before_surgery\n'
        '        invalidate_before_surgery(_construction_joined, "unverified_final_ear_support_and_later_surgery")')

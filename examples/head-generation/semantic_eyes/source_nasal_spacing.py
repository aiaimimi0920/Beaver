"""Install semantic five-rail density repair before independent chin refinement."""
import ast
from pathlib import Path
from source_nasal_flow import MODULES as NASAL_MODULES
from source_chin_retile import CHIN_MODULES

SPACING_MODULES = ('nasal_spacing_rule', 'nasal_rear_rule', 'nasal_return_provenance',
                   'orbital_support_rule', 'orbital_support_bind', 'orbital_support_native',
                   'mouth_corner_rule', 'mouth_corner_native', 'mouth_corner_shapes',
                   'forehead_flow_bind', 'forehead_flow_rule', 'forehead_flow_refine', 'forehead_flow_native',
                   'forehead_star_topology',
                   'forehead_retile_roles',
                   'forehead_surface_batch',
                   'forehead_retile_fair',
                   'forehead_retile_support',
                   'forehead_retile_mirror',
                   'forehead_retile_contract',
                   'forehead_retile_fields',
                   'forehead_retile_shape',
                   'forehead_retile_apply',
                   'forehead_retile_mesh',
                   'nasal_spacing_shape',
                   'nasal_spacing_mesh', 'nasal_spacing_joint')


def adapt(sources):
    out = dict(sources)
    pipeline = out['pipeline.py']
    marker = '        from shell import refine_chin_source'
    prior = 'report.update(refine_nasal_source('
    if pipeline.count(marker) != 1 or pipeline.count(prior) != 1:
        raise ValueError('Five-rail spacing must follow nasal core and precede chin')
    if pipeline.index(prior) > pipeline.index(marker):
        raise ValueError('Five-rail spacing source stages changed order')
    out['pipeline.py'] = pipeline.replace(marker,
        '        from shell import refine_nasal_bridge_spacing, capture_nasal_spacing_baseline\n'
        '        _spacing_baseline = capture_nasal_spacing_baseline(data)\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"],\n'
        '            report["nasal_five_flow_source"]["allocated_flow_records"]))\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            generated_return=report["ear_post_return"], baseline=_spacing_baseline))\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            orbital_calibration=C, assembly_offset=report["head_assembly_y_offset_m"]))\n' + marker, 1)
    mouth_marker = '        report.update(refine_chin_source(data,report["chin_source_anchors"]))'
    if out['pipeline.py'].count(mouth_marker) != 1:
        raise ValueError('Mouth corner stage requires unique completed chin source transaction')
    out['pipeline.py'] = out['pipeline.py'].replace(mouth_marker, mouth_marker + '\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            assembly_offset=report["head_assembly_y_offset_m"],\n'
        '            mouth_context={"calibration": C, "chin": report["chin_source_retile"]}))', 1)
    forehead_marker = '            mouth_context={"calibration": C, "chin": report["chin_source_retile"]}))'
    if out['pipeline.py'].count(forehead_marker) != 1:
        raise ValueError('Forehead stage requires unique completed mouth source stage')
    out['pipeline.py'] = out['pipeline.py'].replace(forehead_marker, forehead_marker + '\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            assembly_offset=report["head_assembly_y_offset_m"],\n'
        '            forehead_context={"calibration": C, "report": report}))', 1)
    shell = out['shell.py']
    existing = {node.name for node in ast.parse(shell).body
                if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
    internal = set(NASAL_MODULES) | set(CHIN_MODULES) | set(SPACING_MODULES)
    root = Path(__file__).parent
    chunks = []
    for module in SPACING_MODULES:
        source = (root / (module + '.py')).read_text(encoding='utf-8')
        excluded = set()
        for node in ast.parse(source).body:
            if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
                if node.name in existing:
                    raise ValueError('Five-rail source helper collides: ' + node.name)
                existing.add(node.name)
            if isinstance(node, ast.ImportFrom) and node.module in internal:
                excluded.update(range(node.lineno - 1, node.end_lineno))
        chunks.append(''.join(line for i, line in enumerate(source.splitlines(keepends=True))
                              if i not in excluded))
    out['shell.py'] = shell + '\n\n' + '\n\n'.join(chunks)
    for name, source in out.items():
        if name.endswith('.py'):
            ast.parse(source, filename=name)
    return out

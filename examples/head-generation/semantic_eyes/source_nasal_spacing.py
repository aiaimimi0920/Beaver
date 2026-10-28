"""Install semantic five-rail density repair before independent chin refinement."""
import ast
from pathlib import Path
from source_nasal_flow import MODULES as NASAL_MODULES
from source_chin_retile import CHIN_MODULES

SPACING_MODULES = ('nasal_spacing_rule', 'nasal_rear_rule', 'nasal_return_provenance', 'nasal_spacing_shape',
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
        '            generated_return=report["ear_post_return"], baseline=_spacing_baseline))\n' + marker, 1)
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

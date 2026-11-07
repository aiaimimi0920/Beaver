"""Integrate current-build lower-mouth flow before automatic runtime export."""
import ast
from pathlib import Path
from source_nasal_flow import MODULES as NASAL_MODULES

CHIN_MODULES = ('chin_source_roles', 'chin_source_fields', 'chin_source_contract',
                'chin_source_template', 'chin_source_solver', 'chin_source_shape', 'chin_source_mesh',
                'chin_joint_mesh')


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']

    def replace(old, new):
        nonlocal shell
        if shell.count(old) != 1:
            raise ValueError('Lower-mouth source capture hook changed: ' + old[:100])
        shell = shell.replace(old, new, 1)

    replace('    shell_faces = []', '    shell_faces = []\n    chin_source_units = []')
    call = ('            shell_faces.extend(feature_chin_front_strip(verts, j, levels, cols, '
            'lambda y, col: feature_flow_grid_point(y, col, lateral, nominal)))')
    replace(call, '            chin_unit_start = len(verts)\n' + call + '\n'
            '            chin_source_units.extend(tuple(range(i,i+3)) '
            'for i in range(chin_unit_start,len(verts),3))')
    marker = '    nasal_source_anchors = capture_nasal_source_anchors(verts,levels,lateral,cols)'
    replace(marker, marker + '\n'
            '    chin_source_anchors = capture_chin_source_roles(verts,shell_faces,\n'
            '        [[r*cols+i for i in feature_chin_row_columns(r,cols)] for r in range(4)],\n'
            '        chin_source_units,outer_flow_report,mouth_patch_fields,chin_retile_recipe())')
    marker = '        "nasal_source_anchors": nasal_source_anchors,'
    replace(marker, marker + '\n        "chin_source_anchors": chin_source_anchors,')
    pipeline = out['pipeline.py']
    marker = ('        report.update(refine_nasal_source(data,semantic_part_ordinals,'
              'report["nasal_source_anchors"]))')
    if pipeline.count(marker) != 1:
        raise ValueError('Lower-mouth transaction must follow the current nasal operation')
    out['pipeline.py'] = pipeline.replace(marker, marker + '\n'
        '        from shell import refine_chin_source\n'
        '        report.update(refine_chin_source(data,report["chin_source_anchors"]))', 1)
    root = Path(__file__).parent
    existing = {node.name for node in ast.parse(shell).body
                if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
    internal = set(CHIN_MODULES) | set(NASAL_MODULES) | {'mouth_patch_geometry'}
    chunks = []
    for module in CHIN_MODULES:
        source = (root / (module + '.py')).read_text(encoding='utf-8')
        excluded = set()
        for node in ast.parse(source).body:
            if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
                if node.name in existing:
                    raise ValueError('Lower-mouth source helper collides: ' + node.name)
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

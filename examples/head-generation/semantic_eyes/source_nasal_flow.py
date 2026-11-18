"""Capture current source anchors and replace only the five-flow authoring patch."""
import ast
from pathlib import Path

MODULES = ('nasal_source_geometry','nasal_source_template','nasal_source_domain',
           'nasal_source_core','nasal_source_binding','nasal_source_boundary',
           'nasal_parameter_adaptation','nasal_source_mesh',
           'nasal_regularization_chart','nasal_regularization_trace',
           'nasal_regularization_solver','nasal_joint_contract','nasal_regularization_mesh',
           'nasal_inner_rule','nasal_inner_adaptation','nasal_inner_mesh','nasal_mesh_snapshot','nasal_joint_mesh')


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    marker = '    flow_chains = [chin_columns, rear_ids, outline_trace["vertex_ids"]]'
    report = '        "whole_mouth_patch_sampling": mouth_patch_sampling_report,'
    if shell.count(marker)!=1 or shell.count(report)!=1:
        raise ValueError('Nasal anchor capture must follow current mouth sampling')
    shell = shell.replace(marker,
        '    nasal_source_anchors = capture_nasal_source_anchors(verts,levels,lateral,cols)\n'+marker,1)
    out['shell.py'] = shell.replace(report,report+'\n        "nasal_source_anchors": nasal_source_anchors,',1)
    pipeline = out['pipeline.py']
    marker = '        # Automatic runtime conversion: no neutral-shape edits follow this step.'
    ear = 'report["feature_flow_ear_source"] = retopologize_flow_ears('
    if pipeline.count(marker)!=1 or pipeline.count(ear)!=1 or pipeline.index(ear)>pipeline.index(marker):
        raise ValueError('Nasal source replacement must precede export and follow ear assembly')
    out['pipeline.py'] = pipeline.replace(marker,
        '        from shell import refine_nasal_source\n'
        '        report.update(refine_nasal_source(data,semantic_part_ordinals,report["nasal_source_anchors"]))\n'+marker,1)
    root = Path(__file__).parent
    existing = {n.name for n in ast.parse(shell).body if isinstance(n,(ast.FunctionDef,ast.ClassDef))}
    chunks = []
    for module in MODULES:
        name = module+'.py'
        source = (root/name).read_text(encoding='utf-8')
        lines = source.splitlines(keepends=True)
        excluded = set()
        for node in ast.parse(source).body:
            if isinstance(node,(ast.FunctionDef,ast.ClassDef)):
                if node.name in existing:
                    raise ValueError('Nasal source helper name collides: '+node.name)
                existing.add(node.name)
            if isinstance(node,ast.ImportFrom) and node.module in MODULES:
                excluded.update(range(node.lineno-1,node.end_lineno))
        chunks.append(''.join(line for i,line in enumerate(lines) if i not in excluded))
    # Use the existing ordered shared-frame packaging contract, so runtime
    # loading and replay see precisely the same functions without new modules
    # silently omitted by the checkpoint's explicit loader registry.
    out['shell.py'] += '\n\n'+'\n\n'.join(chunks)
    for name,source in out.items():
        if name.endswith('.py'):
            ast.parse(source,filename=name)
    return out

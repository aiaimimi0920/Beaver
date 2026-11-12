"""Extract exact source functions; add only a reversible observational callback."""
import ast
import hashlib
from pathlib import Path

HERE=Path(__file__).resolve().parent
SEMANTIC=HERE.parents[1]/'semantic_eyes'


def source_functions(path,names):
    source=path.read_text();lines=source.splitlines(keepends=True);found={}
    for node in ast.parse(source).body:
        if isinstance(node,ast.FunctionDef) and node.name in names:
            found[node.name]=''.join(lines[node.lineno-1:node.end_lineno])
    if set(found)!=set(names):raise ValueError('Missing source function')
    return '\n\n'.join(found[name] for name in names)+'\n'


def source_pair():
    original=source_functions(SEMANTIC/'ear_final_mesh.py',('ear_loop_values','assign_ear_loop'))
    original+=source_functions(SEMANTIC/'ear_flow_geometry.py',('flow_chain_weights','flow_ruled_weights'))
    original+=source_functions(SEMANTIC/'ear_flow_mesh.py',
                               ('flow_point_fields','flow_new_point','flow_ordered_quad','flow_reconnect_skin'))
    original+='\nimport numpy as np\nfrom types import SimpleNamespace\n'
    original+=source_functions(SEMANTIC/'source_ear_quad_solver.py',('ear_quad_least_squares','ear_quad_measures'))
    original+=source_functions(SEMANTIC/'ear_return_cap_solver.py',('solve_return_cap','solve_return_cap_with_seed'))
    original+=source_functions(SEMANTIC/'ear_final_caps.py',('check_stored_cap_neighbors','retopologize_source_return_caps'))
    anchor='    bm.faces.remove(lower); bm.faces.remove(upper)\n'
    replacement="    _observe_joint(locals())\n"+anchor
    if original.count(anchor)!=1:raise ValueError('Source boundary drift')
    observed=original.replace(anchor,replacement,1)
    edits=[(anchor,replacement)]
    for old,new in [
        ("        for i,point in zip(ids,result['vertices']):\n",
         "        _observe_cap('before',locals())\n        for i,point in zip(ids,result['vertices']):\n"),
        ('        bm.faces.remove(first);bm.faces.remove(second)\n',
         '        _observe_cap("allocated",locals())\n        bm.faces.remove(first);bm.faces.remove(second)\n')]:
        if observed.count(old)!=1:raise ValueError('Return-cap source boundary drift')
        observed=observed.replace(old,new,1);edits.append((old,new))
    restored=observed
    for old,new in reversed(edits):restored=restored.replace(new,old,1)
    if restored!=original:raise ValueError('Non-observer source change')
    for name,text in [('baseline',original),('observed',observed)]:compile(text,name+'.py','exec')
    return original,observed

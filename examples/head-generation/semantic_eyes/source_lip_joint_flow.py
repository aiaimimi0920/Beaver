"""Compose the semantic joint ring solver without weakening source depth guards."""
import ast
import inspect
from lip_joint_solver import refine_lip_support


def adapt(sources):
    out = dict(sources)
    text = out['face.py']
    tree = ast.parse(text)
    candidates = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'depth']
    if len(candidates) != 1:
        raise ValueError('Joint lip depth contract changed')
    node = candidates[0]
    body = node.body
    original = [n for n in body if isinstance(n, ast.Assign) and
                any(isinstance(t, ast.Name) and t.id == 'original' for t in n.targets)]
    result = [n for n in body if isinstance(n, ast.Assign) and
              any(isinstance(t, ast.Name) and t.id == 'result' for t in n.targets)]
    if len(original) != 1 or len(result) != 1:
        raise ValueError('Depth terms must have one raw result and original reference')
    split = body.index(result[0])
    prefix = body[:split + 1]
    if any(isinstance(n, (ast.If, ast.Raise, ast.Return)) for n in prefix):
        raise ValueError('Depth term prefix contains guard/control flow')
    pure = ast.FunctionDef(name='lip_depth_terms',
                          args=ast.arguments(posonlyargs=[], args=[ast.arg(arg='x'), ast.arg(arg='y')],
                                             kwonlyargs=[], kw_defaults=[], defaults=[]),
                          body=prefix + [original[0], ast.Return(value=ast.Tuple(
                              elts=[ast.Name(id='result', ctx=ast.Load()),
                                    ast.Name(id='original', ctx=ast.Load())], ctx=ast.Load()))],
                          decorator_list=[])
    ast.fix_missing_locations(pure)
    lines = text.splitlines(keepends=True)
    start, end = prefix[0].lineno - 1, prefix[-1].end_lineno
    lines[start:end] = ['    result, original = lip_depth_terms(x, y)\n']
    # Remove only the now-duplicated reference assignment; all strict guards stay.
    rewritten = ''.join(lines)
    reference = ast.get_source_segment(text, original[0])
    if rewritten.count('    ' + reference) != 1:
        raise ValueError('Original depth reference changed')
    rewritten = rewritten.replace('    ' + reference + '\n', '', 1)
    out['face.py'] = (rewritten + '\n\n' + ast.unparse(pure) + '\n\n'
                      + inspect.getsource(refine_lip_support) + '\n')
    anchor = '        rings.append(boundary)\n'
    if out['shell.py'].count(anchor) != 1:
        raise ValueError('Joint lip ring assembly contract changed')
    out['shell.py'] = out['shell.py'].replace(anchor, anchor +
        '        if kind == "mouth":\n'
        '            lip_joint_reports.append(refine_lip_support(verts, rings, mouth_angles, lip_source_ring_fraction,\n'
        '                lip_depth_terms, depth, H, C,\n'
        '                lambda x: Y0 + H*C["upper_lip_height"] + mouth_profile_shift(x)))\n', 1)
    patch = '    def patch(boundary, kind, closed):'
    if out['shell.py'].count(patch) != 1:
        raise ValueError('Mouth patch function contract changed')
    out['shell.py'] = out['shell.py'].replace(patch, '    lip_joint_reports = []\n' + patch, 1)
    marker = '        \"semantic_rows\": levels,'
    if out['shell.py'].count(marker) != 1:
        raise ValueError('Mouth report contract changed')
    out['shell.py'] = out['shell.py'].replace(marker, '        \"joint_lip_support\": lip_joint_reports,\n' + marker, 1)
    compile(out['face.py'], 'face.py', 'exec')
    compile(out['shell.py'], 'shell.py', 'exec')
    return out

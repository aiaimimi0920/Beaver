"""Guarded private instrumentation of resolved actual ears source, opt-in only."""
import ast
import hashlib
from construction_instrument import function_edit

BEGIN = '''    bpy.context.view_layer.update()
    evaluated = shell.evaluated_get(bpy.context.evaluated_depsgraph_get())
    data = evaluated.to_mesh()'''
OBSERVE = '''    from ear_evaluated import begin_evaluation, close_evaluation, bind_created_ear
    _ear_evaluation = begin_evaluation(shell)
    data = _ear_evaluation["data"]'''
ALLOCATE = '            obj = mesh("Ear hollow pinna " + str(side), vs, fs, SKIN)'
BIND = ALLOCATE + '''
            bind_created_ear(obj, _ear_evaluation, side, roots, rows,
                             lower_neighbors, lower_id, upper_neighbors, upper_id, vs, fs, bridge)'''


def instrument_ears(source):
    """Keep producer math, selectors, source order and all finalization untouched."""
    tree = ast.parse(source)
    functions = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'fit_ear_roots']
    if len(functions) != 1:
        raise ValueError('Missing unique actual ear constructor')
    body = ast.get_source_segment(source, functions[0])
    required = [
        'root_points = [data.vertices[i].co.copy() for i in roots]',
        'lower_id = len(vs)\n            vs.append(tuple(lower))',
        'upper_id = len(vs)\n            vs.append(tuple(upper))',
        'fs.append((upper_id, rows[-1][0], rim[-2]))',
        'rows.append(row)',
        'bridge = lower_ear_bridge(lower_id, rows[0][0], rim[1])',
    ]
    if any(body.count(text) != 1 for text in required):
        raise ValueError('Ear producer/root/bilateral bridge source boundary changed')
    changed = function_edit(source, 'fit_ear_roots', [
        (BEGIN, OBSERVE), (ALLOCATE, BIND),
        ('        evaluated.to_mesh_clear()', '        close_evaluation(_ear_evaluation)'),
    ])
    ast.parse(changed)
    return changed


def instrument_with_proof(source):
    transformed = instrument_ears(source)
    # Exact reverse substitution proves that only observer boundaries changed.
    restored = transformed.replace(OBSERVE, BEGIN).replace(BIND, ALLOCATE).replace(
        '        close_evaluation(_ear_evaluation)', '        evaluated.to_mesh_clear()')
    if restored != source:
        raise ValueError('Ear source observer changed unlisted producer text')
    sha = lambda text: hashlib.sha256(text.encode()).hexdigest()
    return transformed, {'source_sha256': sha(source), 'instrumented_sha256': sha(transformed),
                         'exact_reverse_equals_original': True,
                         'geometry_fields_and_modifier_apply_timing_changed': False,
                         'native_execution_validated': False}

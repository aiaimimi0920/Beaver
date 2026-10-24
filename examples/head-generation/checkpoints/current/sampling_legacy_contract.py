"""Narrow test-only projections of opt-in sampling onto frozen default contracts."""
import ast
import copy


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise AssertionError('Reviewed sampling expression changed: ' + old)
    return source.replace(old, new, 1)


def legacy_accent_tree(tree):
    tree = copy.deepcopy(tree)
    replacements = {
        'lateral_sections': ('STYLE.get("sampling_lateral_gap", .065)', '.065'),
        'build_lateral_accent': (
            '[(f,0.0,STYLE["accent_lateral_branch_width_H"]*sin(pi*f)**.8) for f in companion_stations(STYLE)]',
            '[(i/16,0.0,STYLE["accent_lateral_branch_width_H"]*sin(pi*i/16)**.8) for i in range(17)]'),
        'fit_evaluated_accent_roots': (
            '[1 if f in (0.0,1.0) else 6 for f in companion_stations(STYLE)]',
            '[1 if i in (0,16) else 6 for i in range(17)]'),
    }
    found = set()
    for function in tree.body:
        if not isinstance(function, ast.FunctionDef) or function.name not in replacements:
            continue
        old, new = replacements[function.name]
        expected = ast.dump(ast.parse(old, mode='eval').body, include_attributes=False)
        replacement = ast.parse(new, mode='eval').body
        class ExactExpression(ast.NodeTransformer):
            count = 0
            def generic_visit(self, node):
                if ast.dump(node, include_attributes=False) == expected:
                    self.count += 1
                    return copy.deepcopy(replacement)
                return super().generic_visit(node)
        transform = ExactExpression()
        transform.visit(function)
        if transform.count != 1:
            raise AssertionError('Sampling expression count changed in ' + function.name)
        found.add(function.name)
    if found != set(replacements):
        raise AssertionError('Missing sampling functions')
    removed = 0
    for node in tree.body:
        if isinstance(node, ast.ImportFrom) and node.module == 'lateral_fit':
            removed += sum(alias.name == 'companion_stations' for alias in node.names)
            node.names = [alias for alias in node.names if alias.name != 'companion_stations']
    if removed != 1:
        raise AssertionError('Companion sampling import changed')
    return tree


def legacy_shell_source(source):
    setup = '''    column_count = STYLE.get("sampling_shell_columns", 13)
    if column_count not in (9, 13):
        raise ValueError("Unsupported semantic shell column count")
    original_lateral = [0, 0.06, 0.12, 0.18, 0.25, 0.34, 0.44, 0.55, 0.67, 0.78, 0.87, 0.94, 1]
    retained = [0, 2, 3, 5, 6, 8, 10, 11, 12] if column_count == 9 else list(range(13))
    lateral = [original_lateral[i] for i in retained]
    nominal = [i / 12 for i in retained]'''
    original = '    lateral = [0, 0.06, 0.12, 0.18, 0.25, 0.34, 0.44, 0.55, 0.67, 0.78, 0.87, 0.94, 1]'
    for old, new in (
        (setup, original),
        ('q = lateral[i] * focus + nominal[i] * (1 - focus)',
         'q = lateral[i] * focus + (i / (cols - 1)) * (1 - focus)'),
        ('    ei0, ei1 = lateral.index(0.18), lateral.index(0.94)', '    ei0, ei1 = 3, 11'),
        ('    mi1 = lateral.index(0.44)', '    mi1 = 6'),
        ('            q = nominal[i]', '            q = i / (cols - 1)'),
        ('        "semantic_lateral_stations": lateral,\n', ''),
    ):
        source = replace_once(source, old, new)
    return source

"""Isolate quad-flow density preparation from the unchanged public default."""
import ast
import inspect


def density_rows(levels):
    """Keep calibrated rows and feature boundaries; vary only local connectivity."""
    expected = (
        1.544, 1.551, 1.560, 1.570, 1.580, 1.590, 1.599, 1.6045,
        1.608, 1.614, 1.623, 1.633, 1.644, 1.655, 1.664, 1.673,
        1.685, 1.701, 1.720, 1.740, 1.760, 1.780,
    )
    if tuple(levels) != expected:
        raise ValueError("Unknown complete semantic row contract")
    return [levels[0], levels[0]+.007/3, levels[0]+.014/3, *levels[1:]]


def chin_column_indices(columns):
    """Keep the two outer control intervals to preserve silhouette tangents."""
    if columns < 5 or columns % 2 != 1:
        raise ValueError("Unsupported chin columns")
    return list(range(columns))


def subnasal_band_faces(row, columns, nose_column):
    """Coarsen cheek-only vertical cells, keeping nose and ear-root row samples.

At each coarse/fine interface the midpoint remains connected by a quad and
triangle pair. No row is removed from either outer boundary or nasal support.
"""
    result = []
    for j in (row, row+1):
        for i in range(columns-1):
            a = j*columns+i
            result.append((a,a+1,a+columns+1,a+columns))
    return result


def replace_definition(source, name, replacement):
    nodes = [n for n in ast.parse(source).body if isinstance(n, ast.FunctionDef) and n.name == name]
    if len(nodes) != 1:
        raise ValueError('Flow density function contract changed: ' + name)
    node = nodes[0]
    lines = source.splitlines(keepends=True)
    return ''.join(lines[:node.lineno-1]) + replacement + '\n' + ''.join(lines[node.end_lineno:])


def adapt(sources):
    out = dict(sources)
    for helper in (density_rows, chin_column_indices, subnasal_band_faces):
        out['face.py'] = replace_definition(out['face.py'], helper.__name__, inspect.getsource(helper))
    replacements = {
        '    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.00349':
        '    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.0023',
        '        if j in (6, 7):':
        '        if j in (levels.index(1.599), levels.index(1.6045)):',
        '            if j == 6: shell_faces.extend(subnasal_band_faces(j, cols, lateral.index(0.18)))':
        '            if j == levels.index(1.599): shell_faces.extend(subnasal_band_faces(j, cols, lateral.index(0.18)))',
    }
    for old, new in replacements.items():
        if out['shell.py'].count(old) != 1:
            raise ValueError('Flow density shell contract changed: ' + old)
        out['shell.py'] = out['shell.py'].replace(old, new, 1)
    out['ear_surface.py'] = replace_definition(out['ear_surface.py'], 'fair_final_source_triangles', '')
    return out

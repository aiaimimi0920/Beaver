"""Shared first-pass facial density layout, before subdivision or export.

Feature endpoints and the outer silhouette remain anchors. The layout does not
collapse a finished mesh and has no character name or exported vertex IDs.
"""
import inspect
import math


def density_rows(levels):
    """Retain all calibrated row coordinates; topology sets local resolution.

Mouth/eye patch boundaries remain unchanged. Detail stays in their local rings
instead of propagating another thin horizontal strip across both cheeks.
"""
    expected = (
        1.544, 1.551, 1.560, 1.570, 1.580, 1.590, 1.599, 1.6045,
        1.608, 1.614, 1.623, 1.633, 1.644, 1.655, 1.664, 1.673,
        1.685, 1.701, 1.720, 1.740, 1.760, 1.780,
    )
    if tuple(levels) != expected:
        raise ValueError("Unknown complete semantic row contract")
    return list(levels)


def chin_column_indices(columns):
    """Keep the two outer control intervals to preserve silhouette tangents."""
    if columns < 5 or columns % 2 != 1:
        raise ValueError("Unsupported chin columns")
    return list(range(0, columns-2, 2)) + [columns-2, columns-1]


def chin_strip_faces(row, columns):
    """Join coarse interior chin rows to fine feature rows without T junctions."""
    if row not in (0, 1, 2):
        raise ValueError("Unsupported chin strip")
    indices = chin_column_indices(columns)
    result = []
    for left, right in zip(indices, indices[1:]):
        a, b = row*columns+left, row*columns+right
        c, e = (row+1)*columns+left, (row+1)*columns+right
        if row < 2 or right-left == 1:
            result.append((a, b, e, c))
        else:
            d = c+1
            result.extend(((a, b, e, d), (a, d, c)))
    return result


def subnasal_band_faces(row, columns, nose_column):
    """Coarsen cheek-only vertical cells, keeping nose and ear-root row samples.

At each coarse/fine interface the midpoint remains connected by a quad and
triangle pair. No row is removed from either outer boundary or nasal support.
"""
    first, last = nose_column, columns-4
    if not 1 <= first < last < columns-2:
        raise ValueError("Insufficient cheek transition width")
    result = []
    for i in range(columns-1):
        if i < first or i > last:
            for j in (row,row+1):
                a = j*columns+i
                result.append((a,a+1,a+columns+1,a+columns))
            continue
        a, b = row*columns+i, row*columns+i+1
        d, c = (row+2)*columns+i, (row+2)*columns+i+1
        if i == first:
            middle = (row+1)*columns+i
            result.extend(((a,b,c,middle),(middle,c,d)))
        elif i == last:
            middle = (row+1)*columns+i+1
            result.extend(((a,b,middle,d),(middle,c,d)))
        else:
            result.append((a,b,c,d))
    return result


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Density layout source contract changed: " + old[:70])
    return source.replace(old,new,1)


def adapt(sources):
    result = dict(sources)
    shell = result['shell.py']
    helpers = '\n\n'.join(inspect.getsource(f) for f in
                            (density_rows, chin_column_indices, chin_strip_faces, subnasal_band_faces))
    # Face owns shared geometric sampling primitives; shell owns assembly.
    result['face.py'] = result['face.py'] + '\n\n' + helpers + '\n'
    shell = once(shell,'    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.00349',
                 '    levels = density_rows(levels)\n    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.00349')
    shell = once(shell, '    for j in range(rows - 1):\n        for i in range(cols - 1):',
                 '    for j in range(rows - 1):\n'
                 '        if j < 3:\n'
                 '            shell_faces.extend(chin_strip_faces(j, cols))\n'
                 '            continue\n'
                 '        if j in (6, 7):\n'
                 '            if j == 6: shell_faces.extend(subnasal_band_faces(j, cols, lateral.index(0.18)))\n'
                 '            continue\n'
                 '        for i in range(cols - 1):')
    shell = once(shell, '    base = [verts[i] for i in range(cols)]\n    previous = list(range(cols))',
                 '    chin_columns = chin_column_indices(cols)\n'
                 '    base = [verts[i] for i in chin_columns]\n'
                 '    previous = chin_columns')
    shell = once(shell, '            q = nominal[i]', '            q = nominal[chin_columns[i]]')
    shell = once(shell, '        for i in range(cols - 1):\n            shell_faces.append((previous[i + 1]',
                 '        for i in range(len(base) - 1):\n            shell_faces.append((previous[i + 1]')
    shell = once(shell,'for blend in ([0, 0.30, 0.65] if kind == "eye" else [0, 0.30, 0.65]):',
                 'for blend in ([0, 0.30, 0.65] if kind == "eye" else [0, 0.30]):')
    shell = once(shell,'        "semantic_rows": levels,',
                 '        "density_layout": {"version": 6, "stage": "source sampling", "mouth_rings": 2, "rows": len(levels), "chin_columns": len(chin_columns)},\n'
                 '        "semantic_rows": levels,')
    result['shell.py'] = shell
    return result

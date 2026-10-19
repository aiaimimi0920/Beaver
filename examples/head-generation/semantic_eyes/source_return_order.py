"""Make fresh posterior-return creation independent of BMVert object hashes."""


def adapt_return_order(source):
    replacements = [
        ('    original = [(v, v.co.copy()) for v in bm.verts]',
         '    bm.verts.index_update()\n'
         '    original_index = {v: v.index for v in bm.verts}\n'
         '    original = [(v, v.co.copy()) for v in bm.verts]'),
        ('    ids = {v for edge in selected for v in edge.verts}',
         '    selected.sort(key=lambda e: tuple(sorted(original_index[v] for v in e.verts)))\n'
         '    ids = sorted({v for edge in selected for v in edge.verts}, key=original_index.__getitem__)'),
    ]
    for old, new in replacements:
        if source.count(old) != 1:
            raise ValueError('Posterior return source contract changed')
        source = source.replace(old, new, 1)
    return source

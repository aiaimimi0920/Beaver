"""Overflow-only exact UTF-8 source literal transport, no native execution."""
import ast, base64, copy, hashlib, zlib, unicodedata
MAX_LITERAL = 2 * 1024 * 1024
MAX_TOTAL = 8 * 1024 * 1024

def decoder_source(prefix):
    return f'''import base64 as {prefix}base64
import hashlib as {prefix}hashlib
import zlib as {prefix}zlib
{prefix}total = 0
def {prefix}inflate(encoded, expected, digest):
    global {prefix}total
    if type(expected) is not int or not 0 <= expected <= {MAX_LITERAL}:
        raise ValueError('Invalid source literal byte count')
    if not isinstance(encoded, str) or len(encoded) > {(MAX_LITERAL+2)//3*4}:
        raise ValueError('Invalid encoded source size')
    if {prefix}total + expected > {MAX_TOTAL}:
        raise ValueError('Source literal total exceeds bound')
    packed = {prefix}base64.b64decode(encoded, validate=True)
    unpacker = {prefix}zlib.decompressobj()
    raw = unpacker.decompress(packed, expected + 1)
    if len(raw) != expected or not unpacker.eof or unpacker.unused_data or unpacker.unconsumed_tail:
        raise ValueError('Source literal size or stream mismatch')
    if not isinstance(digest, str) or {prefix}hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError('Source literal hash mismatch')
    text = raw.decode('utf-8', errors='strict')
    {prefix}total += expected
    return text
'''


def encode_sources(request, validator, minimum=4096):
    try:
        validator(request)
        return (request, {'changed': False})
    except ValueError:
        pass
    code = request['code']
    tree = ast.parse(code)
    raw = code.encode('utf-8')
    lines = code.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line.encode('utf-8')))

    def span(n):
        return (offsets[n.lineno - 1] + n.col_offset, offsets[n.end_lineno - 1] + n.end_col_offset)
    docs = {id(n.body[0].value) for n in ast.walk(tree) if isinstance(n, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)) and n.body and isinstance(n.body[0], ast.Expr) and isinstance(n.body[0].value, ast.Constant) and isinstance(n.body[0].value.value, str)}
    annotations = set()
    for node in ast.walk(tree):
        for field in ('annotation', 'returns', 'bound', 'default_value'):
            value = getattr(node, field, None)
            if isinstance(value, ast.AST):
                annotations.update((id(x) for x in ast.walk(value)))
    values = {}
    parents = {id(c): n for n in ast.walk(tree) for c in ast.iter_child_nodes(n)}
    for n in ast.walk(tree):
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and (id(n) not in docs) and (id(n) not in annotations) and (not isinstance(parents.get(id(n)), ast.JoinedStr)) and (len(n.value.encode('utf-8')) >= minimum):
            values.setdefault(n.value, []).append(n)
    prefix = '_beaver_lossless_'
    while prefix in unicodedata.normalize('NFKC', code):
        prefix += 'x_'
    assignments = []
    edits = []
    records = []
    total = 0
    for value, nodes in values.items():
        data = value.encode('utf-8')
        if len(data) > MAX_LITERAL:
            raise ValueError('Original source literal exceeds bound')
        encoded = base64.b64encode(zlib.compress(data, 9)).decode('ascii')
        digest = hashlib.sha256(data).hexdigest()
        name = prefix + 'source_' + str(len(records))
        assignment = f'{name} = {prefix}inflate({encoded!r}, {len(data)}, {digest!r})\n'
        saving = sum((b - a for a, b in map(span, nodes))) - len(assignment) - sum((len(name) for _ in nodes))
        if saving <= 128:
            continue
        total += len(data)
        if total > MAX_TOTAL:
            raise ValueError('Original source total exceeds bound')
        assignments.append(assignment)
        edits.extend(((a, b, name.encode()) for a, b in map(span, nodes)))
        records.append({'name': name, 'bytes': len(data), 'sha256': digest, 'encoded': encoded})
    if not records:
        raise ValueError('No effective exact source encoding available')
    insertion = 0
    for node in tree.body:
        doc = isinstance(node, ast.Expr) and id(node.value) in docs
        future = isinstance(node, ast.ImportFrom) and node.module == '__future__'
        if not (doc or future):
            insertion = span(node)[0]
            if getattr(node, 'decorator_list', None):
                insertion = offsets[node.decorator_list[0].lineno - 1]
            break
        insertion = span(node)[1]
    for a, b, replacement in sorted(edits, reverse=True):
        raw = raw[:a] + replacement + raw[b:]
    header = (decoder_source(prefix) + ''.join(assignments)).encode()
    if insertion and (not raw[:insertion].endswith(b'\n')):
        header = b'\n' + header
    candidate = dict(request, code=(raw[:insertion] + header + raw[insertion:]).decode())
    compile(candidate['code'], '<lossless source transport>', 'exec')
    size = validator(candidate)
    receipt = {'changed': True, 'prefix': prefix, 'records': records, 'serialized_bytes': size}
    if expanded_ast(candidate['code'], receipt) != ast.dump(tree, include_attributes=False):
        raise ValueError('Lossless source transport AST mismatch')
    return (candidate, receipt)

def expanded_ast(encoded, receipt):
    tree = ast.parse(encoded)
    prefix = receipt['prefix']
    values = {}

    def owned(n):
        return isinstance(n, (ast.Import, ast.ImportFrom)) and all(((a.asname or '').startswith(prefix) for a in n.names)) or (isinstance(n, ast.FunctionDef) and n.name == prefix + 'inflate') or (isinstance(n, ast.Assign) and all((isinstance(x, ast.Name) and x.id.startswith(prefix) for x in n.targets)))
    names = {r['name'] for r in receipt['records']}
    selected = [n for n in tree.body if isinstance(n, ast.Assign) and len(n.targets) == 1 and isinstance(n.targets[0], ast.Name) and (n.targets[0].id in names)]
    assignments = {n.targets[0].id: n for n in selected}
    if len(selected) != len(names) or set(assignments) != names:
        raise ValueError('Transport source assignment missing')
    header = [n for n in tree.body if owned(n) and (not (isinstance(n, ast.Assign) and n.targets[0].id in names))]
    if ast.dump(ast.Module(body=header, type_ignores=[]), include_attributes=False) != ast.dump(ast.parse(decoder_source(prefix)), include_attributes=False):
        raise ValueError('Transport decoder AST differs')
    indices = [i for i, n in enumerate(tree.body) if owned(n)]
    first = 0
    for n in tree.body:
        if first == 0 and isinstance(n, ast.Expr) and isinstance(n.value, ast.Constant) and isinstance(n.value.value, str) or (isinstance(n, ast.ImportFrom) and n.module == '__future__'):
            first += 1
        else:
            break
    if indices != list(range(first, first + len(indices))):
        raise ValueError('Transport header is not contiguous or correctly placed')
    owned_nodes = [tree.body[i] for i in indices]
    expected = ast.parse(decoder_source(prefix)).body + [assignments[r['name']] for r in receipt['records']]
    if ast.dump(ast.Module(body=owned_nodes, type_ignores=[]), include_attributes=False) != ast.dump(ast.Module(body=expected, type_ignores=[]), include_attributes=False):
        raise ValueError('Transport header order differs')
    namespace = {}
    exec(decoder_source(prefix), namespace)
    for r in receipt['records']:
        call = assignments[r['name']].value
        if not isinstance(call, ast.Call) or not isinstance(call.func, ast.Name) or call.func.id != prefix + 'inflate' or (len(call.args) != 3) or call.keywords or (not all((isinstance(v, ast.Constant) for v in call.args))):
            raise ValueError('Unexpected transport literal call')
        arguments = [v.value for v in call.args]
        if arguments != [r['encoded'], r['bytes'], r['sha256']]:
            raise ValueError('Transport arguments differ from receipt')
        values[r['name']] = namespace[prefix + 'inflate'](*arguments)
    tree.body = [n for n in tree.body if not owned(n)]

    class Inline(ast.NodeTransformer):

        def visit_Name(self, n):
            return ast.copy_location(ast.Constant(values[n.id]), n) if isinstance(n.ctx, ast.Load) and n.id in values else n
    return ast.dump(Inline().visit(tree), include_attributes=False)

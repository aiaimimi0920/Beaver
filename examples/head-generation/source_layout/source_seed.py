"""Separate a readable retained source module into a hash-bound native seed job."""
import ast
import hashlib
import json
import unicodedata


def seed_retained_bundle(code, prefix):
    tree = ast.parse(code)
    candidates = [node for node in ast.walk(tree)
                  if isinstance(node, ast.Constant) and isinstance(node.value, str)
                  and 'RETAINED_UPPER_SHAPE_SOURCES =' in node.value
                  and 'RETAINED_BROW' in node.value]
    if len(candidates) != 1:
        raise ValueError('Expected one retained semantic bundle literal')
    node = candidates[0]
    source = node.value
    compile(source, '<retained bundle syntax>', 'exec')
    # A large unshared runtime module may also be carried as ordinary readable
    # source in this same seed. Do not compress code or relax native limits.
    runtime_nodes = [call.args[0] for call in ast.walk(tree)
                     if isinstance(call,ast.Call) and isinstance(call.func,ast.Name)
                     and call.func.id == 'compile' and call.args
                     and isinstance(call.args[0],ast.Constant)
                     and isinstance(call.args[0].value,str)
                     and len(call.args[0].value.encode('utf-8')) > 32768]
    runtime_node = max(runtime_nodes,key=lambda n:len(n.value)) if runtime_nodes else None
    runtime_source = runtime_node.value if runtime_node is not None else None
    seed_source = source
    if runtime_source is not None:
        compile(runtime_source,'<runtime seed syntax>','exec')
        seed_source += '\n\nNATIVE_RUNTIME_SOURCE = '+repr(runtime_source)+'\n'
    encoded = seed_source.encode('utf-8')
    digest = hashlib.sha256(encoded).hexdigest()
    path = prefix + 'source_seed_semantic_bundle.py'
    output = {'path': path, 'expectedSha256': None}
    seed_code = ('from pathlib import Path\n'
                 f'Path(beaver_output({path!r})).write_text({seed_source!r},encoding="utf-8",newline="\\n")\n')
    seed = {'code':seed_code,'outputs':[output],'timeoutSeconds':120}
    name = '_beaver_verified_source_seed_'
    while name in unicodedata.normalize('NFKC', code):
        name += '_'
    offsets = [0]
    for line in code.encode('utf-8').splitlines(keepends=True):
        offsets.append(offsets[-1] + len(line))
    raw = code.encode('utf-8')
    substitutions = [(node,'text',source)]
    if runtime_node is not None:
        substitutions.append((runtime_node,'runtime',runtime_source))
    edits = [(offsets[n.lineno-1]+n.col_offset,offsets[n.end_lineno-1]+n.end_col_offset,
              (name+suffix).encode()) for n,suffix,_ in substitutions]
    for start,stop,replacement in sorted(edits,reverse=True):
        raw = raw[:start]+replacement+raw[stop:]
    preserved = 0
    insertion = 0
    for index, statement in enumerate(tree.body):
        doc = (index == 0 and isinstance(statement, ast.Expr)
               and isinstance(statement.value, ast.Constant)
               and isinstance(statement.value.value, str))
        future = isinstance(statement, ast.ImportFrom) and statement.module == '__future__'
        if not (doc or future):
            insertion = offsets[statement.lineno-1] + statement.col_offset
            break
        preserved += 1
    header = (f'from pathlib import Path as {name}Path\n'
              f'import hashlib as {name}hashlib\n'
              f'{name}bytes = {name}Path(beaver_input({path!r})).read_bytes()\n'
              f'if {name}hashlib.sha256({name}bytes).hexdigest() != {digest!r}:\n    raise ValueError("Authored source seed changed")\n'
              f'{name}text = {name}bytes[:{len(source.encode("utf-8"))}].decode("utf-8")\n')
    if runtime_source is not None:
        header += (f'import ast as {name}ast\n'
                   f'{name}pool = {name}ast.parse({name}bytes[{len(source.encode("utf-8"))}:].decode("utf-8")).body\n'
                   f'if len({name}pool)!=1 or not isinstance({name}pool[0],{name}ast.Assign) '
                   f'or len({name}pool[0].targets)!=1 or not isinstance({name}pool[0].targets[0],{name}ast.Name) '
                   f'or {name}pool[0].targets[0].id!="NATIVE_RUNTIME_SOURCE" '
                   f'or not isinstance({name}pool[0].value,{name}ast.Constant) '
                   f'or not isinstance({name}pool[0].value.value,str):\n'
                   '    raise ValueError("Invalid runtime source seed schema")\n'
                   f'{name}runtime = {name}ast.literal_eval({name}pool[0].value)\n')
    if insertion and not raw[:insertion].endswith(b'\n'):
        header = '\n' + header
    result = (raw[:insertion] + header.encode() + raw[insertion:]).decode('utf-8')
    restored = ast.parse(result)
    del restored.body[preserved:preserved+len(ast.parse(header).body)]
    originals = {name+suffix:value for _,suffix,value in substitutions}
    class Restore(ast.NodeTransformer):
        def visit_Name(self, current):
            return ast.Constant(value=originals[current.id]) if current.id in originals else current
    restored = Restore().visit(restored)
    if ast.dump(restored,include_attributes=False) != ast.dump(tree,include_attributes=False):
        raise AssertionError('Seed binding changed original program AST')
    binding = {'path':path,'sha256':digest,
               'native_seed_script_sha256':hashlib.sha256(seed_code.encode()).hexdigest()}
    result += f'\nreport["authored_source_seed"] = {binding!r}\n'
    result += f'Path(beaver_output({prefix+"generation_report.json"!r})).write_text(json.dumps(report,indent=2),encoding="utf-8",newline="\\n")\n'
    compile(result, '<hash-bound source seed>', 'exec')
    return result, seed, binding

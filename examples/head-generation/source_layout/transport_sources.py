"""Share repeated readable source literals without changing emitted module bytes."""

import ast
import json
import unicodedata
from collections import defaultdict


def share_source_literals(code, minimum=4096):
    """Run after source extraction, before freezing the native script identity.

    This is source deduplication, not compression or a change to native limits.
    Every shared value remains an ordinary uncompressed Python string literal.
    """
    tree = ast.parse(code)
    lines = code.encode('utf-8').splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))

    def span(node):
        return (offsets[node.lineno - 1] + node.col_offset,
                offsets[node.end_lineno - 1] + node.end_col_offset)

    docstrings = set()
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.body and isinstance(node.body[0], ast.Expr):
                value = node.body[0].value
                if isinstance(value, ast.Constant) and isinstance(value.value, str):
                    docstrings.add(id(value))
    values = defaultdict(list)
    roles = defaultdict(set)
    for call in ast.walk(tree):
        if not isinstance(call, ast.Call) or not call.args:
            continue
        role = None
        if isinstance(call.func, ast.Name) and call.func.id == 'compile':
            role = 'compile'
        elif isinstance(call.func, ast.Attribute) and call.func.attr == 'write_text':
            role = 'write_text'
        node = call.args[0]
        if (role and isinstance(node, ast.Constant) and isinstance(node.value, str)
                and len(node.value.encode('utf-8')) >= minimum):
            values[node.value].append(node)
            roles[node.value].add(role)
    repeated = sorted(((v, ns) for v, ns in values.items()
                       if len(ns) > 1 and roles[v] == {'compile', 'write_text'}),
                      key=lambda pair: min(span(n)[0] for n in pair[1]))
    if not repeated:
        return code
    prefix = '_beaver_shared_source_'
    normalized = unicodedata.normalize('NFKC', code)
    while prefix in normalized:
        prefix = '_' + prefix
    edits, assignments = [], []
    for index, (value, nodes) in enumerate(repeated):
        name = prefix + str(index)
        assignments.append(name + ' = ' + repr(value) + '\n')
        edits.extend((span(n)[0], span(n)[1], name.encode('ascii')) for n in nodes)
    insertion = 0
    for node in tree.body:
        is_docstring = isinstance(node, ast.Expr) and id(node.value) in docstrings
        is_future = isinstance(node, ast.ImportFrom) and node.module == '__future__'
        if not (is_docstring or is_future):
            # Insert before the first executable statement, even after a semicolon.
            insertion = span(node)[0]
            if getattr(node, 'decorator_list', None):
                insertion = offsets[node.decorator_list[0].lineno - 1]
            break
        insertion = span(node)[1]
    raw = code.encode('utf-8')
    for start, end, replacement in sorted(edits, reverse=True):
        raw = raw[:start] + replacement + raw[end:]
    shared = ''.join(assignments).encode('utf-8')
    if insertion and not raw[:insertion].endswith(b'\n'):
        shared = b'\n' + shared
    result = (raw[:insertion] + shared + raw[insertion:]).decode('utf-8')
    compile(result, '<shared native source literals>', 'exec')
    return result


def validate_native_frame(request):
    """Bound the full direct-host JSON frame, not only its decoded Python code.

    External identities allow 160 UTF-8 bytes each. NUL is their worst JSON
    escaping case (six bytes per one input byte). The top-level protocol ID
    allows 200 bytes; revisions use the Core's maximum u64.
    """
    identity = '\0' * 160
    frame = {"id": "\0" * 200, "method": "external.tool", "input": {
        "taskId": identity, "runId": identity, "revision": 2**64 - 1,
        "requestId": identity,
        "arguments": {"tool": "blender.start", "arguments": request},
    }}
    size = len((json.dumps(frame, ensure_ascii=False, separators=(',', ':')) + '\n').encode('utf-8'))
    if size > 1024 * 1024:
        raise ValueError(f"Native serialized request frame limit exceeded: {size} bytes")
    return size

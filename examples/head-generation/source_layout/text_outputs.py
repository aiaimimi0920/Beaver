"""Make owned native text outputs byte-stable across operating systems."""

import ast


def canonical_text_outputs(code, prefix):
    tree = ast.parse(code)
    offsets = [0]
    for line in code.encode('utf-8').splitlines(keepends=True):
        offsets.append(offsets[-1] + len(line))
    edits = []
    for call in ast.walk(tree):
        if not (isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute)
                and call.func.attr == 'write_text'):
            continue
        receiver = call.func.value
        if not (isinstance(receiver, ast.Call) and isinstance(receiver.func, ast.Name)
                and receiver.func.id == 'Path' and len(receiver.args) == 1):
            continue
        output = receiver.args[0]
        if not (isinstance(output, ast.Call) and isinstance(output.func, ast.Name)
                and output.func.id == 'beaver_output' and len(output.args) == 1
                and isinstance(output.args[0], ast.Constant)
                and isinstance(output.args[0].value, str)
                and output.args[0].value.startswith(prefix)):
            continue
        if len(call.args) != 1:
            raise ValueError('Owned text output must have one data argument')
        keywords = {keyword.arg: keyword.value for keyword in call.keywords}
        if None in keywords or set(keywords) - {'encoding', 'errors', 'newline'}:
            raise ValueError('Unsupported owned text-output options')
        additions = []
        for name, expected in (('encoding', 'utf-8'), ('newline', '\n')):
            if name in keywords:
                if not (isinstance(keywords[name], ast.Constant)
                        and keywords[name].value == expected):
                    raise ValueError('Noncanonical owned text-output ' + name)
            else:
                additions.append(f'{name}={expected!r}')
        if additions:
            last = call.keywords[-1].value if call.keywords else call.args[-1]
            end = offsets[last.end_lineno - 1] + last.end_col_offset
            edits.append((end, (', ' + ', '.join(additions)).encode('ascii')))
    raw = code.encode('utf-8')
    for position, addition in sorted(edits, reverse=True):
        raw = raw[:position] + addition + raw[position:]
    result = raw.decode('utf-8')
    compile(result, '<canonical native text outputs>', 'exec')
    return result

"""Conservative Python source counts, excluding only comments and real docstrings."""
import ast
import io
import tokenize


def conservative_effective_lines(source):
    tree = ast.parse(source)
    docs = {}
    for node in ast.walk(tree):
        if not isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)) or not node.body:
            continue
        first = node.body[0]
        if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) and isinstance(first.value.value, str):
            value = first.value
            docs[(value.lineno, value.col_offset)] = value.value
    ignored = {tokenize.ENCODING, tokenize.ENDMARKER, tokenize.NEWLINE, tokenize.NL,
               tokenize.INDENT, tokenize.DEDENT, tokenize.COMMENT}
    lines = set()
    for token in tokenize.generate_tokens(io.StringIO(source).readline):
        if token.type in ignored:
            continue
        if token.type == tokenize.STRING and token.start in docs:
            try:
                if ast.literal_eval(token.string) == docs[token.start]:
                    continue
            except (ValueError, SyntaxError):
                pass
        lines.update(range(token.start[0], token.end[0] + 1))
    return len(lines)

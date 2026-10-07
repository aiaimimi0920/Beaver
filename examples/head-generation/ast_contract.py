"""Version-portable comparison of serialized Python function AST contracts."""
import ast


def canonical_function_dump(serialized):
    """Ignore only empty generic-parameter metadata, never real type parameters.

    Python3.12 adds FunctionDef/AsyncFunctionDef/ClassDef.type_params. The frozen
    non-generic functions contain [], which Python3.11 does not emit. Parse the
    dump as data and remove that one empty field structurally. Do not replace
    text inside string literals or ignore any executable node or nonempty field.
    """
    expression = ast.parse(serialized, mode="eval")
    for node in ast.walk(expression):
        if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Name):
            continue
        if node.func.id not in ("FunctionDef", "AsyncFunctionDef", "ClassDef"):
            continue
        node.keywords = [
            keyword for keyword in node.keywords
            if not (keyword.arg == "type_params"
                    and isinstance(keyword.value, ast.List)
                    and not keyword.value.elts)
        ]
    return ast.dump(expression, include_attributes=False)

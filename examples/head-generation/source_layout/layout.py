"""Preserve generated source while separating explicit authoring responsibilities."""

import ast
import hashlib
import re


def digest(source):
    return hashlib.sha256(source.encode("utf-8")).hexdigest()


def face_loop(node):
    return (
        isinstance(node, ast.For)
        and isinstance(node.target, ast.Name)
        and node.target.id == "role"
        and ast.literal_eval(node.iter) == ["Face"]
    )


def pipeline_layout(source):
    tree = ast.parse(source)
    if any(
        isinstance(n, ast.ImportFrom) and n.module == "__future__" for n in tree.body
    ):
        raise ValueError("Future-import semantics require explicit migration")
    if any(isinstance(n, ast.AnnAssign) for n in ast.walk(tree)):
        raise ValueError("Annotation state requires explicit migration")
    loops = [n for n in tree.body if face_loop(n)]
    if len(loops) != 2:
        raise ValueError("Expected separate face assembly and morph stages")
    first, second = loops
    exports = [n for n in tree.body if n.lineno > second.end_lineno]
    if not exports:
        raise ValueError("Missing export stage")
    out = [
        n.value
        for n in tree.body
        if isinstance(n, ast.Assign)
        and any(isinstance(t, ast.Name) and t.id == "OUT" for t in n.targets)
    ]
    if (
        len(out) != 1
        or not isinstance(out[0], ast.Constant)
        or not isinstance(out[0].value, str)
    ):
        raise ValueError("Expected one fixed output prefix")
    if not re.fullmatch(r"(?:[a-zA-Z0-9_-]+/)+", out[0].value):
        raise ValueError("Invalid output prefix")
    if any(
        isinstance(n, ast.Name) and n.id.startswith("_pipeline_")
        for n in ast.walk(tree)
    ):
        raise ValueError("Reserved pipeline runner name collision")
    lines = source.splitlines(keepends=True)
    boundaries = [
        0,
        first.lineno - 1,
        second.lineno - 1,
        exports[0].lineno - 1,
        len(lines),
    ]
    names = [
        "pipeline_geometry.py",
        "pipeline_assembly.py",
        "pipeline_runtime.py",
        "pipeline_export.py",
    ]
    chunks = {
        name: "".join(lines[a:b])
        for name, a, b in zip(names, boundaries, boundaries[1:])
    }
    if "".join(chunks.values()) != source:
        raise AssertionError("Source reconstruction changed")
    for name, part in chunks.items():
        compile(part, name, "exec")
    entry = '"""Ordered geometry, material assembly, runtime mesh/morph and export stages share the authoring frame."""\n'
    entry += "from pathlib import Path\n\n"
    entry += f"_pipeline_source_prefix = {out[0].value!r}\n"
    entry += f"_pipeline_stage_names = {tuple(names)!r}\n"
    entry += "for _pipeline_stage_name in _pipeline_stage_names:\n"
    entry += "    _pipeline_stage_path = Path(beaver_input(_pipeline_source_prefix + _pipeline_stage_name))\n"
    entry += '    exec(compile(_pipeline_stage_path.read_text(), str(_pipeline_stage_path), "exec"), globals())\n'
    compile(entry, "pipeline.py", "exec")
    record = {
        "kind": "pipeline_stages",
        "prefix": out[0].value,
        "original_sha256": digest(source),
        "entry_sha256": digest(entry),
        "stages": [
            {"path": name, "sha256": digest(part)} for name, part in chunks.items()
        ],
    }
    return {"pipeline.py": entry, **chunks}, record


def parameter_layout(source):
    tree = ast.parse(source)
    imports = [n for n in tree.body if isinstance(n, (ast.Import, ast.ImportFrom))]
    if not imports:
        raise ValueError("Missing parameter imports")
    functions = [
        n
        for n in tree.body
        if isinstance(n, ast.FunctionDef) and n.end_lineno < imports[0].lineno
    ]
    wanted = {
        "angular_coordinate",
        "angular_derivative",
        "eye_warp",
        "eye_warp_derivative",
        "upper_shape",
        "upper_shape_derivative",
        "lower_coordinate",
        "lower_shape",
    }
    names = [n.name for n in functions]
    if (
        not {"upper_shape", "upper_shape_derivative"} <= set(names)
        or not set(names) <= wanted
    ):
        raise ValueError("Unexpected curve-function boundary")
    lines = source.splitlines(keepends=True)
    cut = max(n.end_lineno for n in functions)
    curves = "".join(lines[:cut])
    suffix = "".join(lines[cut:])
    doc = ast.get_docstring(tree, clean=False)
    prefix = repr(doc) + "\n" if doc is not None else ""
    prefix += (
        "from semantic_parameter_curves import (\n"
        + "".join("    " + name + ",\n" for name in names)
        + ")\n"
    )
    entry = prefix + suffix
    compile(curves, "semantic_parameter_curves.py", "exec")
    compile(entry, "semantic_parameters.py", "exec")
    record = {
        "kind": "parameter_curves",
        "original_sha256": digest(source),
        "entry_sha256": digest(entry),
        "helper_sha256": digest(curves),
        "entry_prefix_bytes": len(prefix.encode("utf-8")),
        "entry_prefix_sha256": digest(prefix),
        "exports": names,
    }
    return {
        "semantic_parameters.py": entry,
        "semantic_parameter_curves.py": curves,
    }, record


def reconstructed(files, record):
    if record["kind"] == "pipeline_stages":
        if digest(files["pipeline.py"]) != record["entry_sha256"]:
            raise ValueError("Changed pipeline entry")
        parts = []
        for item in record["stages"]:
            part = files[item["path"]]
            if digest(part) != item["sha256"]:
                raise ValueError("Changed pipeline stage")
            parts.append(part)
        original = "".join(parts)
    elif record["kind"] == "parameter_curves":
        entry = files["semantic_parameters.py"]
        helper = files["semantic_parameter_curves.py"]
        if (
            digest(entry) != record["entry_sha256"]
            or digest(helper) != record["helper_sha256"]
        ):
            raise ValueError("Changed parameter source")
        raw = entry.encode("utf-8")
        cut = record["entry_prefix_bytes"]
        if hashlib.sha256(raw[:cut]).hexdigest() != record["entry_prefix_sha256"]:
            raise ValueError("Changed parameter import boundary")
        original = helper + raw[cut:].decode("utf-8")
    else:
        raise ValueError("Unknown source layout")
    if digest(original) != record["original_sha256"]:
        raise ValueError("Original source identity changed")
    return original

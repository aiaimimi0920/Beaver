"""Explicit raw13 capture hook at real source-to-six-saved-key boundaries."""

import ast
import hashlib
import json
from pathlib import Path

POLICY = "POST_MORPH_SOURCE_CAPTURE_V1"
RUNTIME_MODULES = ("shared_capture_contract", "shared_capture_native", "shared_capture_normals")
SOURCE_FILES = (*RUNTIME_MODULES, "shared_capture_adapter")


def retained_sources():
    root = Path(__file__).resolve().parent
    return {name + ".py": (root / (name + ".py")).read_text(encoding="utf-8")
            for name in SOURCE_FILES}


def validate_policy(policy, parameters, blink, mouth_design):
    if policy is None:
        return
    if policy != POLICY:
        raise ValueError("Unsupported explicit shared source capture policy")
    if not blink or mouth_design is None:
        raise ValueError("Source capture requires actual blink and researched mouth six-key builder")
    if parameters["sampling_feature_flow"] != 1:
        raise ValueError("Source capture currently supports explicit raw13 feature flow only")
    from sampling_contract import validate_feature_flow_contract
    validate_feature_flow_contract(parameters)


def adapt(sources):
    """Return retained readable modules and hook code; default dispatch never calls it."""
    out = dict(sources)
    if any(name + ".py" in out for name in SOURCE_FILES):
        raise ValueError("Source capture module collision")
    inventory = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in sorted(out.items())}
    encoded = json.dumps(inventory, sort_keys=True, separators=(",", ":"))
    recipe = {"policy": POLICY, "source_sha256": hashlib.sha256(encoded.encode()).hexdigest(),
              "source_file_count": len(inventory),
              "source_binding": "SHA256 compact sorted UTF8 JSON filename-to-source-SHA256 inventory"}
    out["shared_capture_binding.json"] = json.dumps({"recipe": recipe, "source_inventory": inventory})
    pipeline = out["pipeline.py"]
    begin = "        _source_skin_quads=[(p.index,tuple(p.vertices)) for p in data.polygons if len(p.vertices)==4 and data.attributes['NormalRegion'].data[p.index].value in (1,2)]"
    finish = '        report["mouth_motion"] = build_mouth_shapes(members[0], semantic_part_ordinals, C)'
    if pipeline.count(begin) != 1 or pipeline.count(finish) != 1:
        raise ValueError("Source capture requires unique pre-triangle and post-six-key boundaries")
    if pipeline.index(begin) >= pipeline.index(finish):
        raise ValueError("Source capture lifecycle reversed")
    for name in ("begin_source_capture", "finish_source_capture", "_shared_source_capture"):
        if name in pipeline:
            raise ValueError("Source capture hook collision")
    begin_code = (
        "        from shared_capture_native import begin_source_capture, finish_source_capture, clear_markers\n"
        "        _shared_source_capture = begin_source_capture(members[0],\n"
        "            report['ear_shell_membership']['authored_skin_ownership'],\n"
        f"            {recipe!r})\n"
    )
    finish_code = (
        "\n        report['shared_source_capture'] = finish_source_capture(members[0], _shared_source_capture)"
    )
    pipeline = pipeline.replace(begin, begin + "\n" + begin_code.rstrip(), 1)
    pipeline = pipeline.replace(finish, finish + finish_code, 1)
    normal = "        data.normals_split_custom_set(corner_normals)"
    if pipeline.count(normal) != 1 or not pipeline.index(begin) < pipeline.index(normal) < pipeline.index(finish):
        raise ValueError("Unique native normal setter lifecycle required")
    pipeline = pipeline.replace(normal,
        "        from shared_capture_normals import set_captured_normals\n"
        "        set_captured_normals(members[0], _shared_source_capture, corner_normals)", 1)
    out["pipeline.py"] = guard_capture_interval(pipeline)
    out.update(retained_sources())
    for name, text in out.items():
        if name.endswith(".py"):
            ast.parse(text, filename=name)
    return out


def bootstrap(retained_bundle):
    """Reuse the existing single retained source/seed, without extra output slots.

    Parse the literal source dictionary; never execute the retained packager.
    The ordinary source_seed packager binds this one literal to its existing
    verified seed receipt, so runtime sources are not duplicated in the frame.
    """
    script = f"_shared_capture_retained_bundle = {retained_bundle!r}\n"
    script += "import ast as _capture_ast, types as _capture_types, sys as _capture_sys\n"
    script += "_capture_nodes = [node for node in _capture_ast.parse(_shared_capture_retained_bundle).body if isinstance(node, _capture_ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], _capture_ast.Name) and node.targets[0].id == 'RETAINED_SHARED_CAPTURE_SOURCES']\n"
    script += "if len(_capture_nodes) != 1:\n    raise ValueError('Missing retained capture source dictionary')\n"
    script += "_capture_sources = _capture_ast.literal_eval(_capture_nodes[0].value)\n"
    script += f"for _capture_name in {RUNTIME_MODULES!r}:\n"
    script += "    _capture_module = _capture_types.ModuleType(_capture_name)\n"
    script += "    _capture_sys.modules[_capture_name] = _capture_module\n"
    script += "    exec(compile(_capture_sources[_capture_name + '.py'], _capture_name, 'exec'), _capture_module.__dict__)\n"
    return script


def adapt_optional(sources, policy):
    return sources if policy is None else adapt(sources)


def retain_capture(request, retained, policy, resolved_sources):
    sources = retained_sources()
    if policy is None:
        sources = {"shared_capture_adapter.py": sources["shared_capture_adapter.py"]}
    retained["semantic_bundle.py"] += "\nRETAINED_SHARED_CAPTURE_SOURCES = " + repr(sources) + "\n"
    if policy is not None:
        binding = json.loads(resolved_sources["shared_capture_binding.json"])
        retained["semantic_bundle.py"] += "\nRETAINED_SHARED_CAPTURE_SOURCE_BINDING = " + repr(binding) + "\n"
        request["code"] = bootstrap(retained["semantic_bundle.py"]) + request["code"]


def source_write(prefix, name, source, policy):
    value = "_shared_capture_retained_bundle" if name == "semantic_bundle.py" and policy is not None else repr(source)
    return f"Path(beaver_output({(prefix + name)!r})).write_text({value})\n"


def replace_once(source, old, new, *, preview=70):
    """Shared packager marker guard, preserving each caller's diagnostic preview."""
    if source.count(old) != 1:
        raise ValueError("Inherited source contract changed: " + old[:preview])
    return source.replace(old, new, 1)


def guard_capture_interval(pipeline):
    """Cleanup spans triangulation, normal generation and all six native builders."""
    calls = {}
    for node in ast.walk(ast.parse(pipeline)):
        if (isinstance(node, ast.Assign) and isinstance(node.value, ast.Call)
                and isinstance(node.value.func, ast.Name)
                and node.value.func.id in ("begin_source_capture", "finish_source_capture")):
            name = node.value.func.id
            if name in calls:
                raise ValueError("Duplicate capture lifecycle assignment")
            calls[name] = node
    begin, finish = calls["begin_source_capture"], calls["finish_source_capture"]
    if begin.end_lineno >= finish.lineno or begin.col_offset != finish.col_offset:
        raise ValueError("Changed source capture lifetime")
    lines = pipeline.splitlines(keepends=True)
    indent = " " * begin.col_offset
    interval = ["    " + line if line.strip() else line for line in lines[begin.end_lineno:finish.end_lineno]]
    lines[begin.end_lineno:finish.end_lineno] = [indent + "try:\n", *interval,
        indent + "finally:\n", indent + "    clear_markers(members[0].data)\n"]
    return "".join(lines)


def load_module(path, name):
    """Host-only packager loading, preserving the caller's filesystem boundary."""
    import importlib.util
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_support_bound(value):
    import math
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value <= 0:
        raise ValueError("Supply the explicitly reviewed positive support bound in meters")


def validate_output_count(request):
    paths = [output["path"] for output in request["outputs"]]
    if len(paths) > 64 or len(set(paths)) != len(paths):
        raise ValueError("Native request requires at most 64 unique retained outputs")


def bind_expected_outputs(request, root):
    for entry in request["outputs"]:
        path = root / entry["path"]
        if path.exists():
            if not path.is_file():
                raise ValueError("Non-file output: " + entry["path"])
            entry["expectedSha256"] = hashlib.sha256(path.read_bytes()).hexdigest()


def optional_constructor(policy, parameters):
    """Default requests never import or retain the opt-in constructor runtime."""
    if policy is None:
        return None
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "shared_construction_capture"))
    try:
        import construction_adapter
    finally:
        sys.path.pop(0)
    construction_adapter.validate_policy(policy, parameters)
    return construction_adapter

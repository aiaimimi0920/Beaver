"""Explicit constructor-capture policy and hash-bound existing-seed packaging."""
import ast
import hashlib
from pathlib import Path
from construction_instrument import instrument, replace
from construction_topology import digest

POLICY = "CONSTRUCTION_IDENTITY_CAPTURE_V1"
RUNTIME_MODULES = ("construction_topology", "construction_roles", "construction_boundaries", "construction_journal", "construction_mirror_contract",
                   "construction_markers", "construction_native")
SOURCE_MODULES = (*RUNTIME_MODULES, "construction_instrument", "construction_adapter")


def retained_sources():
    root = Path(__file__).resolve().parent
    return {name + ".py": (root / (name + ".py")).read_text(encoding="utf-8") for name in SOURCE_MODULES}


def validate_policy(policy, parameters):
    if policy is None:
        return
    if policy != POLICY or parameters["sampling_feature_flow"] != 1:
        raise ValueError("Constructor capture requires its explicit current feature-flow policy")
    from sampling_contract import validate_feature_flow_contract
    validate_feature_flow_contract(parameters)


def adapt_optional(sources, policy):
    if policy is None:
        return sources
    if any(name + ".py" in sources for name in SOURCE_MODULES):
        raise ValueError("Constructor module collision")
    inventory = {name: hashlib.sha256(value.encode()).hexdigest() for name, value in sorted(sources.items())}
    capture_inventory = {name: hashlib.sha256(value.encode()).hexdigest() for name, value in retained_sources().items()}
    recipe = {"policy": POLICY, "capture_inventory_sha256": digest(capture_inventory), "source_inventory_sha256": digest(inventory), "source_file_count": len(inventory),
              "source_hash_encoding": "SHA256 of UTF8 source text, then canonical JSON inventory"}
    out = instrument(sources, recipe)
    pipeline = out["pipeline.py"]
    before = ('        for m in list(o.modifiers):\n'
              '            if m.type == "SUBSURF" and m.levels == 0:\n'
              '                o.modifiers.remove(m)\n'
              '            else:\n'
              '                bpy.ops.object.modifier_apply(modifier=m.name)')
    pipeline = replace(pipeline, before, '        from construction_native import apply_modifiers\n        apply_modifiers(o)')
    pipeline = replace(pipeline, '    bpy.ops.object.join()',
                       '    from construction_native import join_objects\n    _construction_joined = join_objects(members)')
    pipeline = replace(pipeline, '        bm = bmesh.new()',
                       '        from construction_native import invalidate_before_surgery\n'
                       '        invalidate_before_surgery(_construction_joined, "weld_and_dissolve")\n        bm = bmesh.new()') if pipeline.count('        bm = bmesh.new()') == 1 else insert_before_first_bmesh(pipeline)
    out["pipeline.py"] = pipeline
    out["construction_binding.json"] = __import__("json").dumps({"recipe": recipe, "source_inventory": inventory, "capture_inventory": capture_inventory})
    for name, source in out.items():
        if name.endswith(".py"):
            ast.parse(source, filename=name)
    return out


def insert_before_first_bmesh(pipeline):
    tree = ast.parse(pipeline)
    candidates = [node for node in ast.walk(tree) if isinstance(node, ast.Assign)
                  and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Attribute)
                  and isinstance(node.value.func.value, ast.Name)
                  and (node.value.func.value.id, node.value.func.attr) == ("bmesh", "new")]
    if not candidates:
        raise ValueError("Missing first native surgery boundary")
    node = min(candidates, key=lambda item: item.lineno)
    if node.col_offset != 8:
        raise ValueError("Changed first native surgery scope")
    lines = pipeline.splitlines(keepends=True)
    lines[node.lineno - 1:node.lineno - 1] = ['        from construction_native import invalidate_before_surgery\n',
        '        invalidate_before_surgery(_construction_joined, "weld_and_dissolve")\n']
    return "".join(lines)


def retain_sources(retained, policy, resolved_sources):
    if policy is None:
        return
    import json
    sources = retained_sources()
    binding = json.loads(resolved_sources["construction_binding.json"])
    inventory = {name: hashlib.sha256(value.encode()).hexdigest() for name, value in sources.items()}
    if inventory != binding["capture_inventory"] or digest(inventory) != binding["recipe"]["capture_inventory_sha256"]:
        raise ValueError("Constructor implementation changed during source packaging")
    retained["semantic_bundle.py"] += "\nRETAINED_CONSTRUCTION_SOURCES = " + repr(sources) + "\n"
    retained["semantic_bundle.py"] += "\nRETAINED_CONSTRUCTION_BINDING = " + repr(binding) + "\n"


def bootstrap(source_variable):
    return ("import ast as _construction_ast, types as _construction_types, sys as _construction_sys, hashlib as _construction_hashlib, json as _construction_json\n"
        f"_construction_rows = [n for n in _construction_ast.parse({source_variable}).body if isinstance(n, _construction_ast.Assign) and len(n.targets) == 1 and isinstance(n.targets[0], _construction_ast.Name) and n.targets[0].id in ('RETAINED_CONSTRUCTION_SOURCES', 'RETAINED_CONSTRUCTION_BINDING')]\n"
        "_construction_nodes = {n.targets[0].id: n.value for n in _construction_rows}\n"
        "if len(_construction_rows) != 2 or len(_construction_nodes) != 2:\n    raise ValueError('Missing retained constructor source binding')\n"
        "_construction_sources = _construction_ast.literal_eval(_construction_nodes['RETAINED_CONSTRUCTION_SOURCES'])\n"
        "_construction_binding = _construction_ast.literal_eval(_construction_nodes['RETAINED_CONSTRUCTION_BINDING'])\n"
        "_construction_inventory = {name: _construction_hashlib.sha256(source.encode()).hexdigest() for name, source in _construction_sources.items()}\n"
        "if _construction_inventory != _construction_binding['capture_inventory'] or _construction_hashlib.sha256(_construction_json.dumps(_construction_inventory, sort_keys=True, separators=(',', ':')).encode()).hexdigest() != _construction_binding['recipe']['capture_inventory_sha256']:\n    raise ValueError('Retained constructor implementation hash mismatch')\n"
        f"for _construction_name in {RUNTIME_MODULES!r}:\n"
        "    _construction_module = _construction_types.ModuleType(_construction_name)\n"
        "    _construction_sys.modules[_construction_name] = _construction_module\n"
        "    exec(compile(_construction_sources[_construction_name + '.py'], _construction_name, 'exec'), _construction_module.__dict__)\n")


def bind_bootstrap(request, retained, policy, shared_capture_policy):
    if policy is None:
        return
    if shared_capture_policy is not None:
        # The existing capture preamble owns the one retained literal/seed.
        nodes = ast.parse(request["code"]).body
        first = nodes[0]
        if (not isinstance(first, ast.Assign) or len(first.targets) != 1
                or not isinstance(first.targets[0], ast.Name)
                or first.targets[0].id != "_shared_capture_retained_bundle"):
            raise ValueError("Shared capture seed boundary changed")
        lines = request["code"].splitlines(keepends=True)
        lines[first.end_lineno:first.end_lineno] = [bootstrap("_shared_capture_retained_bundle")]
        request["code"] = "".join(lines)
    else:
        request["code"] = ("_construction_retained_bundle = " + repr(retained["semantic_bundle.py"]) + "\n"
                           + bootstrap("_construction_retained_bundle") + request["code"])


def source_write(prefix, name, source, policy, shared_capture_policy):
    from shared_capture_adapter import source_write as capture_write
    if name == "semantic_bundle.py" and policy is not None and shared_capture_policy is None:
        return f"Path(beaver_output({(prefix + name)!r})).write_text(_construction_retained_bundle)\n"
    return capture_write(prefix, name, source, shared_capture_policy)

"""Build a fresh Beaver request; never launch Blender or reuse final 662/664."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
from controls import pivot_from_style
from prompt_contract import select_intent

HERE = Path(__file__).resolve().parent
RECIPE = HERE.parent / "checkpoints" / "current"


def load_builder(path):
    spec = importlib.util.spec_from_file_location("isolated_recipe_bundle", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate_prefix(prefix):
    if not isinstance(prefix, str) or not re.fullmatch(r"assets/aster/general_face_[a-z0-9_]+/", prefix):
        raise ValueError("Use a new assets/aster/general_face_<case>/ output directory")
    return prefix


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Upstream contract changed: " + old[:80])
    return source.replace(old, new, 1)


def build_request(intent, output_prefix, workspace=None):
    config = select_intent(intent)
    out = validate_prefix(output_prefix)
    base = load_builder(RECIPE / "bundle.py")
    style = json.loads((RECIPE / "face_style.json").read_text())
    pivot = pivot_from_style(style)
    # All adaptation happens in an isolated snapshot; upstream files and shared
    # STYLE objects remain untouched. Snapshot manifests describe executed code.
    pipeline = (RECIPE / "pipeline.py").read_text()
    pipeline = replace_once(pipeline, f'OUT = "{base.OUTPUT_PREFIX}"', f'OUT = "{out}"')
    pipeline = replace_once(pipeline,
        'tmp = Path(beaver_output(OUT + "aster_head.glb")).parent / "runtime_new.glb"',
        'tmp = Path(beaver_output(OUT + "general_base_export.glb"))')
    pipeline = replace_once(pipeline, 'tmp.unlink()', '# Retain declared fresh face-only export.')
    guard_source = (HERE / "definition_guard.py").read_text()
    pipeline = replace_once(pipeline,
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
        f'exec(compile({guard_source!r}, "definition_guard.py", "exec"))\n'
        + 'require_head_only_definition(definition)\n'
        + 'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)')
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        for name in base.SOURCE_FILES:
            (root / name).write_text(pipeline if name == "pipeline.py" else (RECIPE / name).read_text())
        base.OUTPUT_PREFIX = out
        request = base.build_request(root)
    extra = ["general_base_export.glb", "general_face_export.glb", "controls.py",
             "prompt_contract.py", "postprocess_blender.py", "general_intent.json",
             "general_manifest.json", "general_bundle.py", "definition_guard.py", "glb_affine.py", "glb_node_affine.py", "general_face_transported.glb"]
    request["outputs"].extend({"path": out + name, "expectedSha256": None} for name in extra)
    declared = [entry["path"] for entry in request["outputs"]]
    source = (HERE / "controls.py").read_text()
    request["code"] += (
        '\n_general_module = types.ModuleType("general_face_controls")\n'
        'sys.modules["general_face_controls"] = _general_module\n'
        f'exec(compile({source!r}, "controls.py", "exec"), _general_module.__dict__)\n'
        + '\n_glb_module = types.ModuleType("general_face_glb_affine")\n'
        + 'sys.modules["general_face_glb_affine"] = _glb_module\n'
        + f'exec(compile({(HERE / "glb_affine.py").read_text()!r}, "glb_affine.py", "exec"), _glb_module.__dict__)\n'
        + '\n_node_module = types.ModuleType("general_face_glb_node_affine")\n'
        + 'sys.modules["general_face_glb_node_affine"] = _node_module\n'
        + f'exec(compile({(HERE / "glb_node_affine.py").read_text()!r}, "glb_node_affine.py", "exec"), _node_module.__dict__)\n'
        + (HERE / "postprocess_blender.py").read_text()
        + f'\napply_general_face({config!r}, {pivot!r}, {out!r}, {declared!r})\n'
    )
    manifest = {}
    for name, local in [("definition_guard.py", "definition_guard.py"), ("controls.py", "controls.py"), ("prompt_contract.py", "prompt_contract.py"),
                        ("postprocess_blender.py", "postprocess_blender.py"), ("general_bundle.py", "bundle.py"), ("glb_affine.py", "glb_affine.py"), ("glb_node_affine.py", "glb_node_affine.py")]:
        source = (HERE / local).read_text()
        manifest[name] = hashlib.sha256(source.encode()).hexdigest()
        request["code"] += f'Path(beaver_output({(out + name)!r})).write_text({source!r})\n'
    for name, data in [("general_intent.json", config), ("general_manifest.json", manifest)]:
        request["code"] += f'Path(beaver_output({(out + name)!r})).write_text({json.dumps(data, indent=2)!r})\n'
    if workspace:
        for entry in request["outputs"]:
            path = Path(workspace) / entry["path"]
            if path.is_file():
                entry["expectedSha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    compile(request["code"], "<general face native job>", "exec")
    return request


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--intent", required=True, help="Explicit JSON preset or proportions")
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workspace", type=Path)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build_request(json.loads(args.intent), args.prefix, args.workspace)))

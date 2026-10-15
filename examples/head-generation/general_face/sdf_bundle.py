"""Fresh proportion-aware atlas from the bundled NPR baker, without upstream edits."""
import argparse
import hashlib
import json
from pathlib import Path
from bundle import HERE, RECIPE, validate_prefix, replace_once
from prompt_contract import select_intent

NPR = HERE.parents[2] / "resources/packages/npr-characters/addons/npr_character_frame/.ci_script/tools/sdf360"


def adapted_bake(config, out):
    source = (NPR / "bake.py").read_text()
    source = replace_once(source, 'OUT = "assets/aster/sdf_664/"', f'OUT = "{out}sdf/"')
    source = source.replace('assets/aster/head_recovery_662/', out)
    source = source.replace('assets/aster/head_recovery_664/', out)
    p = config["proportions"]
    # Blender axes x/y/z are width/depth/height; Godot proxy axes are
    # width/height/depth. The authored UV field stays fixed under the map.
    anchor = 'normals /= np.linalg.norm(normals, axis=-1, keepdims=True)'
    mapping = ''
    if any(v != 1.0 for v in p.values()):
        mapping = f'normals /= np.array({[p["width"], p["height"], p["depth"]]!r})\n'
    source = replace_once(source, anchor, mapping + anchor)
    source = replace_once(source, '"version": 664,', '"version": "general_face_v1",')
    source = replace_once(source, '"geometry_source": 662,', f'"geometry_source": {out!r},\n    "general_face_intent": {config!r},')
    return source


def build_request(intent, output_prefix, workspace):
    config = select_intent(intent)
    out = validate_prefix(output_prefix)
    if workspace is None:
        raise ValueError("A current generated workspace is required for SDF output hashes")
    root = Path(workspace)
    if not root.is_dir():
        raise ValueError("SDF workspace must be an existing directory")
    required = ["project.godot", out + "aster_head.glb",
                out + "aster_definition.tres", out + "generation_report.json"]
    missing = [path for path in required if not (root / path).is_file()]
    if missing:
        raise ValueError("SDF workspace is missing required generation inputs: " + ", ".join(missing))
    style = json.loads((RECIPE / "face_style.json").read_text())
    sources = {"face": (RECIPE / "face.py").read_text(),
               "distance_field": (NPR / "distance_field.py").read_text(),
               "definition_output": (NPR / "definition_output.py").read_text()}
    bake = adapted_bake(config, out)
    code = 'import sys, types, json, hashlib\nfrom pathlib import Path\n'
    code += f'_general_report_path = Path(beaver_input({(out + "generation_report.json")!r}))\n'
    code += '_general_report = json.loads(_general_report_path.read_text())\n'
    code += f'assert _general_report["general_face"]["intent"] == {config!r}, "Geometry/atlas intent mismatch"\n'
    code += f'assert hashlib.sha256(Path(beaver_input({(out + "aster_head.glb")!r})).read_bytes()).hexdigest() == _general_report["new_glb_sha256"], "Geometry hash mismatch"\n'
    for name, source in sources.items():
        code += f'm = types.ModuleType({name!r}); sys.modules[{name!r}] = m; m.STYLE = {style!r}\n'
        code += f'exec(compile({source!r}, {name!r}, "exec"), m.__dict__)\n'
    code += bake
    code += f'\n_general_report["general_face"]["sdf_status"] = "fresh proportion-aware atlas baked; visual validation pending"\n'
    code += '_general_report["general_face"]["sdf_atlas_sha256"] = hashlib.sha256(Path(beaver_output(OUT + "face_sdf360.png")).read_bytes()).hexdigest()\n'
    code += f'Path(beaver_output({(out + "generation_report.json")!r})).write_text(json.dumps(_general_report, indent=2))\n'
    artifacts = {"bake.py": bake, "sdf_bundle.py": (HERE / "sdf_bundle.py").read_text()}
    artifacts.update({name + ".py": source for name, source in sources.items()})
    paths = [out + "sdf/face_sdf360.png", out + "sdf/bake_report.json",
             out + "aster_definition.tres", out + "generation_report.json"]
    manifest = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in artifacts.items()}
    artifacts["source_manifest.json"] = json.dumps(manifest, indent=2)
    for name, source in artifacts.items():
        path = out + "sdf/" + name
        paths.append(path)
        code += f'Path(beaver_output({path!r})).write_text({source!r})\n'
    outputs = []
    for path in paths:
        local = root / path
        if local.exists() and not local.is_file():
            raise ValueError("SDF output path is not a regular file: " + path)
        digest = hashlib.sha256(local.read_bytes()).hexdigest() if local.is_file() else None
        if path in required and digest is None:
            raise ValueError("Required generated output disappeared while packaging: " + path)
        outputs.append({"path": path, "expectedSha256": digest})
    compile(code, "<general face native SDF job>", "exec")
    return {"code": code, "outputs": outputs, "timeoutSeconds": 240}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--intent", required=True)
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True,
                        help="Current generated project workspace; required to hash overwritten outputs")
    args = parser.parse_args()
    args.output.write_text(json.dumps(build_request(json.loads(args.intent), args.prefix, args.workspace)))

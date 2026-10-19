"""Package fresh directional SDF with explicit geometry-fitted NPR shading policy."""

import argparse
import hashlib
import json
from pathlib import Path
import re
from bundle import HERE, replace_once
from parameters import validate
from receipt_binding import bind_generation_receipt

NPR = (
    HERE.parents[2]
    / "resources/packages/npr-characters/addons/npr_character_frame/.ci_script/tools/sdf360"
)
GENERAL = HERE.parent / "general_face"


def adapted_bake(out, shading_policy="smooth_proxy"):
    if shading_policy not in ("smooth_proxy", "mesh_normals"):
        raise ValueError("Unknown SDF shading policy")
    module = "shading_proxy" if shading_policy == "smooth_proxy" else "mesh_normals"
    source = (NPR / "bake.py").read_text()
    source = replace_once(
        source,
        "from face import depth, nose, lip_detail_depth, STYLE",
        f"from {module} import from_runtime_blend",
    )
    source = replace_once(
        source, 'OUT = "assets/aster/sdf_664/"', f'OUT = {out + "sdf/"!r}'
    )
    source = source.replace("assets/aster/head_recovery_662/", out)
    source = source.replace("assets/aster/head_recovery_664/", out)
    begin = source.index("# Godot UV2 top-origin:")
    end = source.index("atlas = np.zeros(", begin)
    source = (
        source[:begin]
        + f'normals, mesh_normal_report = from_runtime_blend(ROOT / {(out + "aster_head_runtime_morph.blend")!r}, N)\n'
        + source[end:]
    )
    source = replace_once(
        source, '"version": 664,', f'"version": "semantic_eyes_{shading_policy}_v1",'
    )
    source = replace_once(
        source,
        '"geometry_source": 662,',
        f'"geometry_source": {out!r},\n    "mesh_normal_report": mesh_normal_report,',
    )
    lines = source.splitlines()
    for i, line in enumerate(lines):
        if line.strip().startswith('"method":'):
            lines[i] = (
                '    "method": '
                + json.dumps(
                    "Geometry-fitted smooth art-directed convex shading proxy; excludes eye/nose/lip microrelief; fresh directional EDT; not physical mesh normals."
                    if shading_policy == "smooth_proxy"
                    else "Actual neutral runtime front-shell UV2 corner normals; explicit uncovered-pixel continuation; fresh directional EDT."
                )
                + ","
            )
    return "\n".join(lines) + "\n"


def build_request(
    parameters,
    output_prefix,
    workspace,
    generation_receipt,
    shading_policy="smooth_proxy",
    allow_workspace_copy=False,
):
    p = validate(parameters)
    out = output_prefix
    if not re.fullmatch(r"assets/aster/general_face_eyes_[a-z0-9_]+/", out):
        raise ValueError("Expected semantic eye output prefix")
    root = Path(workspace)
    required = ["project.godot"] + [
        out + name
        for name in (
            "aster_head_runtime_morph.blend",
            "aster_head.glb",
            "generation_report.json",
            "aster_definition.tres",
            "semantic_parameters.json",
        )
    ]
    missing = [name for name in required if not (root / name).is_file()]
    if missing:
        raise ValueError("Missing generated SDF inputs: " + ", ".join(missing))
    hashes = {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in required
    }
    binding = bind_generation_receipt(
        generation_receipt, root, out, hashes, allow_workspace_copy
    )
    report = json.loads((root / out / "generation_report.json").read_text())
    saved = json.loads((root / out / "semantic_parameters.json").read_text())
    # New identity-default controls must not invalidate a trusted old generation.
    # Report and receipt-bound parameter file must still agree exactly.
    if report["semantic_eyes"]["parameters"] != saved or validate(saved) != p:
        raise ValueError("Geometry/SDF parameter mismatch")
    if (
        report["semantic_eyes"].get(
            "runtime_blend_sha256", hashes[out + "aster_head_runtime_morph.blend"]
        )
        != hashes[out + "aster_head_runtime_morph.blend"]
    ):
        raise ValueError("Geometry runtime blend hash mismatch")
    if report["new_glb_sha256"] != hashes[out + "aster_head.glb"]:
        raise ValueError("Geometry GLB hash mismatch")
    guard = (GENERAL / "definition_guard.py").read_text()
    scope = {}
    exec(guard, scope)
    scope["require_head_only_definition"](
        (root / out / "aster_definition.tres").read_text()
    )
    sources = {
        "mesh_normals": (HERE / "mesh_normals.py").read_text(),
        "shading_proxy": (HERE / "shading_proxy.py").read_text(),
        "distance_field": (NPR / "distance_field.py").read_text(),
        "definition_output": (NPR / "definition_output.py").read_text()
        + "\n"
        + (GENERAL / "sdf_definition.py").read_text(),
    }
    bake = adapted_bake(out, shading_policy)
    code = "import sys, types, hashlib, json\nfrom pathlib import Path\n"
    code += f"for _path, _sha in {hashes!r}.items():\n"
    code += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "SDF input changed: " + _path\n'
    code += f'exec(compile({guard!r}, "definition_guard.py", "exec"))\n'
    code += f'require_head_only_definition(Path(beaver_input({out + "aster_definition.tres"!r})).read_text())\n'
    for name, source in sources.items():
        code += f"m = types.ModuleType({name!r}); sys.modules[{name!r}] = m\n"
        code += f'exec(compile({source!r}, {name + ".py"!r}, "exec"), m.__dict__)\n'
    code += bake
    code += f"_generation_binding = {binding!r}\n"
    code += f'_generation_report = json.loads(Path(beaver_input({out + "generation_report.json"!r})).read_text())\n'
    code += f'_generation_report["semantic_eyes"]["sdf"] = {{"policy": {shading_policy!r}, "generation_binding": _generation_binding, "normal_field": mesh_normal_report, "visual_acceptance": "pending", "atlas_sha256": hashlib.sha256(Path(beaver_output(OUT + "face_sdf360.png")).read_bytes()).hexdigest()}}\n'
    code += f'Path(beaver_output({out + "generation_report.json"!r})).write_text(json.dumps(_generation_report, indent=2))\n'
    artifacts = {name + ".py": source for name, source in sources.items()}
    artifacts.update(
        {
            "bake.py": bake,
            "sdf_bundle.py": Path(__file__).read_text(),
            "definition_guard.py": guard,
            "generation_binding.json": json.dumps(binding, indent=2),
            "receipt_binding.py": (HERE / "receipt_binding.py").read_text(),
            "input_hashes.json": json.dumps(hashes, indent=2),
        }
    )
    artifacts["source_manifest.json"] = json.dumps(
        {
            name: hashlib.sha256(text.encode()).hexdigest()
            for name, text in artifacts.items()
        },
        indent=2,
    )
    paths = [
        out + name
        for name in (
            "sdf/face_sdf360.png",
            "sdf/bake_report.json",
            "aster_definition.tres",
            "generation_report.json",
        )
    ]
    for name, source in artifacts.items():
        path = out + "sdf/" + name
        paths.append(path)
        code += f"Path(beaver_output({path!r})).write_text({source!r})\n"
    outputs = []
    for path in paths:
        local = root / path
        if local.exists() and not local.is_file():
            raise ValueError("Expected regular output file: " + path)
        outputs.append(
            {
                "path": path,
                "expectedSha256": (
                    hashlib.sha256(local.read_bytes()).hexdigest()
                    if local.is_file()
                    else None
                ),
            }
        )
    compile(code, "<semantic mesh-normal SDF job>", "exec")
    return {"code": code, "outputs": outputs, "timeoutSeconds": 240}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parameters", required=True)
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--generation-receipt", type=Path, required=True)
    parser.add_argument(
        "--allow-workspace-copy",
        action="store_true",
        help="Require original receipt hashes for all three immutable copied geometry outputs",
    )
    parser.add_argument(
        "--shading-policy",
        choices=("smooth_proxy", "mesh_normals"),
        default="smooth_proxy",
    )
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            build_request(
                json.loads(args.parameters),
                args.prefix,
                args.workspace,
                args.generation_receipt,
                args.shading_policy,
                args.allow_workspace_copy,
            )
        )
    )

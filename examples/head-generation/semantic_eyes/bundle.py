"""Package a fresh semantic eye recipe for Beaver; never execute a model here."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
from parameters import adapt_style, validate

HERE = Path(__file__).resolve().parent
RECIPE = HERE.parent / "checkpoints/current"
REQUIRED = ["project.godot", "authoring/head_generation_guide.md"] + [
    "assets/aster/head_recovery_49/" + name
    for name in (
        "aster_head_editable.blend",
        "aster_head.glb",
        "aster_definition.tres",
        "hair_base.png",
        "hair_ilm.png",
    )
]


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Inherited source contract changed: " + old[:70])
    return source.replace(old, new, 1)


def build_request(parameters, output_prefix, workspace):
    p = validate(parameters)
    if not re.fullmatch(r"assets/aster/general_face_eyes_[a-z0-9_]+/", output_prefix):
        raise ValueError("Use assets/aster/general_face_eyes_<case>/")
    root = Path(workspace)
    missing = [name for name in REQUIRED if not (root / name).is_file()]
    if missing:
        raise ValueError("Missing fresh recipe inputs: " + ", ".join(missing))
    spec = importlib.util.spec_from_file_location(
        "semantic_recipe_builder", RECIPE / "bundle.py"
    )
    base = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(base)
    style = adapt_style(json.loads((RECIPE / "face_style.json").read_text()), p)
    # This recipe does not restore coordinates from any accepted final head.
    base.MODULES = [
        m for m in base.MODULES if m not in ("preserve_accepted", "eye_rim_refine")
    ]
    base.MODULES.append("semantic_eye_runtime")
    base.SOURCE_FILES = [
        n
        for n in base.SOURCE_FILES
        if n not in ("preserve_accepted.py", "eye_rim_refine.py")
    ]
    base.SOURCE_FILES.append("semantic_eye_runtime.py")
    pipeline = (RECIPE / "pipeline.py").read_text()
    pipeline = replace_once(
        pipeline, 'OUT = "' + base.OUTPUT_PREFIX + '"', "OUT = " + repr(output_prefix)
    )
    pipeline = replace_once(
        pipeline,
        "        from preserve_accepted import preserve_non_eye\n"
        '        report["accepted_non_eye_preservation"] = preserve_non_eye(data,ROOT,report)\n'
        "        from eye_rim_refine import refine_outer_eye\n"
        '        report["outer_eye_local_depth"] = refine_outer_eye(data)',
        '        report["semantic_eye_final_restore"] = "none; fresh procedural geometry"',
    )
    pipeline = replace_once(
        pipeline,
        "PARTS[:] = stable_part_order(PARTS)",
        "PARTS[:] = stable_part_order(PARTS)\n"
        "semantic_part_ordinals = {part.name: i + 10 for i, part in enumerate(PARTS)}\n"
        "from semantic_eye_runtime import select_components\n"
        f'report["semantic_eye_selection"] = select_components(PARTS, {p!r})\n'
        f'report["semantic_eyes"] = {{"parameters": {p!r}, "native_acceptance": "pending", '
        '"scope": "eye-family only; regenerated procedural non-eye baseline"}',
    )
    pipeline = replace_once(
        pipeline,
        "        region = part_index + 10",
        "        region = semantic_part_ordinals[part.name]",
    )
    if p["supplemental_lashes"] == 0:
        pipeline = replace_once(
            pipeline,
            'report["eye_accent_connections"] = measure_accent_connections()',
            'report["eye_accent_connections"] = {"status": "not_applicable", "reason": "all supplemental child parts are removed before delivery; main band obstacle checks remain active"}',
        )
    guard = (HERE.parent / "general_face/definition_guard.py").read_text()
    pipeline = replace_once(
        pipeline,
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
        f'exec(compile({guard!r}, "definition_guard.py", "exec"))\n'
        "require_head_only_definition(definition)\n"
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
    )
    pipeline = replace_once(
        pipeline,
        'tmp = Path(beaver_output(OUT + "aster_head.glb")).parent / "runtime_new.glb"',
        'tmp = Path(beaver_output(OUT + "semantic_face_export.glb"))',
    )
    pipeline = replace_once(
        pipeline, "tmp.unlink()", "# Retain declared same-job fresh Face export."
    )
    pipeline = replace_once(
        pipeline,
        'Path(beaver_output(OUT + "generation_report.json")).write_text(json.dumps(report, indent=2))',
        "from semantic_eye_runtime import final_runtime_evidence\n"
        'report["semantic_eyes"].update(final_runtime_evidence(beaver_output(OUT + "aster_head_runtime_morph.blend")))\n'
        'Path(beaver_output(OUT + "generation_report.json")).write_text(json.dumps(report, indent=2))',
    )
    sources = {
        name: (RECIPE / name).read_text()
        for name in base.SOURCE_FILES
        if name != "semantic_eye_runtime.py"
    }
    sources.update(
        {
            "face_style.json": json.dumps(style, indent=2),
            "pipeline.py": pipeline,
            "semantic_eye_runtime.py": (HERE / "runtime.py").read_text(),
        }
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        '    image.filepath = "//" + role.lower() + "_base.png"',
        "    # Keep the same-job absolute atlas path; the inherited blend is still open.\n"
        "    image.pack()",
    )
    sources["eye_socket.py"] = replace_once(
        sources["eye_socket.py"],
        "/ 0.044 + 0.5",
        f'/ (0.044 * {p["iris_height"]!r}) + 0.5',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "(v - 0.5) * 0.044",
        f'(v - 0.5) * (0.044 * {p["iris_height"]!r})',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "iv = y / 0.036 + 0.5",
        f'iv = y / (0.036 * {p["iris_height"]!r}) + 0.5',
    )
    sources["face.py"] = replace_once(
        sources["face.py"],
        '                        * C["lower_cover"]\n                        * H',
        '                        * C["lower_cover"]\n'
        '                        * STYLE.get("lower_lash_width_scale", 1.0)\n                        * H',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "outline = 0.16 * exp(-(((disc - 1) / 0.055) ** 2))",
        'outline = 0.16 * min(1., STYLE["iris_graphic_disc_strength"] / .30) * exp(-(((disc - 1) / 0.055) ** 2))',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        ")) / 0.2)) * 0.7",
        ')) / 0.2)) * STYLE.get("iris_accent_strength", .7)',
    )
    base.OUTPUT_PREFIX = output_prefix
    with tempfile.TemporaryDirectory() as directory:
        snapshot = Path(directory)
        for name, source in sources.items():
            (snapshot / name).write_text(source)
        request = base.build_request(snapshot)
    input_hashes = {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in REQUIRED
    }
    verification = "import hashlib\nfrom pathlib import Path\n"
    verification += f"for _path, _sha in {input_hashes!r}.items():\n"
    verification += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "Recipe input changed: " + _path\n'
    request["code"] = verification + request["code"]
    retained = {
        "semantic_parameters.json": json.dumps(p, indent=2),
        "semantic_bundle.py": Path(__file__).read_text(),
        "semantic_parameters.py": (HERE / "parameters.py").read_text(),
        "definition_guard.py": guard,
        "semantic_input_hashes.json": json.dumps(
            {
                name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                for name in REQUIRED
            },
            indent=2,
        ),
    }
    for name, source in retained.items():
        request[
            "code"
        ] += f"Path(beaver_output({(output_prefix + name)!r})).write_text({source!r})\n"
    for name in [*retained, "semantic_face_export.glb"]:
        request["outputs"].append(
            {"path": output_prefix + name, "expectedSha256": None}
        )
    for entry in request["outputs"]:
        path = root / entry["path"]
        if path.exists():
            if not path.is_file():
                raise ValueError("Non-file output: " + entry["path"])
            entry["expectedSha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    compile(request["code"], "<semantic eye native job>", "exec")
    return request


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parameters", required=True, help="JSON numeric controls")
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            build_request(json.loads(args.parameters), args.prefix, args.workspace)
        )
    )

"""Two managed jobs keep every source declared within Beaver's 64-output limit."""

import ast
import copy
import hashlib
import json
from pathlib import Path
from .layout import digest, pipeline_layout, parameter_layout
from .evidence import HELPERS, native_job, resource
import re


def literal_writes(code):
    found = {}
    for node in ast.walk(ast.parse(code)):
        if not isinstance(node, ast.Expr) or not isinstance(node.value, ast.Call):
            continue
        call = node.value
        if not isinstance(call.func, ast.Attribute) or call.func.attr != "write_text":
            continue
        if (
            len(call.args) != 1
            or not isinstance(call.args[0], ast.Constant)
            or not isinstance(call.args[0].value, str)
        ):
            continue
        path = call.func.value
        if (
            not isinstance(path, ast.Call)
            or not isinstance(path.func, ast.Name)
            or path.func.id != "Path"
            or len(path.args) != 1
        ):
            continue
        output = path.args[0]
        if (
            not isinstance(output, ast.Call)
            or not isinstance(output.func, ast.Name)
            or output.func.id != "beaver_output"
            or len(output.args) != 1
        ):
            continue
        value = output.args[0]
        if isinstance(value, ast.Constant) and isinstance(value.value, str):
            if value.value in found:
                raise ValueError("Duplicate literal source output")
            found[value.value] = (
                call.args[0].value,
                ast.get_source_segment(code, node),
            )
    return found


def modular_plan(request, prefix):
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Invalid composition prefix")
    request = copy.deepcopy(request)
    writes = literal_writes(request["code"])
    replacements = {}
    helpers = {}
    records = []
    original_pipeline = None
    stages = None
    for filename, splitter in [
        ("pipeline.py", pipeline_layout),
        ("semantic_parameters.py", parameter_layout),
    ]:
        if prefix + filename not in writes:
            raise ValueError("Missing retained source output: " + filename)
        source, expression = writes[prefix + filename]
        files, record = splitter(source)
        records.append({"path": filename, **record})
        replacements[expression] = (
            f"Path(beaver_output({prefix + filename!r})).write_text({files[filename]!r})"
        )
        helpers.update({prefix + k: v for k, v in files.items() if k != filename})
        if filename == "pipeline.py":
            if record["prefix"] != prefix:
                raise ValueError("Pipeline OUT differs from requested prefix")
            original_pipeline = source
            stages = [(x["path"], files[x["path"]]) for x in record["stages"]]
    if request["code"].count(original_pipeline) != 1:
        raise ValueError("Expected one executable pipeline body")
    runner = f"for _pipeline_stage_name, _pipeline_stage_source in {stages!r}:\n"
    runner += '    exec(compile(_pipeline_stage_source, _pipeline_stage_name, "exec"), globals())\n'
    request["code"] = request["code"].replace(original_pipeline, runner, 1)
    for old, new in replacements.items():
        if request["code"].count(old) != 1:
            raise ValueError("Literal output expression changed")
        request["code"] = request["code"].replace(old, new, 1)
    source_manifest, expression = writes[prefix + "input-manifest.json"]
    manifest = json.loads(source_manifest)
    for record in records:
        manifest[record["path"]] = record["entry_sha256"]
    manifest.update(
        {path.removeprefix(prefix): digest(source) for path, source in helpers.items()}
    )
    request["code"] = request["code"].replace(
        expression,
        f'Path(beaver_output({prefix + "input-manifest.json"!r})).write_text({json.dumps(manifest,indent=2)!r})',
        1,
    )
    layout = {
        "schema_version": 1,
        "records": records,
        "helper_sha256": {path: digest(source) for path, source in helpers.items()},
        "scope": "Ordinary Python source modules, all declared across generation and source-packaging jobs; original recipes unchanged.",
    }
    request["code"] += f'report["source_layout"] = {layout!r}\n'
    request[
        "code"
    ] += f'Path(beaver_output({prefix + "generation_report.json"!r})).write_text(json.dumps(report, indent=2))\n'
    compile(request["code"], "<modular generation>", "exec")
    if (
        len(request["outputs"]) > 64
        or len(request["code"].encode("utf-8")) > 1024 * 1024
    ):
        raise ValueError("Native generation request limit exceeded")
    return {
        "generation": request,
        "source_packaging": {
            "prefix": prefix,
            "layout": layout,
            "helpers": helpers,
            "generation_outputs": [row["path"] for row in request["outputs"]],
            "generation_script_sha256": digest(request["code"]),
        },
    }


def packaging_request(plan, receipt_path, workspace):
    root = Path(workspace).resolve()
    prefix = plan["prefix"]
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Invalid composition prefix")
    raw = Path(receipt_path).read_bytes()
    job, outputs = native_job(receipt_path, root)
    if (
        set(outputs) != set(plan["generation_outputs"])
        or job["process"]["scriptSha256"] != plan["generation_script_sha256"]
    ):
        raise ValueError("Native generation differs from planned request")
    required = {
        prefix + name
        for name in (
            "generation_report.json",
            "pipeline.py",
            "semantic_parameters.py",
            "input-manifest.json",
            "aster_head.glb",
            "aster_head_runtime_morph.blend",
        )
    }
    if not required <= set(outputs):
        raise ValueError("Required native generation outputs missing")
    for path, sha in outputs.items():
        if (
            not path.startswith(prefix)
            or hashlib.sha256(resource(root, path).read_bytes()).hexdigest() != sha
        ):
            raise ValueError("Changed generation output")
    report = json.loads(resource(root, prefix + "generation_report.json").read_text())
    if report.get("source_layout") != plan["layout"]:
        raise ValueError("Source plan not bound to native report")
    wanted = plan["layout"]["helper_sha256"]
    if set(wanted) != {prefix + name for name in HELPERS}:
        raise ValueError("Unexpected helper path")
    if {p: digest(s) for p, s in plan["helpers"].items()} != wanted:
        raise ValueError("Changed planned helper source")
    records = plan["layout"]["records"]
    if len(records) != 2 or {r["path"] for r in records} != {
        "pipeline.py",
        "semantic_parameters.py",
    }:
        raise ValueError("Unexpected modular source entries")
    for record in records:
        if outputs[prefix + record["path"]] != record["entry_sha256"]:
            raise ValueError("Modular entry differs from native output")
    binding = {
        "generation_job": job["jobId"],
        "generation_receipt_sha256": hashlib.sha256(raw).hexdigest(),
        "generation_outputs": outputs,
        "helper_sha256": wanted,
    }
    source = "import hashlib\nfrom pathlib import Path\n"
    source += f"for _path, _sha in {outputs!r}.items():\n"
    source += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "Generation changed before source packaging"\n'
    payload = {
        **plan["helpers"],
        prefix + "source_layout_binding.json": json.dumps(binding, indent=2),
    }
    declared = []
    for path, content in payload.items():
        if resource(root, path).exists():
            raise ValueError("Source packaging requires fresh outputs")
        source += f"Path(beaver_output({path!r})).write_text({content!r})\n"
        declared.append({"path": path, "expectedSha256": None})
    if len(declared) > 64 or len(source.encode()) > 1024 * 1024:
        raise ValueError("Native packaging limit exceeded")
    return {"code": source, "outputs": declared, "timeoutSeconds": 120}

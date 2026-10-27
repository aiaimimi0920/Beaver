"""Default two-job source packaging, with an explicit optional seed job."""

import ast
import copy
import hashlib
import json
from pathlib import Path
from .layout import digest, pipeline_layout, parameter_layout, reconstructed
from .surface_layout import surface_layout
from .module_layout import module_layout
from .evidence import HELPERS, native_job, resource, validate_layout_set
from .transport_sources import share_source_literals, validate_native_frame
from .source_seed import seed_retained_bundle
from .text_outputs import canonical_text_outputs
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


def modular_plan(request, prefix, *, source_seed=False):
    if not isinstance(source_seed, bool):
        raise TypeError("source_seed must be a boolean")
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Invalid composition prefix")
    request = copy.deepcopy(request)
    writes = literal_writes(request["code"])
    replacements = {}
    helpers = {}
    records = []
    original_pipeline = None
    stages = None
    splitters=[("pipeline.py",pipeline_layout),("semantic_parameters.py",parameter_layout)]
    if prefix+"ear_surface.py" in writes and "def polygon_diagonals(" in writes[prefix+"ear_surface.py"][0]:
        splitters.append(("ear_surface.py",surface_layout))
    if prefix + "face.py" in writes and "def capture_oral_skin_donors(" in writes[prefix + "face.py"][0]:
        for filename in ("face.py", "shell.py", "ear_topology.py"):
            splitters.append((filename, lambda source, name=filename: module_layout(source, name)))
    for filename, splitter in splitters:
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
    request["code"] = canonical_text_outputs(request["code"], prefix)
    request["code"] = share_source_literals(request["code"], minimum=2048)
    seed, seed_binding = None, None
    if source_seed:
        request["code"], seed, seed_binding = seed_retained_bundle(request["code"], prefix)
        validate_native_frame(seed)
    compile(request["code"], "<modular generation>", "exec")
    if (
        len(request["outputs"]) > 64
        or len(request["code"].encode("utf-8")) > 1024 * 1024
    ):
        raise ValueError("Native generation request limit exceeded")
    validate_native_frame(request)
    result = {
        "generation": request,
        "source_packaging": {
            "prefix": prefix,
            "layout": layout,
            "helpers": helpers,
            "generation_outputs": [row["path"] for row in request["outputs"]],
            "generation_script_sha256": digest(request["code"]),
        },
    }
    if source_seed:
        result["source_seed"] = seed
        result["source_seed_binding"] = seed_binding
        result["source_packaging"]["source_seed_binding"] = seed_binding
    return result


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
    seed_binding = plan.get("source_seed_binding")
    seed_receipt = None
    if seed_binding:
        from .seed_evidence import verify_seed, verify_retained_source
        seed_receipt = Path(plan["source_seed_receipt_path"])
        if verify_seed(seed_binding, seed_receipt, root) != seed_binding or report.get("authored_source_seed") != seed_binding:
            raise ValueError("Generation source seed chain changed")
        verify_retained_source(seed_binding,root,prefix + "semantic_bundle.py")
    if report.get("source_layout") != plan["layout"]:
        raise ValueError("Source plan not bound to native report")
    wanted = plan["layout"]["helper_sha256"]
    validate_layout_set(plan["layout"], prefix)
    if {p: digest(s) for p, s in plan["helpers"].items()} != wanted:
        raise ValueError("Changed planned helper source")
    records = plan["layout"]["records"]
    validate_layout_set(plan["layout"], prefix)
    for record in records:
        if outputs[prefix + record["path"]] != record["entry_sha256"]:
            raise ValueError("Modular entry differs from native output")
    for record in records:
        if record.get('kind')=='surface_rules':
            reconstructed({'ear_surface.py':resource(root,prefix+'ear_surface.py').read_text(),
                           'source_mesh_rules.py':plan['helpers'][prefix+'source_mesh_rules.py']},record)
        elif record.get('kind') == 'ordered_module':
            files = {record['entry_name']: resource(root, prefix + record['entry_name']).read_text()}
            files.update({item['path']: plan['helpers'][prefix + item['path']] for item in record['stages']})
            reconstructed(files, record)
    binding = {
        "generation_job": job["jobId"],
        "generation_receipt_sha256": hashlib.sha256(raw).hexdigest(),
        "generation_outputs": outputs,
        "helper_sha256": wanted,
    }
    if seed_binding:
        binding["source_seed"] = seed_binding
    source = "import hashlib\nfrom pathlib import Path\n"
    source += f"for _path, _sha in {outputs!r}.items():\n"
    source += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "Generation changed before source packaging"\n'
    payload = {
        **plan["helpers"],
        prefix + "source_layout_binding.json": json.dumps(binding, indent=2),
    }
    if seed_receipt:
        seed_raw = seed_receipt.read_bytes()
        if hashlib.sha256(seed_raw).hexdigest() != seed_binding["receipt_sha256"]:
            raise ValueError("Seed receipt changed before packaging")
        payload[prefix + "source_seed_receipt.json"] = seed_raw
    declared = []
    for path, content in payload.items():
        if resource(root, path).exists():
            raise ValueError("Source packaging requires fresh outputs")
        method = "write_bytes" if isinstance(content, bytes) else "write_text"
        options = '' if isinstance(content, bytes) else ", encoding='utf-8', newline='\\n'"
        source += f"Path(beaver_output({path!r})).{method}({content!r}{options})\n"
        declared.append({"path": path, "expectedSha256": None})
    if len(declared) > 64 or len(source.encode()) > 1024 * 1024:
        raise ValueError("Native packaging limit exceeded")
    request = {"code": source, "outputs": declared, "timeoutSeconds": 120}
    validate_native_frame(request)
    return request

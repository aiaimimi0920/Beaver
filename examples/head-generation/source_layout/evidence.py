"""Verify the native job chain without fabricating or rewriting their receipts."""

import hashlib
import json
import re
from pathlib import Path

HELPERS = {
    "pipeline_geometry.py",
    "pipeline_assembly.py",
    "pipeline_runtime.py",
    "pipeline_export.py",
    "semantic_parameter_curves.py",
}


def validate_layout_set(layout, prefix):
    from .module_layout import MODULE_STAGES
    records = layout['records']
    paths = {r['path'] for r in records}
    base = {'pipeline.py', 'semantic_parameters.py'}
    ordered = set(MODULE_STAGES) & paths
    if len(paths) != len(records) or not base <= paths:
        raise ValueError('Unexpected source layout entries')
    if paths - base - {'ear_surface.py'} != ordered or ordered not in (set(), set(MODULE_STAGES)):
        raise ValueError('Incomplete or unexpected ordered module family')
    extra = {'source_mesh_rules.py'} if 'ear_surface.py' in paths else set()
    if extra:
        record = next(r for r in records if r['path'] == 'ear_surface.py')
        if record.get('kind') != 'surface_rules' or record.get('helper_sha256') != layout['helper_sha256'][prefix + 'source_mesh_rules.py']:
            raise ValueError('Surface rule module not bound to layout')
    for name in ordered:
        record = next(r for r in records if r['path'] == name)
        wanted = [path for path, _ in MODULE_STAGES[name]]
        if (record.get('kind') != 'ordered_module' or record.get('entry_name') != name
                or [item['path'] for item in record.get('stages', [])] != wanted):
            raise ValueError('Ordered source module contract changed')
        for item in record['stages']:
            if item['sha256'] != layout['helper_sha256'].get(prefix + item['path']):
                raise ValueError('Ordered module source is not hash bound')
        extra.update(wanted)
    if set(layout['helper_sha256']) != {prefix + name for name in HELPERS | extra}:
        raise ValueError('Unexpected modular source set')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def resource(root, name):
    if not isinstance(name, str) or Path(name).is_absolute():
        raise ValueError("Invalid source artifact path")
    lexical = root / name
    if any(
        p.is_symlink()
        for p in [lexical, *lexical.parents]
        if p != root and p.is_relative_to(root)
    ):
        raise ValueError("Linked source artifacts are not permitted")
    path = lexical.resolve()
    if not path.is_relative_to(root) or ".." in Path(name).parts:
        raise ValueError("Source artifact escapes workspace")
    return path


def native_job(path, root, raw_bytes=None):
    raw = json.loads(Path(path).read_bytes() if raw_bytes is None else raw_bytes)
    if "response" in raw:
        job = raw["response"]["result"]["result"]
    else:
        # A completed managed job journal is authoritative disk evidence, not an
        # invented RPC receipt. Keep it unchanged at its original workspace path.
        job = raw
        for key in ("runId", "requestId"):
            value = job[key]
            if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", value):
                raise ValueError("Durable native job identifiers must be single components")
        relative = ".beaver-context/external/{}/{}/job.json".format(
            job["runId"], job["requestId"]
        )
        journal = resource(root, relative)
        if Path(path).absolute() != journal or Path(job["process"]["jobDirectory"]).resolve() != journal.parent:
            raise ValueError("Durable native job must retain its original location")
        completed = json.loads(resource(root, str(Path(relative).with_name("completed.json"))).read_text())
        if (completed.get("completed") is not True
            or job["result"].get("ownedTreeCleanup") != "completed"
            or job["result"].get("termination") != "exited"):
            raise ValueError("Durable native job did not complete and clean up")
    if (
        job["status"] != "succeeded"
        or job["result"]["success"] is not True
        or job["result"]["exitCode"] != 0
        or Path(job["process"]["workspace"]).resolve() != root
    ):
        raise ValueError("Successful same-workspace native job required")
    rows = job["result"]["outputs"]
    hashes = {row["path"]: row["sha256"] for row in rows}
    if len(rows) != len(hashes):
        raise ValueError("Duplicate native output")
    return job, hashes


def verify_packaging(report, root, prefix, generation_receipt, packaging_receipt=None):
    root = Path(root).resolve()
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[a-z0-9_]+/", prefix):
        raise ValueError("Invalid composition prefix")
    layout = report.get("source_layout")
    if layout is None:
        if packaging_receipt is not None:
            raise ValueError("Unexpected source packaging receipt")
        return {}
    if packaging_receipt is None:
        raise ValueError("Modular generation requires native source packaging receipt")
    if layout.get("schema_version") != 1:
        raise ValueError("Unknown source layout version")
    wanted = layout["helper_sha256"]
    validate_layout_set(layout, prefix)
    gen, generated = native_job(generation_receipt, root)
    from .pack_batches import original_packaging_jobs
    packed_ids, outputs = original_packaging_jobs(packaging_receipt, root, gen["jobId"])
    binding_path = prefix + "source_layout_binding.json"
    seed_binding = report.get("authored_source_seed")
    seed_sidecar = {prefix + "source_seed_receipt.json"} if seed_binding else set()
    if seed_binding and seed_binding.get('schema_version') == 3:
        seed_sidecar.add(prefix + 'source_seed_runtime_receipt.json')
    if set(outputs) != set(wanted) | {binding_path} | seed_sidecar:
        raise ValueError("Incomplete or unexpected source packaging outputs")
    for name, digest in outputs.items():
        if sha(resource(root, name)) != digest:
            raise ValueError("Changed native packaged source")
    if any(outputs[name] != digest for name, digest in wanted.items()):
        raise ValueError("Packaged source differs from generation plan")
    binding = json.loads(resource(root, binding_path).read_text())
    expected_binding = {
        "generation_job": gen["jobId"],
        "generation_receipt_sha256": sha(generation_receipt),
        "generation_outputs": generated,
        "helper_sha256": wanted,
    }
    if seed_binding:
        from .seed_evidence import verify_seed, verify_retained_source
        if seed_binding.get("schema_version") not in (1, 2, 3):
            raise ValueError("Unknown source seed binding schema")
        receipt = resource(root, prefix + 'source_seed_receipt.json')
        if seed_binding.get('schema_version') == 3:
            receipt = [receipt, resource(root, prefix + 'source_seed_runtime_receipt.json')]
        verified = verify_seed(seed_binding, receipt, root)
        seed_jobs = {verified['job_id']}
        if verified.get('schema_version') == 3:
            seed_jobs.add(verified['runtime_job_id'])
        if seed_jobs & ({gen['jobId']} | packed_ids):
            raise ValueError("Invalid native source seed/generation chain")
        if generated.get(prefix + "semantic_bundle.py") != sha(resource(root,prefix + "semantic_bundle.py")):
            raise ValueError("Retained source differs from generation receipt")
        verify_retained_source(verified,root,prefix + "semantic_bundle.py")
        expected_binding["source_seed"] = verified
    if binding != expected_binding:
        raise ValueError("Source packaging not bound to generation receipt")
    manifest = json.loads(resource(root, prefix + "input-manifest.json").read_text())
    if generated.get(prefix + "input-manifest.json") != sha(
        resource(root, prefix + "input-manifest.json")
    ):
        raise ValueError("Changed input manifest")
    for name, digest in wanted.items():
        if manifest.get(name.removeprefix(prefix)) != digest:
            raise ValueError("Source helper not bound to input manifest")
    records = layout["records"]
    validate_layout_set(layout, prefix)
    for record in records:
        name = prefix + record["path"]
        if (
            generated.get(name) != record["entry_sha256"]
            or sha(resource(root, name)) != record["entry_sha256"]
        ):
            raise ValueError("Changed modular source entry")
    for record in records:
        if record.get('kind')=='surface_rules':
            from .layout import reconstructed
            reconstructed({name:resource(root,prefix+name).read_text() for name in ('ear_surface.py','source_mesh_rules.py')},record)
        elif record.get('kind') == 'ordered_module':
            from .layout import reconstructed
            names = [record['entry_name']] + [item['path'] for item in record['stages']]
            reconstructed({name: resource(root, prefix + name).read_text() for name in names}, record)
    return outputs

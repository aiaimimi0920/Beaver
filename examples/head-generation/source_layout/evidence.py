"""Verify both native jobs without fabricating or rewriting their receipts."""

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


def native_job(path, root):
    raw = json.loads(Path(path).read_text())
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
    if set(wanted) != {prefix + name for name in HELPERS}:
        raise ValueError("Unexpected modular source set")
    gen, generated = native_job(generation_receipt, root)
    packed, outputs = native_job(packaging_receipt, root)
    if packed["jobId"] == gen["jobId"]:
        raise ValueError("Source packaging must be a separate native job")
    binding_path = prefix + "source_layout_binding.json"
    if set(outputs) != set(wanted) | {binding_path}:
        raise ValueError("Incomplete or unexpected source packaging outputs")
    for name, digest in outputs.items():
        if sha(resource(root, name)) != digest:
            raise ValueError("Changed native packaged source")
    if any(outputs[name] != digest for name, digest in wanted.items()):
        raise ValueError("Packaged source differs from generation plan")
    binding = json.loads(resource(root, binding_path).read_text())
    if binding != {
        "generation_job": gen["jobId"],
        "generation_receipt_sha256": sha(generation_receipt),
        "generation_outputs": generated,
        "helper_sha256": wanted,
    }:
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
    if len(records) != 2 or {x["path"] for x in records} != {
        "pipeline.py",
        "semantic_parameters.py",
    }:
        raise ValueError("Unexpected source layout entries")
    for record in records:
        name = prefix + record["path"]
        if (
            generated.get(name) != record["entry_sha256"]
            or sha(resource(root, name)) != record["entry_sha256"]
        ):
            raise ValueError("Changed modular source entry")
    return outputs

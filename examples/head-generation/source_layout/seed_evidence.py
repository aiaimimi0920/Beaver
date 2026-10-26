"""Bind a native authored-source seed to generation and source packaging."""
import copy
import hashlib
import json
from pathlib import Path
from .evidence import native_job, resource, sha
from .transport_sources import validate_native_frame


def verify_seed(binding, receipt, root):
    root = Path(root).resolve()
    snapshot = Path(receipt).read_bytes()
    if "response" not in json.loads(snapshot):
        raise ValueError("Source seed requires an original RPC receipt; journal copies are not authoritative")
    job, outputs = native_job(receipt, root, raw_bytes=snapshot)
    if outputs != {binding['path']:binding['sha256']}:
        raise ValueError('Unexpected native seed outputs')
    if job['process']['scriptSha256'] != binding['native_seed_script_sha256']:
        raise ValueError('Native seed script differs from frozen plan')
    if sha(resource(root,binding['path'])) != binding['sha256']:
        raise ValueError('Authored source seed content changed')
    complete = dict(binding, schema_version=1, job_id=job['jobId'], receipt_sha256=hashlib.sha256(snapshot).hexdigest())
    if 'job_id' in binding and complete != binding:
        raise ValueError('Seed receipt is not the bound native job')
    return complete


def finalize_seed_plan(plan, receipt, workspace):
    result = copy.deepcopy(plan)
    previous = result['source_seed_binding']
    if 'job_id' in previous:
        raise ValueError('Seed plan already finalized')
    binding = verify_seed(previous,receipt,workspace)
    marker = f'report["authored_source_seed"] = {previous!r}'
    code = result['generation']['code']
    if code.count(marker) != 1:
        raise ValueError('Missing exact planned seed report binding')
    result['generation']['code'] = code.replace(marker,f'report["authored_source_seed"] = {binding!r}',1)
    validate_native_frame(result['generation'])
    result['source_seed_binding'] = binding
    packing = result['source_packaging']
    packing['source_seed_binding'] = binding
    packing['source_seed_receipt_path'] = str(Path(receipt).resolve())
    packing['generation_script_sha256'] = hashlib.sha256(result['generation']['code'].encode()).hexdigest()
    return result

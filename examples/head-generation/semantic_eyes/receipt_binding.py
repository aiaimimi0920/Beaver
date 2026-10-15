"""Bind immutable geometry outputs to a caller-supplied trusted Beaver poll receipt."""
import hashlib
import json
from pathlib import Path


def bind_generation_receipt(path, workspace, prefix, current_hashes, allow_workspace_copy=False):
    raw = Path(path).read_bytes()
    receipt = json.loads(raw)['response']['result']['result']
    result = receipt['result']
    if receipt['status'] != 'succeeded' or result['success'] is not True or result['exitCode'] != 0:
        raise ValueError('Generation receipt must record a successful native job')
    source_workspace = Path(receipt['process']['workspace']).resolve()
    bake_workspace = Path(workspace).resolve()
    copied = source_workspace != bake_workspace
    if copied and not allow_workspace_copy:
        raise ValueError('Generation receipt workspace mismatch; explicit verified-copy mode required')
    outputs = result['outputs']
    hashes = {entry['path']: entry['sha256'] for entry in outputs}
    if len(hashes) != len(outputs):
        raise ValueError('Duplicate receipt output paths')
    bound = {}
    for name in ('aster_head_runtime_morph.blend', 'aster_head.glb', 'semantic_parameters.json'):
        key = prefix + name
        if hashes.get(key) != current_hashes[key]:
            raise ValueError('Same-job generation receipt output mismatch: ' + key)
        bound[key] = hashes[key]
    return {'receipt_sha256': hashlib.sha256(raw).hexdigest(), 'job_id': receipt['jobId'],
            'script_sha256': receipt['process']['scriptSha256'], 'immutable_outputs': bound,
            'source_generation_workspace': str(source_workspace),
            'current_bake_workspace': str(bake_workspace),
            'workspace_copy': copied,
            'copy_verification': 'all three immutable output hashes match original successful job' if copied else 'same workspace',
            'trust': 'caller-provided native Beaver receipt; not a cryptographic signature'}

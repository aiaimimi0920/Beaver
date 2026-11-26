"""Bind immutable geometry outputs to a caller-supplied trusted Beaver poll receipt."""
import hashlib
import json
from pathlib import Path


def bind_generation_receipt(path, workspace, prefix, current_hashes, allow_workspace_copy=False):
    raw = Path(path).read_bytes()
    parsed = json.loads(raw)
    if 'response' in parsed:
        receipt = parsed['response']['result']['result']
    else:
        receipt = parsed
        for key in ('runId', 'requestId'):
            value = receipt[key]
            if not isinstance(value, str) or not value or value in ('.', '..') or '/' in value or '\\' in value:
                raise ValueError('Durable native job identifiers must be single components')
        root = Path(workspace).resolve()
        relative = Path('.beaver-context/external') / receipt['runId'] / receipt['requestId'] / 'job.json'
        journal = root / relative
        if ('..' in relative.parts or not journal.is_relative_to(root)
            or Path(path).absolute() != journal
            or any(p.is_symlink() for p in [journal, *journal.parents] if p != root and p.is_relative_to(root))
            or Path(receipt['process']['jobDirectory']).resolve() != journal.parent):
            raise ValueError('Durable native job must retain its original location')
        marker = journal.with_name('completed.json')
        if (marker.is_symlink() or json.loads(marker.read_text()).get('completed') is not True
            or receipt['result'].get('ownedTreeCleanup') != 'completed'
            or receipt['result'].get('termination') != 'exited'):
            raise ValueError('Incomplete durable native job')
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

"""Bind a native authored-source seed to generation and source packaging."""
import copy
import ast
import hashlib
import json
from pathlib import Path
from .evidence import native_job, resource, sha
from .transport_sources import validate_native_frame


def verify_retained_source(binding, root, retained_path):
    """Match the generated recipe to its verified seed, with an optional pool."""
    root = Path(root).resolve()
    seeded = resource(root,binding['path']).read_bytes()
    retained = resource(root,retained_path).read_bytes()
    if hashlib.sha256(seeded).hexdigest() != binding['sha256']:
        raise ValueError('Authored source seed content changed')
    if binding.get('schema_version', 1) in (2, 3):
        runtime = resource(root, binding['runtime_path']).read_bytes()
        if hashlib.sha256(runtime).hexdigest() != binding['runtime_sha256']:
            raise ValueError('Runtime source seed content changed')
        if seeded != retained:
            raise ValueError('Generation retained bundle differs from native seed')
        compile(retained.decode('utf-8'), '<retained source>', 'exec')
        compile(runtime.decode('utf-8'), '<runtime source>', 'exec')
        return
    if seeded == retained:
        return
    if not retained or not seeded.startswith(retained):
        raise ValueError('Generation retained bundle differs from native seed')
    suffix = seeded[len(retained):]
    if not suffix.startswith(b'\n\nNATIVE_RUNTIME_SOURCE = '):
        raise ValueError('Invalid appended runtime source framing')
    nodes = ast.parse(suffix.decode('utf-8')).body
    if (len(nodes)!=1 or not isinstance(nodes[0],ast.Assign) or len(nodes[0].targets)!=1
            or not isinstance(nodes[0].targets[0],ast.Name)
            or nodes[0].targets[0].id!='NATIVE_RUNTIME_SOURCE'
            or not isinstance(nodes[0].value,ast.Constant)
            or not isinstance(nodes[0].value.value,str)):
        raise ValueError('Invalid appended runtime source schema')
    compile(retained.decode('utf-8'),'<retained source>','exec')
    compile(ast.literal_eval(nodes[0].value),'<runtime source>','exec')


def verify_seed(binding, receipt, root):
    root = Path(root).resolve()
    if binding.get('schema_version') == 3:
        return verify_two_seed_jobs(binding, receipt, root)
    snapshot = Path(receipt).read_bytes()
    if "response" not in json.loads(snapshot):
        raise ValueError("Source seed requires an original RPC receipt; journal copies are not authoritative")
    job, outputs = native_job(receipt, root, raw_bytes=snapshot)
    version = binding.get('schema_version', 1)
    if version not in (1, 2):
        raise ValueError('Unknown source seed binding schema')
    expected = {binding['path']: binding['sha256']}
    if version == 2:
        if binding['runtime_path'] == binding['path']:
            raise ValueError('Duplicate native seed paths')
        expected[binding['runtime_path']] = binding['runtime_sha256']
    if outputs != expected:
        raise ValueError('Unexpected native seed outputs')
    if job['process']['scriptSha256'] != binding['native_seed_script_sha256']:
        raise ValueError('Native seed script differs from frozen plan')
    for path, digest in expected.items():
        if sha(resource(root, path)) != digest:
            raise ValueError('Authored source seed content changed')
    complete = dict(binding, schema_version=version, job_id=job['jobId'], receipt_sha256=hashlib.sha256(snapshot).hexdigest())
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
    if binding.get('schema_version') == 3:
        packing['source_seed_receipt_paths'] = [str(Path(path).resolve()) for path in receipt]
    else:
        packing['source_seed_receipt_path'] = str(Path(receipt).resolve())
    packing['generation_script_sha256'] = hashlib.sha256(result['generation']['code'].encode()).hexdigest()
    return result


def verify_two_seed_jobs(binding, receipts, root):
    """Verify two untouched native receipts; never synthesize a combined receipt."""
    if not isinstance(receipts, (list, tuple)) or len(receipts) != 2:
        raise ValueError('Two ordered original native seed receipts required')
    if binding['path'] == binding['runtime_path']:
        raise ValueError('Two seed files must have distinct paths')
    first = {'path': binding['path'], 'sha256': binding['sha256'],
             'native_seed_script_sha256': binding['native_seed_script_sha256']}
    second = {'path': binding['runtime_path'], 'sha256': binding['runtime_sha256'],
              'native_seed_script_sha256': binding['runtime_seed_script_sha256']}
    a = verify_seed(first, receipts[0], root)
    b = verify_seed(second, receipts[1], root)
    if a['job_id'] == b['job_id'] or a['receipt_sha256'] == b['receipt_sha256']:
        raise ValueError('Distinct native source seed jobs required')
    complete = dict(binding, job_id=a['job_id'], receipt_sha256=a['receipt_sha256'],
                    runtime_job_id=b['job_id'], runtime_receipt_sha256=b['receipt_sha256'])
    if 'job_id' in binding and complete != binding:
        raise ValueError('Seed receipts are not the two bound native jobs')
    return complete

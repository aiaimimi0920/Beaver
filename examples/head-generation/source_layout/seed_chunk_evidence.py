"""Schema4 requires EVERY ordered, distinct, untouched native RPC receipt."""
import hashlib
import json
from pathlib import Path
from .evidence import native_job, resource
from .seed_chunks import assembly_code


def validate_chunk_manifest(binding):
    modules, jobs = binding['modules'], binding['jobs']
    expected_paths = [binding['path']]
    expected_hashes = [binding['sha256']]
    if 'runtime_path' in binding:
        expected_paths.append(binding['runtime_path'])
        expected_hashes.append(binding['runtime_sha256'])
    if ([m['path'] for m in modules] != expected_paths or
            [m['sha256'] for m in modules] != expected_hashes or
            len(set(expected_paths)) != len(expected_paths)):
        raise ValueError('Seed module order/path/hash mismatch')
    cursor, paths = 0, set()
    for number, module in enumerate(modules):
        if not module['chunks']:
            raise ValueError('Missing seed chunks')
        total = 0
        for index, chunk in enumerate(module['chunks']):
            if chunk['index'] != index or chunk['path'] != module['path'] + f'.part{index:04d}.txt':
                raise ValueError('Seed chunk order/path mismatch')
            record = jobs[cursor]
            expected = dict(chunk, kind='chunk', module_index=number, job_index=cursor,
                            native_seed_script_sha256=record['native_seed_script_sha256'])
            if record != expected or chunk['path'] in paths or chunk['byte_length'] < 0:
                raise ValueError('Invalid/duplicate chunk job')
            paths.add(chunk['path']);total += chunk['byte_length'];cursor += 1
        expected = {'kind': 'assembly', 'module_index': number, 'job_index': cursor,
                    'path': module['path'], 'sha256': module['sha256'], 'byte_length': module['byte_length'],
                    'native_seed_script_sha256': hashlib.sha256(assembly_code(module).encode()).hexdigest()}
        if (module['assembly_job_index'] != cursor or jobs[cursor] != expected or
                total != module['byte_length'] or module['path'] in paths):
            raise ValueError('Invalid seed assembly job/order/size')
        paths.add(module['path']);cursor += 1
    if cursor != len(jobs) or binding['native_seed_script_sha256'] != jobs[0]['native_seed_script_sha256']:
        raise ValueError('Missing/extra seed jobs or initial script mismatch')
    return jobs


def verify_chunk_seed(binding, receipts, root):
    try:
        jobs = validate_chunk_manifest(binding)
    except (KeyError, IndexError, TypeError) as error:
        raise ValueError('Incomplete schema4 manifest') from error
    if not isinstance(receipts, (list, tuple)) or len(receipts) != len(jobs):
        raise ValueError('All ordered original native chunk AND assembly receipts required')
    identities, hashes, evidence = set(), set(), []
    for planned, receipt in zip(jobs, receipts):
        snapshot = Path(receipt).read_bytes()
        if 'response' not in json.loads(snapshot):
            raise ValueError('Original native RPC receipt required for every seed job')
        job, outputs = native_job(receipt, root, raw_bytes=snapshot)
        if outputs != {planned['path']: planned['sha256']} or job['process']['scriptSha256'] != planned['native_seed_script_sha256']:
            raise ValueError('Seed receipt output/script/order mismatch')
        digest = hashlib.sha256(snapshot).hexdigest()
        if job['jobId'] in identities or digest in hashes:
            raise ValueError('Distinct native seed receipts and jobs required')
        identities.add(job['jobId']);hashes.add(digest)
        data = resource(root, planned['path']).read_bytes()
        if len(data) != planned['byte_length'] or hashlib.sha256(data).hexdigest() != planned['sha256']:
            raise ValueError('Seed chunk/module content or size changed')
        data.decode('utf-8')
        evidence.append({'job_id': job['jobId'], 'receipt_sha256': digest})
    for module in binding['modules']:
        assembled = b''.join(resource(root, c['path']).read_bytes() for c in module['chunks'])
        if assembled != resource(root, module['path']).read_bytes():
            raise ValueError('Seed assembly differs from ordered chunks')
        compile(assembled.decode('utf-8'), '<verified authored source>', 'exec')
    complete = dict(binding, job_id=evidence[0]['job_id'], receipt_sha256=evidence[0]['receipt_sha256'],
                    job_receipts=evidence)
    if 'job_id' in binding and complete != binding:
        raise ValueError('Seed receipts differ from frozen ordered job identities')
    return complete


def seed_receipt_sidecars(binding):
    """Each sidecar carries exact original receipt bytes; NEVER combined receipts."""
    if binding.get('schema_version') == 4:
        return [(f'source_seed_receipt_{i:04d}.json', row['receipt_sha256'])
                for i, row in enumerate(binding['job_receipts'])]
    rows = [('source_seed_receipt.json', binding['receipt_sha256'])]
    if binding.get('schema_version') == 3:
        rows.append(('source_seed_runtime_receipt.json', binding['runtime_receipt_sha256']))
    return rows


def seed_job_ids(binding):
    if binding.get('schema_version') == 4:
        return {row['job_id'] for row in binding['job_receipts']}
    jobs = {binding['job_id']}
    if binding.get('schema_version') == 3:
        jobs.add(binding['runtime_job_id'])
    return jobs

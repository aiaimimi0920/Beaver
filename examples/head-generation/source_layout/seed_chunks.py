"""Bounded ordered plain-UTF8 seed chunks and exact module assembly (schema4)."""
import hashlib
from .transport_sources import validate_native_frame


def assembly_code(module):
    """The manifest itself is frozen into the independently receipted assembly job."""
    return ('import hashlib\nfrom pathlib import Path\n'
            f'_chunks = {module["chunks"]!r}\n'
            '_parts = []\n_seen = set()\n'
            'for _index, _chunk in enumerate(_chunks):\n'
            '    if _chunk["index"] != _index or _chunk["path"] in _seen:\n'
            '        raise ValueError("Seed chunk order or duplicate path")\n'
            '    _seen.add(_chunk["path"])\n'
            '    _raw = Path(beaver_input(_chunk["path"])).read_bytes()\n'
            '    _raw.decode("utf-8")\n'
            '    if len(_raw) != _chunk["byte_length"] or hashlib.sha256(_raw).hexdigest() != _chunk["sha256"]:\n'
            '        raise ValueError("Seed chunk size/hash changed")\n'
            '    _parts.append(_raw)\n'
            '_module = b"".join(_parts)\n'
            f'if len(_module) != {module["byte_length"]!r} or hashlib.sha256(_module).hexdigest() != {module["sha256"]!r}:\n'
            '    raise ValueError("Assembled source size/hash changed")\n'
            'compile(_module.decode("utf-8"), "<assembled authored source>", "exec")\n'
            f'Path(beaver_output({module["path"]!r})).write_bytes(_module)\n')


def chunk_seed_jobs(modules, readable_literal):
    """modules is ordered [(destination.py, uncompressed_source_text), ...]."""
    requests, records, manifests = [], [], []
    for module_index, (destination, text) in enumerate(modules):
        raw = text.encode('utf-8')
        module = {'path': destination, 'sha256': hashlib.sha256(raw).hexdigest(),
                  'byte_length': len(raw), 'chunks': []}
        offset = 0
        while offset < len(text) or not module['chunks']:
            index = len(module['chunks'])
            path = destination + f'.part{index:04d}.txt'
            def request_for(count):
                part = text[offset:offset + count]
                code = ('from pathlib import Path\n'
                        f'Path(beaver_output({path!r})).write_text({readable_literal(part)},encoding="utf-8",newline="")\n')
                return {'code': code, 'outputs': [{'path': path, 'expectedSha256': None}],
                        'timeoutSeconds': 120}
            # Cap ordinary text length, then test the COMPLETE native frame.
            count = min(262144, len(text) - offset)
            while True:
                request = request_for(count)
                try:
                    validate_native_frame(request)
                    break
                except ValueError:
                    if count <= 1:
                        raise ValueError('Even one UTF8 character cannot fit seed frame')
                    count //= 2
            data = text[offset:offset + count].encode('utf-8')
            chunk = {'index': index, 'path': path, 'byte_length': len(data),
                     'sha256': hashlib.sha256(data).hexdigest()}
            module['chunks'].append(chunk)
            records.append(dict(chunk, kind='chunk', module_index=module_index,
                                job_index=len(records), native_seed_script_sha256=
                                hashlib.sha256(request['code'].encode()).hexdigest()))
            requests.append(request)
            offset += count
            if offset == len(text):
                break
        request = {'code': assembly_code(module),
                   'outputs': [{'path': destination, 'expectedSha256': None}], 'timeoutSeconds': 120}
        validate_native_frame(request)
        module['assembly_job_index'] = len(records)
        records.append({'kind': 'assembly', 'module_index': module_index, 'job_index': len(records),
                        'path': destination, 'sha256': module['sha256'], 'byte_length': len(raw),
                        'native_seed_script_sha256': hashlib.sha256(request['code'].encode()).hexdigest()})
        requests.append(request)
        manifests.append(module)
    return requests, {'schema_version': 4, 'modules': manifests, 'jobs': records}

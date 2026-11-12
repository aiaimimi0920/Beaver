"""Bounded native source-file batches and disjoint original-receipt verification."""
from .transport_sources import validate_native_frame


def source_packaging_batches(prefix, payload):
    """Every job independently rechecks the same frozen generation outputs."""
    requests = []
    code, outputs = prefix, []
    for path, content in payload.items():
        method = "write_bytes" if isinstance(content, bytes) else "write_text"
        options = "" if isinstance(content, bytes) else ", encoding='utf-8', newline='\\n'"
        line = f"Path(beaver_output({path!r})).{method}({content!r}{options})\n"
        output = {"path": path, "expectedSha256": None}
        candidate = {"code": code+line, "outputs": outputs+[output], "timeoutSeconds": 120}
        try:
            if len(candidate['outputs']) > 64 or len(candidate['code'].encode()) > 1024*1024:
                raise ValueError('Native packaging size limit')
            validate_native_frame(candidate)
        except ValueError:
            if not outputs:
                raise ValueError('One source output exceeds the native packaging limit')
            requests.append({"code": code, "outputs": outputs, "timeoutSeconds": 120})
            code, outputs = prefix, []
        code += line
        outputs.append(output)
        validate_native_frame({"code": code, "outputs": outputs, "timeoutSeconds": 120})
    if outputs:
        requests.append({"code": code, "outputs": outputs, "timeoutSeconds": 120})
    if not requests:
        raise ValueError('Empty source packaging payload')
    return requests


def original_packaging_jobs(receipts, root, generation_id):
    from .evidence import native_job
    paths = receipts if isinstance(receipts, (list, tuple)) else [receipts]
    if not paths:
        raise ValueError('Missing original source packaging receipts')
    jobs, outputs = set(), {}
    for path in paths:
        job, current = native_job(path, root)
        if job['jobId'] == generation_id or job['jobId'] in jobs:
            raise ValueError('Source packaging requires distinct native jobs')
        if not current or set(outputs) & set(current):
            raise ValueError('Empty or overlapping source packaging outputs')
        jobs.add(job['jobId'])
        outputs.update(current)
    return jobs, outputs

"""Package a hash-bound read-only native UV diagnostic; never execute Blender."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from receipt_binding import bind_generation_receipt

HERE = Path(__file__).resolve().parent


def build_request(prefix, workspace, receipt):
    if not re.fullmatch(r'assets/aster/general_face_eyes_[a-z0-9_]+/', prefix):
        raise ValueError('Expected semantic eye output prefix')
    root = Path(workspace)
    required = [prefix + name for name in ('aster_head_runtime_morph.blend', 'aster_head.glb', 'semantic_parameters.json')]
    hashes = {name: hashlib.sha256((root/name).read_bytes()).hexdigest() for name in required}
    binding = bind_generation_receipt(receipt, root, prefix, hashes)
    source = (HERE/'uv_diagnostic.py').read_text()
    code = 'import hashlib, json\nfrom pathlib import Path\n'
    code += f'for _path, _sha in {hashes!r}.items():\n'
    code += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "Diagnostic input changed"\n'
    code += f'exec(compile({source!r}, "uv_diagnostic.py", "exec"))\n'
    code += f'_report = diagnose_blend(beaver_input({prefix + "aster_head_runtime_morph.blend"!r}))\n'
    code += f'_report["generation_binding"] = {binding!r}\n'
    code += f'Path(beaver_output({prefix + "uv_diagnostic.json"!r})).write_text(json.dumps(_report, indent=2))\n'
    code += f'Path(beaver_output({prefix + "uv_diagnostic.py"!r})).write_text({source!r})\n'
    outputs = []
    for name in ('uv_diagnostic.json', 'uv_diagnostic.py'):
        path = prefix + name
        old = root/path
        outputs.append({'path': path, 'expectedSha256': hashlib.sha256(old.read_bytes()).hexdigest() if old.is_file() else None})
    compile(code, '<native UV diagnostic>', 'exec')
    return {'code': code, 'outputs': outputs, 'timeoutSeconds': 240}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--generation-receipt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build_request(args.prefix, args.workspace, args.generation_receipt)))

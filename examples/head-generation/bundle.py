"""Package a reviewed recipe for Beaver blender.start; never run Blender here."""
import argparse
import hashlib
import json
from pathlib import Path

MODULES = ['face', 'eye_socket', 'mouth', 'ears', 'shell', 'textures', 'glb_merge']
SOURCE_FILES = [name + '.py' for name in MODULES] + [
    'pipeline.py', 'authoring-brief.md', 'planner-mapping.md',
    'eye-structure-plan.md', 'face_style.json',
]
ASSET_FILES = [
    'aster_head_editable.blend', 'aster_head_runtime_morph.blend',
    'aster_head.glb', 'aster_definition.tres', 'generation_report.json',
    'face_base.png', 'hair_base.png', 'face_ilm.png', 'face_map.png', 'hair_ilm.png',
]
OUTPUT_PREFIX = 'assets/aster/head_recovery_81/'


def build_request(source_dir=None):
    root = Path(source_dir) if source_dir else Path(__file__).parent
    style = json.loads((root / 'face_style.json').read_text())
    code = 'import types, sys\n'
    for name in MODULES:
        source = (root / (name + '.py')).read_text()
        code += f'module = types.ModuleType({name!r}); sys.modules[{name!r}] = module\n'
        code += 'module.beaver_output = beaver_output\n'
        code += f'module.STYLE = {style!r}\n'
        code += f'exec(compile({source!r}, {name!r}, "exec"), module.__dict__)\n'
    code += (root / 'pipeline.py').read_text()
    for name in SOURCE_FILES:
        code += f'Path(beaver_output({(OUTPUT_PREFIX + name)!r})).write_text({(root / name).read_text()!r})\n'
    manifest = {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in SOURCE_FILES
    }
    code += f'Path(beaver_output({(OUTPUT_PREFIX + "input-manifest.json")!r})).write_text({json.dumps(manifest, indent=2)!r})\n'
    return {
        'code': code,
        'outputs': [
            {'path': OUTPUT_PREFIX + name, 'expectedSha256': None}
            for name in ASSET_FILES + SOURCE_FILES + ['input-manifest.json']
        ],
        'timeoutSeconds': 240,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    request = build_request()
    compile(request['code'], '<Beaver recipe>', 'exec')
    args.output.write_text(json.dumps(request))
    print(f'Wrote {len(request["outputs"])} declared outputs to {args.output}')


if __name__ == '__main__':
    main()

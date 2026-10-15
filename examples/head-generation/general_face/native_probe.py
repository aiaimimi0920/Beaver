"""CPU-only manifest binding for the separately executed Beaver native GUT probe."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from controls import validate
from definition_guard import require_head_only_definition
from glb_affine import read_glb, read_rows, accessor_values, require

SCHEMA = "native-node-affine-probe-v1"
CONTRACT = "affine-of-actual-imported-same-job-baseline"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def values(doc, binary, index):
    accessor = doc['accessors'][index]
    kind = accessor['type']
    if kind in ('VEC2', 'VEC3', 'VEC4'):
        return accessor_values(doc, binary, index, int(kind[-1]))
    require(kind == 'SCALAR' and accessor['componentType'] in (5121, 5123, 5125)
            and not accessor.get('normalized') and not accessor.get('sparse')
            and not accessor.get('extensions'), 'Unsupported index accessor')
    return read_rows(doc, binary, accessor['bufferView'], accessor.get('byteOffset', 0),
                     accessor['count'], {5121: '<B', 5123: '<H', 5125: '<I'}[accessor['componentType']])


def streams(raw):
    doc, binary = read_glb(raw)
    meshes = [m for m in doc['meshes'] if m.get('name') == 'Face']
    require(len(meshes) == 1, 'Expected unique Face mesh')
    mesh = meshes[0]
    require(not mesh.get('extensions'), 'Unsupported Face extension')
    result = []
    for p in mesh['primitives']:
        require(not p.get('extensions') and p.get('mode', 4) == 4, 'Unsupported primitive')
        result.append((
            {k: values(doc, binary, v) for k, v in p['attributes'].items()},
            [{k: values(doc, binary, v) for k, v in t.items()} for t in p.get('targets', [])],
            values(doc, binary, p['indices']) if 'indices' in p else None))
    return mesh.get('extras', {}).get('targetNames'), result


def build_case(name, local, resource):
    require(bool(re.fullmatch(r'[A-Za-z0-9_]+', name)), 'Unsafe case name')
    resource = resource.rstrip('/')
    require(bool(re.fullmatch(r'res://[A-Za-z0-9_/-]+', resource))
            and all(part not in ('', '.', '..') for part in resource[6:].split('/')),
            'Expected safe res:// resource directory')
    local = Path(local)
    paths = {key: local / filename for key, filename in (
        ('base', 'general_base_export.glb'), ('variant', 'aster_head.glb'),
        ('definition', 'aster_definition.tres'), ('intent', 'general_intent.json'))}
    require(all(p.is_file() for p in paths.values()), 'Missing generated probe input')
    require_head_only_definition(paths['definition'].read_text())
    require(streams(paths['base'].read_bytes()) == streams(paths['variant'].read_bytes()),
            'Local Face joint streams, morphs or indices/order differ')
    intent = json.loads(paths['intent'].read_text())
    proportions = validate(intent['proportions'])
    result = {'name': name, 'declared_scale': [proportions[k] for k in ('width', 'height', 'depth')],
              'declared_pivot': [0., 1.641, 0.], 'raw_face_streams_and_indices_exact': True}
    for key, path in paths.items():
        result[key + '_resource'] = resource + '/' + path.name
        result[key + '_sha256'] = sha(path)
    return result


def build_manifest(cases):
    require(bool(cases), 'At least one probe case is required')
    result = [build_case(*case) for case in cases]
    require(len({c['name'] for c in result}) == len(result), 'Duplicate case names')
    return {'schema': SCHEMA, 'contract': CONTRACT, 'weights': [0., .25, .5, .75, 1.],
            'normal_atol': 3e-6, 'position_atol': 2e-7, 'rtol': 2e-6,
            'generator_sha256': sha(__file__), 'cases': result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--case', nargs=3, action='append', required=True,
                        metavar=('NAME', 'LOCAL_DIR', 'RESOURCE_DIR'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build_manifest(args.case), indent=2) + '\n')


if __name__ == '__main__':
    main()

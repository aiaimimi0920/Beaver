"""CPU-only manifest binding for the separately executed Beaver native GUT probe."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from controls import validate, vector
from definition_guard import require_head_only_definition
from glb_affine import read_glb, read_rows, require

SCHEMA = "native-node-affine-probe-v1"
CONTRACT = "affine-of-actual-imported-same-job-baseline"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def values(doc, binary, index):
    """Canonical raw component bytes plus semantics; ignore only storage layout.

    Normalized integers must retain both their bits and normalization flag.
    Float bytes also retain signed zero, with no rounding or tolerance comparison.
    Sparse implicit entries expand to zero bytes before exact comparison.
    """
    require(isinstance(index, int) and 0 <= index < len(doc['accessors']), 'Invalid accessor')
    accessor = doc['accessors'][index]
    kind, component = accessor['type'], accessor['componentType']
    widths = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}
    sizes = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
    require(kind in widths and component in sizes and not accessor.get('extensions'),
            'Unsupported accessor format')
    normalized = accessor.get('normalized', False)
    require(isinstance(normalized, bool) and (not normalized or component in (5120, 5121, 5122, 5123)),
            'Invalid accessor normalization')
    count, size = accessor['count'], widths[kind] * sizes[component]
    require(isinstance(count, int) and count > 0, 'Invalid accessor count')

    def raw_rows(view, offset, length, packed=False):
        require((doc['bufferViews'][view].get('byteOffset', 0) + offset) % sizes[component] == 0,
                'Misaligned component bytes')
        return [bytes(r) for r in read_rows(doc, binary, view, offset, length,
                                          '<' + 'B' * size, packed=packed)]

    rows = (raw_rows(accessor['bufferView'], accessor.get('byteOffset', 0), count)
            if 'bufferView' in accessor else [bytes(size)] * count)
    if 'sparse' in accessor:
        sparse = accessor['sparse']
        require(not sparse.get('extensions') and 0 < sparse['count'] <= count, 'Invalid sparse accessor')
        indices, replacements = sparse['indices'], sparse['values']
        require(indices['componentType'] in (5121, 5123, 5125), 'Invalid sparse index type')
        ids = [r[0] for r in read_rows(doc, binary, indices['bufferView'], indices.get('byteOffset', 0),
                                     sparse['count'], {5121: '<B', 5123: '<H', 5125: '<I'}[indices['componentType']],
                                     packed=True)]
        require(ids == sorted(set(ids)) and ids[-1] < count, 'Invalid sparse indices')
        for idx, row in zip(ids, raw_rows(replacements['bufferView'], replacements.get('byteOffset', 0),
                                         sparse['count'], packed=True)):
            rows[idx] = row
    return kind, component, normalized, rows


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
        ('definition', 'aster_definition.tres'), ('intent', 'general_intent.json'),
        ('report', 'generation_report.json'))}
    require(all(p.is_file() for p in paths.values()), 'Missing generated probe input')
    require_head_only_definition(paths['definition'].read_text())
    require(streams(paths['base'].read_bytes()) == streams(paths['variant'].read_bytes()),
            'Local Face joint streams, morphs or indices/order differ')
    intent = json.loads(paths['intent'].read_text())
    proportions = validate(intent['proportions'])
    report = json.loads(paths['report'].read_text())
    pivot = vector(report['general_face']['pivot_blender_xyz'])
    result = {'name': name, 'declared_scale': [proportions[k] for k in ('width', 'height', 'depth')],
              'declared_pivot': [pivot[0], pivot[2], -pivot[1]], 'raw_face_streams_and_indices_exact': True}
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

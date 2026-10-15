"""Build the two-case native eye import manifest; never execute Godot."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys
import numpy as np

sys.path.append(str(Path(__file__).resolve().parents[1] / 'general_face'))
from glb_affine import accessor_values, read_glb

WEIGHTS = [0, .25, .5, .75, 1]
SCOPE = 'Head-only NPRCharacter import and MouthOpen; no performance/FaceRig or visual acceptance'


def resource_file(workspace, relative):
    path = PurePosixPath(relative)
    if path.is_absolute() or '..' in path.parts or '\\' in relative:
        raise ValueError('Expected a contained project-relative resource path')
    root = Path(workspace).resolve()
    target = (root / path).resolve()
    if not target.is_relative_to(root) or not target.is_file():
        raise ValueError('Missing or escaped resource: ' + relative)
    return target


def case_bounds(raw):
    doc, binary = read_glb(raw)
    nodes = [node for node in doc['nodes'] if node.get('name') == 'Face']
    if len(nodes) != 1:
        raise ValueError('Expected one Face node')
    node = nodes[0]
    if ('matrix' in node or node.get('translation', [0, 0, 0]) != [0, 0, 0]
            or node.get('scale', [1, 1, 1]) != [1, 1, 1]
            or node.get('rotation', [0, 0, 0, 1]) != [0, 0, 0, 1]):
        raise ValueError('Expected identity local Face transform')
    primitives = doc['meshes'][node['mesh']]['primitives']
    if len(primitives) != 1 or len(primitives[0].get('targets', [])) != 1:
        raise ValueError('Expected one Face primitive and one morph')
    primitive = primitives[0]
    positions = np.array(accessor_values(doc, binary, primitive['attributes']['POSITION'], 3))
    deltas = np.array(accessor_values(doc, binary, primitive['targets'][0]['POSITION'], 3))
    if positions.shape != deltas.shape:
        raise ValueError('Mismatched morph vertex count')
    index = doc['accessors'][primitive['indices']]
    if index['type'] != 'SCALAR' or index['componentType'] not in (5121, 5123, 5125) or index['count'] <= 0:
        raise ValueError('Expected nonempty triangle indices')
    return {'vertex_count': len(positions), 'index_count': index['count'],
            'bounds': [{'weight': weight, 'min': (positions + weight * deltas).min(axis=0).tolist(),
                        'max': (positions + weight * deltas).max(axis=0).tolist()} for weight in WEIGHTS]}


def build_manifest(workspace, base_prefix, target_prefix, original):
    result = {'scope': SCOPE, 'position_atol_m': 3e-6, 'weights': WEIGHTS,
              'files': [], 'cases': [], 'original_resource': 'res://' + original}
    def bind(relative):
        path = resource_file(workspace, relative)
        result['files'].append({'path': 'res://' + relative,
                                'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
        return path
    bind(original)
    for name, prefix in [('base', base_prefix.rstrip('/')), ('target', target_prefix.rstrip('/'))]:
        files = {filename: bind(prefix + '/' + filename) for filename in (
            'aster_head.glb', 'aster_definition.tres', 'generation_report.json', 'semantic_parameters.json')}
        result['cases'].append({'name': name, 'definition': 'res://' + prefix + '/aster_definition.tres',
                               'model': 'res://' + prefix + '/aster_head.glb',
                               **case_bounds(files['aster_head.glb'].read_bytes())})
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--base-prefix', required=True)
    parser.add_argument('--target-prefix', required=True)
    parser.add_argument('--original', default='assets/aster/head_recovery_49/aster_head.glb')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(build_manifest(args.workspace, args.base_prefix,
                                                   args.target_prefix, args.original), indent=2) + '\n')

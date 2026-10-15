"""CPU-only retained manifest fixtures, with no native engine claim."""
import copy
import json
from pathlib import Path
import subprocess
import struct
import sys
import tempfile
import unittest
from native_probe import build_manifest, case_bounds, resource_file, WEIGHTS
from glb_affine import read_glb, write_glb
from test_glb_affine import fixture as float_fixture


def fixture():
    doc, binary = read_glb(float_fixture())
    view = len(doc["bufferViews"])
    doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": 6})
    accessor = len(doc["accessors"])
    doc["accessors"].append({"bufferView": view, "componentType": 5123, "type": "SCALAR", "count": 3})
    doc["meshes"][0]["primitives"][0]["indices"] = accessor
    return write_glb(doc, binary + struct.pack("<HHH", 0, 1, 2))


class NativeProbeTests(unittest.TestCase):
    def test_bounds_and_rejection(self):
        raw = fixture()
        result = case_bounds(raw)
        self.assertEqual([b['weight'] for b in result['bounds']], WEIGHTS)
        self.assertGreater(result['vertex_count'], 0)
        doc, binary = read_glb(raw)
        for mutation in ('transform', 'morph', 'node'):
            changed = copy.deepcopy(doc)
            if mutation == 'transform':
                changed['nodes'][0]['scale'] = [2, 1, 1]
            elif mutation == 'morph':
                changed['meshes'][0]['primitives'][0]['targets'] = []
            else:
                changed['nodes'].append(copy.deepcopy(changed['nodes'][0]))
            with self.assertRaises(ValueError):
                case_bounds(write_glb(changed, binary))

    def test_cli_hash_binding_and_containment(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'original.glb').write_bytes(fixture())
            for name in ('base', 'target'):
                path = root / name
                path.mkdir()
                (path / 'aster_head.glb').write_bytes(fixture())
                for filename in ('aster_definition.tres', 'generation_report.json', 'semantic_parameters.json'):
                    (path / filename).write_text('CPU fixture')
            expected = build_manifest(root, 'base', 'target', 'original.glb')
            self.assertEqual(len(expected['files']), 9)
            self.assertEqual(expected['position_atol_m'], 3e-6)
            output = root / 'manifest.json'
            subprocess.run([sys.executable, str(Path(__file__).with_name('native_probe.py')),
                            '--workspace', str(root), '--base-prefix', 'base', '--target-prefix', 'target',
                            '--original', 'original.glb', '--output', str(output)], check=True)
            self.assertEqual(json.loads(output.read_text()), expected)
            (root / 'base/generation_report.json').write_text('changed')
            self.assertNotEqual(build_manifest(root, 'base', 'target', 'original.glb')['files'], expected['files'])
            for path in ('../outside', '/absolute', 'missing'):
                with self.assertRaises(ValueError):
                    resource_file(root, path)


if __name__ == '__main__':
    unittest.main()

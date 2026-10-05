"""Focused packaging and morph-preserving merge tests; no Blender dependency."""
import copy
import json
import struct
import unittest
from pathlib import Path
from bundle import build_request, OUTPUT_PREFIX, SOURCE_FILES
from glb_merge import merge_face, read_glb


def glb(document, data):
    text = json.dumps(document).encode()
    text += b' ' * (-len(text) % 4)
    data += b'\0' * (-len(data) % 4)
    return (struct.pack('<III', 1179937895, 2, 28 + len(text) + len(data))
            + struct.pack('<II', len(text), 1313821514) + text
            + struct.pack('<II', len(data), 5130562) + data)


def fixture():
    base = {
        'asset': {'version': '2.0'}, 'buffers': [{'byteLength': 12}],
        'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 12}],
        'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 1, 'type': 'VEC3'}],
        'meshes': [{'name': name, 'primitives': [{'attributes': {'POSITION': 0}, 'material': 0}]}
                   for name in ['Body', 'Face', 'Hair']],
        'nodes': [{'name': name, 'mesh': i} for i, name in enumerate(['Body', 'Face', 'Hair'])],
        'materials': [{'name': 'old'}], 'images': [{'bufferView': 0}],
        'samplers': [{}], 'textures': [{'source': 0, 'sampler': 0}],
    }
    new = {
        'asset': {'version': '2.0'}, 'buffers': [{'byteLength': 24}],
        'bufferViews': [{'buffer': 0, 'byteOffset': i * 12, 'byteLength': 12} for i in range(2)],
        'accessors': [{'bufferView': i, 'componentType': 5126, 'count': 1, 'type': 'VEC3'} for i in range(2)],
        'meshes': [{'name': 'Face', 'weights': [0.0], 'extras': {'targetNames': ['MouthOpen']},
                    'primitives': [{'attributes': {'POSITION': 0}, 'material': 0,
                                    'targets': [{'POSITION': 1}]}]}],
        'nodes': [{'name': 'Face', 'mesh': 0}],
        'materials': [{'pbrMetallicRoughness': {'baseColorTexture': {'index': 0}}}],
        'images': [{'bufferView': 0}], 'samplers': [{}],
        'textures': [{'source': 0, 'sampler': 0}],
    }
    return base, new


class RecipeTests(unittest.TestCase):
    def test_payload_compiles_without_running_model_tools(self):
        request = build_request()
        compile(request['code'], '<packaged recipe>', 'exec')
        paths = [x['path'] for x in request['outputs']]
        self.assertEqual(len(paths), len(set(paths)))
        self.assertTrue(all(x.startswith(OUTPUT_PREFIX) for x in paths))
        self.assertTrue(all(x['expectedSha256'] is None for x in request['outputs']))
        self.assertIn(OUTPUT_PREFIX + 'input-manifest.json', paths)

    def test_bundle_is_repeatable(self):
        self.assertEqual(build_request(), build_request())

    def test_generator_sources_have_no_reference_asset_dependency(self):
        root = Path(__file__).parent
        for name in SOURCE_FILES:
            source = (root / name).read_text().lower()
            for marker in ['silver_wolf', 'silverwolf', 'silver wolf', '银狼', 'recovered-reference']:
                self.assertNotIn(marker, source, name)
        self.assertNotIn('/workspace/', build_request()['code'])

    def test_merge_preserves_frozen_bytes_and_morph_indices(self):
        base, new = fixture()
        original = copy.deepcopy(base)
        oldbin = struct.pack('<3f', 1, 2, 3)
        raw, report = merge_face(glb(base, oldbin), glb(new, struct.pack('<6f', 4, 5, 6, 0, -.1, 0)))
        doc, binary = read_glb(raw)
        self.assertEqual(binary[:12], oldbin)
        self.assertEqual(doc['meshes'][0], original['meshes'][0])
        self.assertEqual(doc['meshes'][2], original['meshes'][2])
        face = doc['meshes'][1]
        self.assertEqual(face['primitives'][0]['attributes']['POSITION'], 1)
        self.assertEqual(face['primitives'][0]['targets'], [{'POSITION': 2}])
        self.assertEqual(doc['nodes'][1]['weights'], [0.0])
        self.assertEqual(doc['images'][1]['bufferView'], 1)
        self.assertEqual(doc['textures'][1], {'source': 1, 'sampler': 1})
        self.assertEqual(doc['materials'][1]['pbrMetallicRoughness']['baseColorTexture']['index'], 1)
        self.assertTrue(report['original_binary_prefix_preserved'])
        self.assertEqual(report['frozen_primitives'], {'Body': True, 'Hair': True})

    def test_missing_mouth_morph_is_rejected(self):
        base, new = fixture()
        new['meshes'][0]['primitives'][0]['targets'] = []
        with self.assertRaisesRegex(AssertionError, 'MouthOpen morph lost'):
            merge_face(glb(base, b'0' * 12), glb(new, b'1' * 24))

    def test_external_texture_is_rejected(self):
        base, new = fixture()
        new['images'][0] = {'uri': 'unapproved.png'}
        with self.assertRaisesRegex(AssertionError, 'Self-contained GLB required'):
            merge_face(glb(base, b'0' * 12), glb(new, b'1' * 24))


if __name__ == '__main__':
    unittest.main()

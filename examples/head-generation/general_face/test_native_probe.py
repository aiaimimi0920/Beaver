"""CPU fixtures and package checks only. These tests never execute Godot."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from definition_guard import FORBIDDEN_PROFILES, require_head_only_definition
from native_probe import build_manifest, CONTRACT, SCHEMA, streams
from test_glb_affine import fixture
from glb_node_affine import transport_face_node, RUNTIME_POLICY
from bundle import build_request
from glb_affine import transport_glb, read_glb, write_glb
import test_sdf_bundle
import sdf_bundle


class NativeProbeTests(unittest.TestCase):
    def test_definition_guard(self):
        require_head_only_definition('[resource]\nmodel_scene = ExtResource("1")\n')
        for key in FORBIDDEN_PROFILES:
            for value in ('null', 'ExtResource("2")', '"res://performance.json"'):
                with self.subTest(key=key, value=value), self.assertRaisesRegex(ValueError, key):
                    require_head_only_definition('[resource]\n  ' + key + ' = ' + value)
        request = build_request({'preset': 'baseline'}, 'assets/aster/general_face_test/')
        self.assertIn('require_head_only_definition(definition)', request['code'])
        self.assertTrue(any(p['path'].endswith('/definition_guard.py') for p in request['outputs']))

    def test_normalized_color_bytes_and_semantics(self):
        doc, binary = read_glb(fixture())
        offset = len(binary)
        color = bytes([0, 127, 255, 255, 20, 40, 60, 255, 255, 0, 128, 255])
        doc['bufferViews'].append({'buffer': 0, 'byteOffset': offset, 'byteLength': len(color)})
        doc['accessors'].append({'bufferView': len(doc['bufferViews']) - 1,
                                 'componentType': 5121, 'type': 'VEC4', 'normalized': True, 'count': 3})
        doc['meshes'][0]['primitives'][0]['attributes']['COLOR_0'] = len(doc['accessors']) - 1
        raw = write_glb(doc, binary + color)
        expected = streams(raw)
        self.assertEqual(streams(transport_face_node(raw, {'width': 1.06}, (0, 0, 1.641), RUNTIME_POLICY)), expected)
        changed = bytearray(binary + color)
        changed[offset + 1] += 1
        self.assertNotEqual(streams(write_glb(doc, changed)), expected)
        doc['accessors'][-1]['normalized'] = False
        self.assertNotEqual(streams(write_glb(doc, binary + color)), expected)

    def test_sdf_rejects_optional_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = test_sdf_bundle.SdfBundleTests()
            root = fixture.workspace_fixture(directory)
            for field in FORBIDDEN_PROFILES:
                (root / fixture.prefix / 'aster_definition.tres').write_text(field + ' = null')
                with self.assertRaisesRegex(ValueError, field):
                    sdf_bundle.build_request({'preset': 'baseline'}, fixture.prefix, root)

    def test_fixture_manifest_cli_and_rejections(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            raw = fixture()
            (root / 'general_base_export.glb').write_bytes(raw)
            (root / 'aster_head.glb').write_bytes(transport_face_node(
                raw, {'width': 1.06, 'height': .96}, (0, 0, 1.641), RUNTIME_POLICY))
            (root / 'aster_definition.tres').write_text('[resource]\n')
            intent = root / 'general_intent.json'
            intent.write_text(json.dumps({'proportions': {'width': 1.06, 'height': .96, 'depth': 1.}}))
            report = root / 'generation_report.json'
            report.write_text(json.dumps({'general_face': {'pivot_blender_xyz': [.01, -.02, 1.7]}}))
            case = ('soft', directory, 'res://assets/aster/general_face_soft')
            manifest = build_manifest([case])
            self.assertEqual(manifest['cases'][0]['declared_pivot'], [.01, 1.7, .02])
            self.assertEqual(manifest['schema'], SCHEMA)
            self.assertEqual(manifest['contract'], CONTRACT)
            self.assertEqual(manifest['weights'], [0., .25, .5, .75, 1.])
            self.assertEqual(manifest['cases'][0]['declared_scale'], [1.06, .96, 1.])
            self.assertTrue(manifest['cases'][0]['raw_face_streams_and_indices_exact'])
            for key in ('base', 'variant', 'definition', 'intent', 'report'):
                self.assertRegex(manifest['cases'][0][key + '_sha256'], r'^[0-9a-f]{64}$')
            output = root / 'manifest.json'
            subprocess.run([sys.executable, str(Path(__file__).with_name('native_probe.py')),
                            '--case', *case, '--output', str(output)], check=True)
            self.assertEqual(json.loads(output.read_text()), manifest)
            report.write_text('{"general_face":{"pivot_blender_xyz":[0,0,true]}}')
            with self.assertRaises(ValueError):
                build_manifest([case])
            report.write_text('{"general_face":{"pivot_blender_xyz":[0,0,1.641]}}')
            self.assertNotEqual(build_manifest([case])['cases'][0]['report_sha256'], manifest['cases'][0]['report_sha256'])
            for cases in ([], [case, case], [('soft', directory, 'res://../bad')]):
                with self.assertRaises(ValueError):
                    build_manifest(cases)
            (root / 'aster_head.glb').write_bytes(fixture(sparse=False))
            # Dense and sparse representations of identical values are allowed.
            build_manifest([case])
            (root / 'aster_head.glb').write_bytes(transport_glb(raw, {'width': 1.06}, (0, 0, 1.641)))
            with self.assertRaisesRegex(ValueError, 'streams'):
                build_manifest([case])
            (root / 'aster_head.glb').write_bytes(raw)
            intent.write_text('{"proportions":{"width":true}}')
            with self.assertRaises(ValueError):
                build_manifest([case])
            intent.write_text('{"proportions":{}}')
            (root / 'aster_definition.tres').write_text('[resource]\neye_geometry_profile = null')
            with self.assertRaisesRegex(ValueError, 'eye_geometry_profile'):
                build_manifest([case])
            (root / 'general_base_export.glb').unlink()
            with self.assertRaisesRegex(ValueError, 'Missing'):
                build_manifest([case])


if __name__ == '__main__':
    unittest.main()

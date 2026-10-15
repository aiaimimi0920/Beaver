import ast
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from sdf_bundle import adapted_bake, build_request, NPR
from prompt_contract import select_intent


class SdfBundleTests(unittest.TestCase):
    prefix = "assets/aster/general_face_test/"

    def workspace_fixture(self, directory):
        root = Path(directory)
        target = root / self.prefix
        target.mkdir(parents=True, exist_ok=True)
        (root / "project.godot").write_text("; isolated packaging-only test fixture")
        (target / "aster_head.glb").write_bytes(b"test fixture, never execute as a model")
        (target / "aster_definition.tres").write_text("; fixture definition")
        (target / "generation_report.json").write_text(json.dumps({
            "general_face": {"intent": select_intent({"preset": "baseline"})},
            "new_glb_sha256": hashlib.sha256((target / "aster_head.glb").read_bytes()).hexdigest(),
        }))
        return root

    def test_adaptation_order_axes_and_paths(self):
        config = select_intent({"proportions": {"width": 1.06, "height": .96, "depth": 1.02}})
        source = adapted_bake(config, "assets/aster/general_face_test/")
        self.assertIn('normals /= np.array([1.06, 0.96, 1.02])', source)
        self.assertLess(source.index('normals /= np.array'), source.index('lit = (normals @ light)'))
        self.assertNotIn('head_recovery_662', source)
        self.assertNotIn('head_recovery_664', source)
        self.assertIn('xx, yy = np.meshgrid((u - 0.5) * 0.235, 1.5 + 0.31 * (1 - v))', source)
        ast.parse(source)

    def test_baseline_and_readable_sources(self):
        before = {p.name: p.read_bytes() for p in NPR.glob('*.py')}
        config = select_intent({"preset": "baseline"})
        self.assertNotIn('normals /= np.array', adapted_bake(config, "assets/aster/general_face_test/"))
        with tempfile.TemporaryDirectory() as directory:
            workspace = self.workspace_fixture(directory)
            request = build_request({"preset": "baseline"}, self.prefix, workspace)
        compile(request['code'], 'native sdf job', 'exec')
        paths = [o['path'] for o in request['outputs']]
        self.assertIn('assets/aster/general_face_test/sdf/bake.py', paths)
        self.assertIn('assets/aster/general_face_test/sdf/definition_output.py', paths)
        self.assertIn('Geometry/atlas intent mismatch', request['code'])
        self.assertEqual(before, {p.name: p.read_bytes() for p in NPR.glob('*.py')})


    def test_missing_workspace_and_generated_inputs_fail(self):
        with self.assertRaises(TypeError):
            build_request({"preset": "baseline"}, self.prefix)
        with self.assertRaisesRegex(ValueError, "workspace is required"):
            build_request({"preset": "baseline"}, self.prefix, None)
        for name in ("project.godot", self.prefix + "aster_head.glb",
                     self.prefix + "aster_definition.tres", self.prefix + "generation_report.json"):
            with tempfile.TemporaryDirectory() as directory:
                root = self.workspace_fixture(directory)
                (root / name).unlink()
                with self.assertRaisesRegex(ValueError, "missing required generation inputs") as failure:
                    build_request({"preset": "baseline"}, self.prefix, root)
                self.assertIn(name, str(failure.exception))

    def test_existing_outputs_always_have_current_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.workspace_fixture(directory)
            existing_atlas = root / self.prefix / "sdf/face_sdf360.png"
            existing_atlas.parent.mkdir()
            existing_atlas.write_bytes(b"prior atlas")
            request = build_request({"preset": "baseline"}, self.prefix, root)
            hashes = {entry['path']: entry['expectedSha256'] for entry in request['outputs']}
            for name in ("aster_definition.tres", "generation_report.json", "sdf/face_sdf360.png"):
                self.assertEqual(hashes[self.prefix + name], hashlib.sha256((root / self.prefix / name).read_bytes()).hexdigest())
            self.assertIsNone(hashes[self.prefix + "sdf/bake_report.json"])
            definition = root / self.prefix / "aster_definition.tres"
            definition.write_text("changed current definition")
            refreshed = build_request({"preset": "baseline"}, self.prefix, root)
            digest = next(entry['expectedSha256'] for entry in refreshed['outputs'] if entry['path'] == self.prefix + "aster_definition.tres")
            self.assertNotEqual(digest, hashes[self.prefix + "aster_definition.tres"])
            self.assertEqual(digest, hashlib.sha256(definition.read_bytes()).hexdigest())

    def test_cli_requires_workspace_and_compiles_fixture_request(self):
        script = Path(__file__).with_name('sdf_bundle.py')
        with tempfile.TemporaryDirectory() as directory:
            root = self.workspace_fixture(directory)
            output = root / 'packaging-test-request.json'
            args = [sys.executable, str(script), '--intent', '{"preset":"baseline"}',
                    '--prefix', self.prefix, '--output', str(output)]
            missing = subprocess.run(args, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn('--workspace', missing.stderr)
            self.assertFalse(output.exists())
            success = subprocess.run(args + ['--workspace', str(root)], capture_output=True, text=True)
            self.assertEqual(success.returncode, 0, success.stderr)
            request = json.loads(output.read_text())
            compile(request['code'], '<temporary CLI packaging fixture>', 'exec')
            for name in ('aster_definition.tres', 'generation_report.json'):
                entry = next(item for item in request['outputs'] if item['path'] == self.prefix + name)
                self.assertEqual(entry['expectedSha256'], hashlib.sha256((root / self.prefix / name).read_bytes()).hexdigest())


if __name__ == '__main__':
    unittest.main()

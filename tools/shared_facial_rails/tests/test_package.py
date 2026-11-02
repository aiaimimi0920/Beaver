"""Package, command-line and import-isolation behavior."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class PackageTests(unittest.TestCase):
    def run_module(self, *arguments, cwd):
        tools_root = Path(__file__).resolve().parents[2]
        env = dict(os.environ, PYTHONPATH=str(tools_root), PYTHONDONTWRITEBYTECODE="1")
        return subprocess.run(
            [sys.executable, "-m", "shared_facial_rails", *arguments],
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_demo_stdout_from_unrelated_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = self.run_module("--synthetic-demo", cwd=temporary)
            self.assertEqual(result.returncode, 0, result.stderr)
            records = json.loads(result.stdout)
            self.assertEqual(len(records), 4)
            self.assertTrue(all(not r["native_ready"] for r in records))
            self.assertTrue(all(not r["full_model_success"] for r in records))
            self.assertEqual(list(Path(temporary).iterdir()), [])

    def test_output_is_exclusive(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "existing.json"
            output.write_bytes(b"preserve evidence")
            result = self.run_module("--synthetic-demo", "--output", str(output), cwd=temporary)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(output.read_bytes(), b"preserve evidence")

    def test_requires_explicit_demo(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = self.run_module(cwd=temporary)
            self.assertEqual(result.returncode, 2)
            self.assertIn("--synthetic-demo", result.stderr)

    def test_unrelated_flat_modules_cannot_shadow_package(self):
        with tempfile.TemporaryDirectory() as temporary:
            for module in ("geometry", "packet", "redistribute", "fixtures"):
                (Path(temporary) / (module + ".py")).write_text("raise RuntimeError('shadow')\n")
            result = self.run_module("--synthetic-demo", cwd=temporary)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(json.loads(result.stdout)), 4)


if __name__ == "__main__":
    unittest.main()

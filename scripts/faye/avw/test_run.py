import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import run
import paths


class OutputProtectionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / "validation"
        self.source.mkdir()
        for name in run.INPUTS:
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"test-fixture")

    def test_existing_output_is_never_reused(self):
        target = self.root / "existing"
        target.mkdir()
        with self.assertRaises(FileExistsError):
            run.prepare(self.source, target)

    def test_output_cannot_be_inside_input(self):
        with self.assertRaises(ValueError):
            run.prepare(self.source, self.source / "candidate")

    def test_missing_inputs_do_not_create_output(self):
        (self.source / run.INPUTS[0]).unlink()
        output = self.root / "new"
        with self.assertRaises(FileNotFoundError):
            run.prepare(self.source, output)
        self.assertFalse(output.exists())

    def test_preparation_records_inputs_and_output_marker(self):
        source, output, hashes = run.prepare(self.source, self.root / "fresh")
        self.assertEqual(len(hashes), len(run.INPUTS))
        with patch.dict(
            os.environ, FAYE_VALIDATION_ROOT=str(source), FAYE_OUTPUT_ROOT=str(output)
        ):
            self.assertEqual(paths.candidate_root(), output)

    def test_direct_execution_requires_explicit_environment(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(RuntimeError):
                paths.candidate_root()

    def test_missing_marker_is_rejected(self):
        output = self.root / "unmanaged"
        output.mkdir()
        with patch.dict(
            os.environ,
            FAYE_VALIDATION_ROOT=str(self.source),
            FAYE_OUTPUT_ROOT=str(output),
        ):
            with self.assertRaises(RuntimeError):
                paths.candidate_root()


if __name__ == "__main__":
    unittest.main()

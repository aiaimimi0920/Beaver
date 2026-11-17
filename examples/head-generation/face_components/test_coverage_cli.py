"""Exit-code contract for opt-in batch verification; fixtures are not native evidence."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

HERE = Path(__file__).parent


class StrictCoverageCLI(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.batch = Path(self.temp.name) / "batch.json"
        self.batch.write_text("[]")

    def run_fixture(self, statuses, strict):
        # Exercise the actual __main__/SystemExit path with deterministic statuses.
        report = {"schema_version": 1, "runs": [{"status": s} for s in statuses],
                  "scope": "synthetic CLI fixture, not native evidence"}
        code = (
            "import runpy,sys;from unittest.mock import patch;"
            "sys.path.insert(0,sys.argv.pop(1));"
            "import verify_run;"
            "from compose import SLOTS;"
            "statuses=" + repr(statuses) + ";"
            "results=iter({'status':s} for s in statuses);"
            "verify_run.verify_run=lambda spec:next(results);"
            "runpy.run_path('coverage.py',run_name='__main__')"
        )
        rows = [{"selection": {s: "fixture" for s in
                 ["eyes", "brows", "mouth", "nose", "ears", "face"]},
                 "fit_policy": "legacy", "prefix": "fixture"} for _ in statuses]
        self.batch.write_text(json.dumps(rows))
        command = [sys.executable, "-c", code, str(HERE), "--batch", str(self.batch)]
        if strict:
            command.append("--require-verified")
        result = subprocess.run(command, cwd=HERE, capture_output=True, text=True)
        actual = json.loads(result.stdout)
        self.assertEqual([r["status"] for r in actual["runs"]], statuses)
        self.assertIn("not certify", actual["scope"])
        self.assertEqual(result.stderr, "")
        return result, actual

    def test_strict_exit_and_report_parity(self):
        cases = [([], 1), (["technically_verified"], 0),
                 (["technically_verified", "technically_verified"], 0)]
        for status in ["incomplete", "integrity_failed", "generation_failed",
                       "sdf_failed", "source_packaging_failed"]:
            cases.extend([([status], 1), (["technically_verified", status], 1)])
        for statuses, expected in cases:
            with self.subTest(statuses=statuses):
                default, report = self.run_fixture(statuses, False)
                strict, strict_report = self.run_fixture(statuses, True)
                self.assertEqual(default.returncode, 0)
                self.assertEqual(strict.returncode, expected)
                self.assertEqual(report, strict_report)

    def test_real_invalid_spec_preserves_json_and_fails_strict(self):
        self.batch.write_text("[{}]")
        args = [sys.executable, str(HERE / "coverage.py"), "--batch", str(self.batch)]
        default = subprocess.run(args, capture_output=True, text=True)
        strict = subprocess.run(args + ["--require-verified"], capture_output=True, text=True)
        self.assertEqual(default.returncode, 0)
        self.assertEqual(strict.returncode, 1)
        self.assertEqual(json.loads(default.stdout), json.loads(strict.stdout))
        self.assertEqual(json.loads(strict.stdout)["runs"][0]["status"], "integrity_failed")

    def test_missing_receipt_is_not_success(self):
        self.batch.write_text(json.dumps([{
            "selection": {s: "fixture" for s in ["eyes", "brows", "mouth", "nose", "ears", "face"]},
            "fit_policy": "legacy", "workspace": self.temp.name,
            "prefix": "assets/aster/general_face_eyes_combofixture/",
            "generation_receipt": str(Path(self.temp.name) / "missing.json"),
        }]))
        result = subprocess.run([sys.executable, str(HERE / "coverage.py"), "--batch",
                                 str(self.batch), "--require-verified"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(json.loads(result.stdout)["runs"][0]["status"], "incomplete")

    def test_malformed_input_cannot_exit_success(self):
        self.batch.write_text("{broken")
        result = subprocess.run([sys.executable, str(HERE / "coverage.py"), "--batch",
                                 str(self.batch), "--require-verified"], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()

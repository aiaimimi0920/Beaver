"""Focused exit/report regressions; no product builds or network needed."""
import argparse
import importlib.util
import json
import os
from unittest.mock import patch
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("development_quality.py")
spec = importlib.util.spec_from_file_location("quality", SCRIPT)
quality = importlib.util.module_from_spec(spec)
spec.loader.exec_module(quality)


class ClassificationTests(unittest.TestCase):
    def classify(self, kind, code, stdout="", stderr="", report=None):
        return quality.classify(kind, code, stdout, stderr, report)

    def reject(self, *args):
        with self.assertRaises((ValueError, TypeError, KeyError)):
            self.classify(*args)

    def test_rustfmt_clean_and_diff(self):
        self.assertEqual(self.classify("rustfmt", 0), 0)
        self.assertEqual(self.classify("rustfmt", 1, "Diff in C:/x.rs:1:\n-old\n+new\n"), 1)

    def test_rustfmt_failures_are_not_findings(self):
        for code, stdout, stderr in [
            (1, "", ""), (2, "", ""), (1, "", "error: parse failed"),
            (1, "Diff in x.rs:1:\n", "error: another file failed"),
            (1, "Diff in x.rs:1:\nerror: failed\n", ""), (0, "unexpected", ""),
        ]:
            with self.subTest(code=code, stdout=stdout, stderr=stderr):
                self.reject("rustfmt", code, stdout, stderr)

    def test_eslint_findings_and_fatal_errors(self):
        row = dict(filePath="x.js", errorCount=1, warningCount=0,
                   fatalErrorCount=0, messages=[{"ruleId": "no-unused-vars"}])
        self.assertEqual(self.classify("eslint", 1, json.dumps([row])), 1)
        self.reject("eslint", 0, json.dumps([row]))
        row["fatalErrorCount"] = 1
        self.reject("eslint", 1, json.dumps([row]))
        for raw in ["", "[]", "{}", "not-json"]:
            self.reject("eslint", 0, raw)
        self.reject("eslint", 2, json.dumps([row]))

    def test_clippy_requires_successful_compilation(self):
        warning = {"reason": "compiler-message", "message": {"level": "warning"}}
        done = {"reason": "build-finished", "success": True}
        payload = "\n".join(map(json.dumps, [warning, done]))
        self.assertEqual(self.classify("clippy", 0, payload), 1)
        self.reject("clippy", 101, payload)
        self.reject("clippy", 0, json.dumps(warning))
        self.reject("clippy", 0, json.dumps(dict(done, success=False)))
        warning["message"]["level"] = "error"
        self.reject("clippy", 0, "\n".join(map(json.dumps, [warning, done])))

    def test_loom_report_preserves_completeness_failures(self):
        report = dict(schemaVersion=1, mode="ratchet", summary={"scanned": 8},
                      warnings=[], violations=["x.rs: oversized baseline file changed before reaching 700 lines"])
        self.assertEqual(self.classify("loom-lines", 1, report=report), 1)
        for finding in ["x: symbolic links and junctions are not scanned",
                        "x: directory escapes repository root", "unknown new diagnostic"]:
            report["violations"] = [finding]
            self.reject("loom-lines", 1, "", "", report)
        report.update(violations=[], summary={"scanned": 0})
        self.reject("loom-lines", 0, "", "", report)
        self.reject("loom-lines", 2, "", "", report)

    def test_beaver_report_and_tool_exits(self):
        report = dict(ok=False, files=[{"path": "x.rs", "status": "splitRequired"}],
                      violations=["x.rs: 900 effective lines; split into responsibility-owned modules (no exception above 700)"])
        self.assertEqual(self.classify("beaver-lines", 1, report=report), 1)
        self.reject("beaver-lines", 2, "", "", report)
        self.reject("beaver-lines", 0, "", "", report)
        report["violations"] = ["x.rs: stale or invalid 501-700 line exception; split or update its evidence"]
        self.reject("beaver-lines", 1, "", "", report)

    def invoke(self, root, code, report=None):
        args = argparse.Namespace(kind="rustfmt", output=str(root / "evidence"),
                                  report=report, timeout=10,
                                  command=[sys.executable, "-c", code])
        return quality.run(args)

    def test_process_failure_retains_raw_exit_and_report(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            self.assertEqual(self.invoke(root, "import sys; print('broken'); sys.exit(3)"), 2)
            data = json.loads((root / "evidence/result.json").read_text())
            self.assertEqual(data["status"], "tool_error")
            self.assertEqual(data["exit_code"], 3)
            self.assertEqual((root / "evidence/stdout.log").read_text().strip(), "broken")

    def test_missing_and_stale_reports_fail(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            self.assertEqual(self.invoke(root, "pass", str(root / "missing.json")), 2)
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            report = root / "stale.json"
            report.write_text("{}")
            self.assertEqual(self.invoke(root, "pass", str(report)), 2)

    def test_unknown_diagnostics_fail_closed(self):
        self.reject("rustfmt", 1, "Diff in x.rs:1:\nUNKNOWN TOOL FAILURE\n")
        row = dict(filePath="x.js", errorCount=0, warningCount=0, fatalErrorCount=0, messages=[])
        self.reject("eslint", 0, json.dumps([row]), "ERROR plugin execution incomplete")
        done = json.dumps({"reason": "build-finished", "success": True})
        self.reject("clippy", 0, done, "ERROR unknown tool diagnostic")

    def test_strict_candidate_blocks_valid_findings(self):
        with tempfile.TemporaryDirectory() as root, patch.dict(os.environ, {"QUALITY_STRICT": "true"}):
            root = Path(root)
            code = "import sys; print('Diff in x.rs:1:'); sys.exit(1)"
            self.assertEqual(self.invoke(root, code), 1)
            self.assertEqual(json.loads((root / "evidence/result.json").read_text())["status"], "findings")

    def test_timeout_retains_partial_logs(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            args = argparse.Namespace(kind="rustfmt", output=str(root / "evidence"),
                                      report=None, timeout=0.3,
                                      command=[sys.executable, "-u", "-c",
                                               "import time; print('partial evidence', flush=True); time.sleep(5)"])
            self.assertEqual(quality.run(args), 2)
            self.assertIn("partial evidence", (root / "evidence/stdout.log").read_text())

    @unittest.skipUnless(shutil.which("cargo") and shutil.which("cargo-clippy"), "Clippy is not installed")
    def test_real_clippy_deny_lint_and_compile_error(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            (root / "src").mkdir()
            (root / "Cargo.toml").write_text('[package]\nname="quality-fixture"\nversion="0.1.0"\nedition="2021"\n')
            source = root / "src/main.rs"
            command = ["cargo", "clippy", "--offline", "--manifest-path", str(root / "Cargo.toml"),
                       "--message-format=json", "--", "--cap-lints", "warn"]
            source.write_text('#![deny(clippy::len_zero)]\nfn main() { let v = Vec::<u8>::new(); if v.len() == 0 { println!("empty"); } }\n')
            p = subprocess.run(command, capture_output=True)
            self.assertGreater(self.classify("clippy", p.returncode, p.stdout.decode(), p.stderr.decode()), 0)
            source.write_text("fn main() { let x: u8 = false; }")
            p = subprocess.run(command, capture_output=True)
            self.reject("clippy", p.returncode, p.stdout.decode(), p.stderr.decode())

    @unittest.skipUnless(shutil.which("rustfmt"), "rustfmt is not installed")
    def test_real_rustfmt_diff_parse_error_and_clean(self):
        with tempfile.TemporaryDirectory() as root:
            source = Path(root) / "fixture.rs"
            for text, expected in [
                ("fn main(){println!(\"hi\");}\n", "findings"),
                ("fn main() {\n    println!(\"hi\");\n}\n", "clean"),
                ("fn main( {\n", "error"),
            ]:
                source.write_text(text, encoding="utf-8")
                p = subprocess.run(["rustfmt", "--check", str(source)], capture_output=True)
                out, err = p.stdout.decode(), p.stderr.decode()
                if expected == "error":
                    self.reject("rustfmt", p.returncode, out, err)
                else:
                    count = self.classify("rustfmt", p.returncode, out, err)
                    self.assertEqual(count > 0, expected == "findings")


if __name__ == "__main__":
    unittest.main()

"""Offline preflight regression checks; no credentials or network requests."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("send-probe.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class PreflightTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "request.json"
        self.raw = json.dumps({"request_id": "test_1", "queue_id": "smoke-test",
            "kind": "connectivity_check", "expected_result": "BEAVER_DOT_SMOKE_OK"}).encode()
        self.path.write_bytes(self.raw)
        self.digest = hashlib.sha256(self.raw).hexdigest()

    def check(self, site="https://example.chatgpt.site", digest=None, request="test_1"):
        return probe.validate(self.path, digest or self.digest, request, site)

    def test_exact_request(self):
        self.assertEqual(self.check(), ("https://example.chatgpt.site/api/demo", self.digest))

    def test_changed_bytes_or_identity(self):
        for args in ({"digest": "0" * 64}, {"request": "other"}):
            with self.assertRaises(ValueError):
                self.check(**args)

    def test_destination_restrictions(self):
        for site in ("http://example.chatgpt.site", "https://example.com",
            "https://a.chatgpt.site.evil.com", "https://u:p@a.chatgpt.site",
            "https://a.chatgpt.site:443", "https://a.chatgpt.site/path"):
            with self.assertRaises(ValueError):
                self.check(site=site)

    def test_wrong_marker(self):
        self.raw = self.raw.replace(b"BEAVER_DOT_SMOKE_OK", b"execute_shell")
        self.path.write_bytes(self.raw)
        with self.assertRaises(ValueError):
            self.check(digest=hashlib.sha256(self.raw).hexdigest())

    def test_redirect_refused(self):
        with self.assertRaises(RuntimeError):
            probe.NoRedirect().redirect_request(None, None, 302, None, None, "https://other")


if __name__ == "__main__":
    unittest.main()

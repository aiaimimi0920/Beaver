"""Exercise the real loopback protocol without importing Blender."""

import http.client
import json
from pathlib import Path
import sys
import threading
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "resources" / "workflows"))
from asset_observer_transport import ObserverState, start_transport


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.state = ObserverState(dict(
            taskId="task-one", projectId="project-one", sessionId="session-one",
            token="unit-test-secret",
        ))
        self.server = start_transport(self.state)
        self.addCleanup(self.server.server_close)
        self.addCleanup(self.server.shutdown)
        self.headers = {
            "Authorization": "Bearer unit-test-secret",
            "X-Beaver-Task": "task-one",
            "X-Beaver-Project": "project-one",
            "X-Beaver-Session": "session-one",
        }

    def request(self, path, data=None, headers=None, method="POST", body=None):
        connection = http.client.HTTPConnection(*self.server.server_address, timeout=6)
        try:
            payload = json.dumps(data if data is not None else {}) if body is None else body
            connection.request(method, path, payload, self.headers if headers is None else headers)
            response = connection.getresponse()
            content = response.read()
            result = content if response.getheader("Content-Type") == "image/png" else json.loads(content)
            return response.status, result
        finally:
            connection.close()

    def test_every_identity_field_and_origin_are_checked(self):
        self.assertEqual(self.request("/status")[0], 200)
        for field in self.headers:
            headers = dict(self.headers, **{field: "wrong"})
            with self.subTest(field=field):
                self.assertEqual(self.request("/status", headers=headers)[0], 403)
                self.assertEqual(self.request("/frame/current", headers=headers, method="GET")[0], 403)
        self.assertEqual(self.request("/status", headers={})[0], 403)
        self.assertEqual(self.request("/status", headers=dict(self.headers, Origin="https://example.com"))[0], 403)

    def test_status_lease_limit_renewal_and_release(self):
        for index in range(8):
            status, result = self.request("/status", {"subscriber": f"window-{index}"})
            self.assertEqual(status, 200)
            self.assertEqual(result["taskId"], "task-one")
        self.assertTrue(self.state.subscribed())
        self.assertEqual(self.request("/status", {"subscriber": "ninth"})[0], 409)
        self.assertEqual(self.request("/status", {"subscriber": "window-0"})[0], 200)
        self.assertEqual(self.request("/status", {"subscriber": "window-0", "release": True})[0], 200)
        self.assertEqual(self.request("/status", {"subscriber": "ninth"})[0], 200)
        self.assertEqual(len(self.state.leases), 8)

    def test_body_shape_size_and_unknown_operations_are_rejected(self):
        for body in ("[]", "null", "not-json", " "):
            with self.subTest(body=body):
                self.assertEqual(self.request("/status", body=body)[0], 409)
        for size in ("-1", "1", "32769", "invalid"):
            headers = dict(self.headers, **{"Content-Length": size})
            self.assertEqual(self.request("/status", headers=headers)[0], 409)
        self.assertEqual(self.request("/execute", {"code": "not allowed"})[0], 409)
        self.assertEqual(self.request("/status", {"subscriber": "x" * 101})[0], 409)

    def test_frame_bytes_and_metadata_share_the_bounded_frame_identity(self):
        for index in range(4):
            self.state.put_frame({"id": f"frame-{index}"}, bytes([index]))
        self.assertEqual(self.request("/frame/frame-0", method="GET")[0], 409)
        self.assertEqual(self.request("/frameMeta", {"frameId": "frame-0"})[0], 409)
        self.assertEqual(self.request("/frame/frame-3", method="GET"), (200, b"\x03"))
        self.assertEqual(self.request("/frameMeta", {"frameId": "frame-3"}), (200, {"id": "frame-3"}))

    def test_commands_wait_for_main_thread_receipt_and_are_bounded(self):
        observed = []

        def main_thread_stub():
            operation, data, event, response, deadline = self.state.commands.get(timeout=3)
            observed.append((operation, data, deadline))
            response["result"] = {"checkpoint": "saved.blend"}
            event.set()

        consumer = threading.Thread(target=main_thread_stub)
        consumer.start()
        status, result = self.request("/checkpoint", {"reason": "before-edit"})
        consumer.join(timeout=3)
        self.assertFalse(consumer.is_alive())
        self.assertEqual((status, result), (200, {"checkpoint": "saved.blend"}))
        self.assertEqual(observed[0][:2], ("checkpoint", {"reason": "before-edit"}))
        self.assertGreater(observed[0][2], time.monotonic())
        for _ in range(4):
            self.state.commands.put_nowait(None)
        self.assertEqual(self.request("/pick", {"frameId": "current"})[0], 409)
        self.assertEqual(self.state.commands.qsize(), 4)


if __name__ == "__main__":
    unittest.main()

"""Resource pressure and coalescing checks for the Blender observation bridge."""

from http.server import BaseHTTPRequestHandler
from pathlib import Path
import socket
import sys
import threading
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "resources" / "workflows"))
from asset_observer_transport import BoundedServer, ObserverState


class StateTests(unittest.TestCase):
    def setUp(self):
        self.state = ObserverState(dict(taskId="task", projectId="project", sessionId="session"))

    def test_fast_drag_keeps_only_latest_view_without_occupying_main_thread_queue(self):
        for seq in range(1, 1001):
            self.state.set_view(dict(self.state.view, seq=seq, yaw=seq / 100))
        self.state.set_view(dict(self.state.view, seq=999, yaw=0))
        self.assertEqual(self.state.snapshot()["view"]["seq"], 1000)
        self.assertEqual(self.state.snapshot()["view"]["yaw"], 10)
        self.assertEqual(self.state.commands.qsize(), 0)

    def test_invalid_views_cannot_poison_last_valid_camera(self):
        original = dict(self.state.view)
        invalid = [
            dict(original, seq=True), dict(original, target=[0, 0]),
            dict(original, yaw=float("nan")), dict(original, distance=float("inf")),
            dict(original, width=500.5), dict(original, height=961),
            dict(original, pitch=2), dict(original, target=[0, 0, 100001]),
            dict(original, seq=2 ** 53), dict(original, extra="field"),
        ]
        for view in invalid:
            with self.subTest(view=view), self.assertRaises(ValueError):
                self.state.set_view(view)
            self.assertEqual(self.state.view, original)

    def test_unrenewed_leases_expire_and_stop_sampling(self):
        with patch("asset_observer_transport.time.monotonic", return_value=100):
            self.state.leases = {"expired": 99, "active": 105}
            self.assertTrue(self.state.subscribed())
            self.assertEqual(list(self.state.leases), ["active"])
        with patch("asset_observer_transport.time.monotonic", return_value=106):
            self.assertFalse(self.state.subscribed())
            self.assertEqual(self.state.leases, {})

    def test_frame_cache_keeps_three_latest_images_and_reports_main_thread_age(self):
        self.state.last_tick = 10
        self.state.busy = {"operation": "modifier", "startedAt": 11000}
        for index in range(100):
            self.state.put_frame({"id": str(index)}, b"image")
        self.assertEqual(list(self.state.frames), ["97", "98", "99"])
        status = self.state.snapshot()
        self.assertEqual(status["frame"]["id"], "99")
        self.assertEqual(status["mainThreadAt"], 10000)
        self.assertEqual(status["busy"]["operation"], "modifier")


class ThreadBoundTests(unittest.TestCase):
    def test_extra_http_connection_is_closed_and_slot_recovers_after_release(self):
        release = threading.Event()
        saturated = threading.Event()
        lock = threading.Lock()
        count = 0

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                nonlocal count
                with lock:
                    count += 1
                    if count == 12:
                        saturated.set()
                release.wait(5)
                self.send_response(204)
                self.end_headers()

        server = BoundedServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        clients = []
        try:
            for _ in range(12):
                client = socket.create_connection(server.server_address, timeout=2)
                client.sendall(b"GET / HTTP/1.0\r\n\r\n")
                clients.append(client)
            self.assertTrue(saturated.wait(3), "HTTP worker pool did not fill")
            extra = socket.create_connection(server.server_address, timeout=2)
            try:
                extra.sendall(b"GET / HTTP/1.0\r\n\r\n")
                try:
                    self.assertEqual(extra.recv(128), b"")
                except (ConnectionResetError, ConnectionAbortedError):
                    pass
            finally:
                extra.close()
            release.set()
            for client in clients:
                self.assertIn(b"204", client.recv(128))
            deadline = time.monotonic() + 2
            recovered = False
            while time.monotonic() < deadline:
                with socket.create_connection(server.server_address, timeout=2) as client:
                    client.sendall(b"GET / HTTP/1.0\r\n\r\n")
                    try:
                        recovered = b"204" in client.recv(128)
                    except (ConnectionResetError, ConnectionAbortedError):
                        pass
                if recovered:
                    break
                time.sleep(0.01)
            self.assertTrue(recovered, "HTTP worker slots leaked")
        finally:
            release.set()
            for client in clients:
                client.close()
            server.shutdown()
            server.server_close()
            thread.join(timeout=3)


if __name__ == "__main__":
    unittest.main()

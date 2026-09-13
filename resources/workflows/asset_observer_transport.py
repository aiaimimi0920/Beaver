"""Authenticated loopback transport. This module never imports bpy or GPU APIs."""

from collections import OrderedDict
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hmac
import json
import math
import queue
import threading
import time


class ObserverState:
    def __init__(self, config):
        self.config = config
        self.lock = threading.RLock()
        self.frames = OrderedDict()
        self.leases = {}
        self.commands = queue.Queue(maxsize=4)
        self.view = dict(target=[0, 0, 1], yaw=0.65, pitch=0.2, distance=5,
                         width=720, height=540, seq=0)
        self.busy = None
        self.error = None
        self.revision = 0
        self.generation = "initial"
        self.last_tick = time.time()

    def subscribed(self):
        with self.lock:
            now = time.monotonic()
            self.leases = {key: expiry for key, expiry in self.leases.items() if expiry > now}
            return bool(self.leases)

    def snapshot(self):
        with self.lock:
            frame = next(reversed(self.frames.values()))[0] if self.frames else None
            return dict(protocolVersion=1, taskId=self.config["taskId"],
                        projectId=self.config["projectId"], sessionId=self.config["sessionId"],
                        generation=self.generation, sceneRevision=self.revision,
                        viewRevision=self.view["seq"], view=dict(self.view), frame=frame, busy=self.busy,
                        error=self.error, mainThreadAt=int(self.last_tick * 1000))

    def put_frame(self, meta, png):
        with self.lock:
            self.frames[meta["id"]] = (meta, png)
            while len(self.frames) > 3:
                self.frames.popitem(last=False)
            self.error = None

    def set_view(self, value):
        required = {"target", "yaw", "pitch", "distance", "width", "height", "seq"}
        if not isinstance(value, dict) or set(value) != required or not isinstance(value["target"], list) or len(value["target"]) != 3:
            raise ValueError("Invalid observer view")
        numbers = value["target"] + [value[k] for k in required if k != "target"]
        if any(type(n) not in (float, int) or not math.isfinite(n) for n in numbers):
            raise ValueError("Nonfinite observer view")
        if not (128 <= value["width"] <= 1280 and 128 <= value["height"] <= 960
                and 0.02 <= value["distance"] <= 100000 and abs(value["pitch"]) <= 1.55
                and abs(value["yaw"]) <= 100000 and max(map(abs, value["target"])) <= 100000):
            raise ValueError("Observer view out of bounds")
        if any(int(value[k]) != value[k] for k in ("width", "height", "seq")) or not 0 <= value["seq"] <= 2 ** 53 - 1:
            raise ValueError("Observer size and sequence must be integers")
        with self.lock:
            if value["seq"] > self.view["seq"]:
                self.view = dict(value)
        return self.snapshot()


class BoundedServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address, handler):
        self.slots = threading.BoundedSemaphore(12)
        super().__init__(address, handler)

    def process_request(self, request, client_address):
        if not self.slots.acquire(blocking=False):
            request.close()
            return
        try:
            super().process_request(request, client_address)
        except BaseException:
            self.slots.release()
            raise

    def process_request_thread(self, request, client_address):
        try:
            super().process_request_thread(request, client_address)
        finally:
            self.slots.release()


def start_transport(state):
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def setup(self):
            super().setup()
            self.connection.settimeout(6)

        def authorized(self):
            config = state.config
            return (not self.headers.get("Origin")
                    and hmac.compare_digest(self.headers.get("Authorization", ""), "Bearer " + config["token"])
                    and self.headers.get("X-Beaver-Task") == config["taskId"]
                    and self.headers.get("X-Beaver-Project") == config["projectId"]
                    and self.headers.get("X-Beaver-Session") == config["sessionId"])

        def reply(self, code, value, content_type="application/json"):
            body = value if isinstance(value, bytes) else json.dumps(value).encode("utf-8")
            self.send_response(code)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if not self.authorized():
                self.reply(403, {"error": "Invalid observer identity"})
                return
            with state.lock:
                frame = state.frames.get(self.path.removeprefix("/frame/")) if self.path.startswith("/frame/") else None
            if frame:
                self.reply(200, frame[1], "image/png")
            else:
                self.reply(409, {"error": "Reference frame expired; refresh before selecting"})

        def do_POST(self):
            if not self.authorized():
                self.reply(403, {"error": "Invalid observer identity"})
                return
            try:
                size = int(self.headers.get("Content-Length", "0"))
                if size < 2 or size > 32768:
                    raise ValueError("Invalid request size")
                data = json.loads(self.rfile.read(size))
                if not isinstance(data, dict):
                    raise ValueError("Observer request must be an object")
                if self.path == "/status":
                    state.subscribed()
                    lease = data.get("subscriber")
                    if lease is not None:
                        if not isinstance(lease, str) or not 1 <= len(lease) <= 100:
                            raise ValueError("Invalid subscriber")
                        with state.lock:
                            if data.get("release"):
                                state.leases.pop(lease, None)
                            elif len(state.leases) < 8 or lease in state.leases:
                                state.leases[lease] = time.monotonic() + 5
                            else:
                                raise ValueError("Observer subscriber limit reached")
                    result = state.snapshot()
                elif self.path == "/view":
                    result = state.set_view(data)
                elif self.path == "/frameMeta":
                    with state.lock:
                        frame = state.frames.get(data.get("frameId"))
                        if frame is None:
                            raise ValueError("Reference frame expired; refresh before selecting")
                        result = frame[0]
                elif self.path in ("/pick", "/validate", "/checkpoint"):
                    event = threading.Event()
                    response = {}
                    state.commands.put_nowait((self.path[1:], data, event, response, time.monotonic() + 4))
                    if not event.wait(4):
                        self.reply(409, {"error": "Blender main thread is busy; wait for the operation to finish"})
                        return
                    if "error" in response:
                        raise ValueError(response["error"])
                    result = response["result"]
                else:
                    raise ValueError("Unknown observer operation")
                self.reply(200, result)
            except (ValueError, TypeError, KeyError, queue.Full) as error:
                self.reply(409, {"error": str(error) or "Observer queue is full"})

    server = BoundedServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, name="BeaverObserverHTTP", daemon=True).start()
    return server

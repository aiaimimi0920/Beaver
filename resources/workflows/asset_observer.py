"""Own the observer main-thread lifecycle; scene writers use Blender's same thread."""

from pathlib import Path
import queue
import time
import uuid

import bpy
from bpy.app.handlers import persistent
from asset_observer_transport import ObserverState, start_transport
from asset_observer_view import ObserverView


class Observer:
    def __init__(self, config):
        self.state = ObserverState(config)
        self.view = ObserverView(self.state)
        self.server = start_transport(self.state)
        self.next_capture = 0
        self.dirty = True
        self.last_view = -1
        self.directory = Path(config["checkpoints"]).resolve()
        self.directory.mkdir(parents=True, exist_ok=True)

        @persistent
        def changed(_scene, depsgraph):
            if depsgraph.updates:
                self.invalidate()

        @persistent
        def loaded(_dummy):
            with self.state.lock:
                self.state.generation = uuid.uuid4().hex
                self.state.frames.clear()
            self.view.release()
            self.view.identities.clear()
            self.invalidate()
            if not bpy.app.timers.is_registered(self.tick):
                bpy.app.timers.register(self.tick, first_interval=0.1, persistent=True)

        self.changed = changed
        self.loaded = loaded
        bpy.app.handlers.depsgraph_update_post.append(changed)
        bpy.app.handlers.load_post.append(loaded)
        bpy.app.timers.register(self.tick, first_interval=0.1, persistent=True)

    def invalidate(self):
        with self.state.lock:
            self.state.revision += 1
        self.dirty = True

    def begin_operation(self, operation):
        with self.state.lock:
            self.state.busy = dict(operation=operation, startedAt=int(time.time() * 1000))

    def end_operation(self):
        with self.state.lock:
            self.state.busy = None
        bpy.context.view_layer.update()
        self.dirty = True

    def checkpoint(self):
        if sum(1 for _ in self.directory.glob("*.blend")) >= 160:
            raise RuntimeError("Recovery scene limit reached; retained checkpoints must be reviewed")
        path = self.directory / (uuid.uuid4().hex + ".blend")
        bpy.ops.wm.save_as_mainfile(filepath=str(path), copy=True, check_existing=False)
        if not path.is_file() or path.stat().st_size < 12:
            raise RuntimeError("Blender did not save the recovery scene")
        return dict(path=str(path), savedAt=int(time.time() * 1000),
                    generation=self.state.generation, sceneRevision=self.state.revision)

    def tick(self):
        self.state.last_tick = time.time()
        for _ in range(2):
            try:
                method, data, event, response, deadline = self.state.commands.get_nowait()
            except queue.Empty:
                break
            try:
                if time.monotonic() >= deadline:
                    raise RuntimeError("Observer command expired while Blender was busy")
                if method == "pick":
                    response["result"] = self.view.pick(data)
                elif method == "validate":
                    response["result"] = self.view.validate(data)
                else:
                    self.begin_operation("save recovery scene")
                    response["result"] = self.checkpoint()
            except Exception as error:
                response["error"] = str(error)
            finally:
                if method == "checkpoint":
                    with self.state.lock:
                        self.state.busy = None
                event.set()
        if not self.state.subscribed():
            self.view.release()
            return 0.1
        now = time.monotonic()
        with self.state.lock:
            sequence = self.state.view["seq"]
        if now >= self.next_capture and (self.dirty or sequence != self.last_view or now >= self.next_capture + 0.6):
            try:
                self.view.capture()
                self.last_view = sequence
                self.dirty = False
            except Exception as error:
                with self.state.lock:
                    self.state.error = str(error)
            self.next_capture = time.monotonic() + 0.12
        return 0.03

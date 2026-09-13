"""Main-thread offscreen view and frame-specific evaluated-geometry picking."""

import hashlib
import json
import math
import struct
import time
import uuid
import zlib

import bpy
import gpu
from mathutils import Matrix, Vector


def encode_png(width, height, pixels):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    stride = width * 4
    rows = b"".join(b"\x00" + pixels[y * stride:(y + 1) * stride] for y in reversed(range(height)))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows, 3)) + chunk(b"IEND", b""))


def flatten(matrix):
    return [float(value) for row in matrix for value in row]


def matrix(values):
    return Matrix([values[i:i + 4] for i in range(0, 16, 4)])


class ObserverView:
    def __init__(self, state):
        self.state = state
        self.buffer = None
        self.size = None
        self.identities = {}

    def release(self):
        if self.buffer is not None:
            self.buffer.free()
            self.buffer = None
            self.size = None

    def capture(self):
        with self.state.lock:
            view = dict(self.state.view)
        width, height = int(view["width"]), int(view["height"])
        if self.size != (width, height):
            self.release()
            self.buffer = gpu.types.GPUOffScreen(width, height)
            self.size = (width, height)
        target = Vector(view["target"])
        yaw, pitch, distance = view["yaw"], view["pitch"], view["distance"]
        eye = target + Vector((math.sin(yaw) * math.cos(pitch), -math.cos(yaw) * math.cos(pitch), math.sin(pitch))) * distance
        camera = (Matrix.Translation(eye) @ (target - eye).to_track_quat('-Z', 'Y').to_matrix().to_4x4()).inverted()
        near, far = max(0.001, distance / 1000), max(1000, distance * 100)
        focal = 1 / math.tan(math.radians(45) / 2)
        projection = Matrix(((focal * height / width, 0, 0, 0), (0, focal, 0, 0),
                             (0, 0, -(far + near) / (far - near), -2 * far * near / (far - near)), (0, 0, -1, 0)))
        context = next(((window, area, region) for window in bpy.context.window_manager.windows
                        for area in window.screen.areas if area.type == 'VIEW_3D'
                        for region in area.regions if region.type == 'WINDOW'), None)
        if context is None:
            raise RuntimeError("No 3D viewport is available in the owned Blender window")
        window, area, region = context
        with bpy.context.temp_override(window=window, area=area, region=region):
            self.buffer.draw_view3d(bpy.context.scene, bpy.context.view_layer, area.spaces.active,
                                    region, camera, projection, do_color_management=True, draw_background=True)
            with self.buffer.bind():
                pixels = gpu.state.active_framebuffer_get().read_color(0, 0, width, height, 4, 0, 'UBYTE')
                pixels.dimensions = width * height * 4
                raw = bytes(pixels)
        png = encode_png(width, height, raw)
        with self.state.lock:
            meta = dict(id=uuid.uuid4().hex, sessionId=self.state.config["sessionId"],
                        generation=self.state.generation, sceneRevision=self.state.revision,
                        viewRevision=view["seq"], capturedAt=int(time.time() * 1000), width=width, height=height,
                        viewMatrix=flatten(camera), projectionMatrix=flatten(projection))
        self.state.put_frame(meta, png)

    def pick(self, data):
        point = data.get("point")
        if not isinstance(point, list) or len(point) != 2 or any(not isinstance(v, (int, float)) or not math.isfinite(v) or not 0 <= v <= 1 for v in point):
            raise ValueError("Pick requires normalized image coordinates")
        frame = self.validate(data)
        inverse = (matrix(frame["projectionMatrix"]) @ matrix(frame["viewMatrix"])).inverted()
        points = []
        for depth in (-1, 1):
            p = inverse @ Vector((point[0] * 2 - 1, 1 - point[1] * 2, depth, 1))
            points.append(p.xyz / p.w)
        origin, end = points
        direction = (end - origin).normalized()
        depsgraph = bpy.context.evaluated_depsgraph_get()
        nearest = None
        for instance in depsgraph.object_instances:
            obj = instance.object
            if obj.type != 'MESH' or obj.hide_get() or obj.hide_viewport:
                continue
            transform = instance.matrix_world.copy()
            try:
                inverse_world = transform.inverted()
            except ValueError:
                continue
            local_origin = inverse_world @ origin
            local_direction = (inverse_world.to_3x3() @ direction).normalized()
            hit, local, normal, face = obj.ray_cast(local_origin, local_direction, depsgraph=depsgraph)
            if not hit:
                continue
            world = transform @ local
            distance = (world - origin).length
            if nearest is not None and nearest[0] <= distance:
                continue
            original = obj.original
            uid = getattr(original, "session_uid", None)
            if uid is None:
                uid = self.identities.setdefault(original.as_pointer(), uuid.uuid4().hex)
            identity = f"{frame['sessionId']}:{frame['generation']}:{uid}"
            instance_key = json.dumps([list(instance.persistent_id), flatten(transform)], separators=(",", ":"))
            instance_id = identity + ":" + hashlib.sha256(instance_key.encode()).hexdigest()[:20] if instance.is_instance else identity
            nearest = (distance, dict(frameId=frame["id"], objectId=identity, objectName=original.name,
                        instanceId=instance_id, local=list(local), world=list(world),
                        normal=list((inverse_world.transposed().to_3x3() @ normal).normalized()),
                        face=face, vertices=len(obj.data.vertices), polygons=len(obj.data.polygons), point=point))
        return nearest[1] if nearest else None

    def validate(self, data):
        # The authenticated native host owns frozen metadata, not the three-frame ring.
        frame = data.get("frame")
        bpy.context.view_layer.update()
        with self.state.lock:
            if (not isinstance(frame, dict) or frame.get("sessionId") != self.state.config["sessionId"]
                    or frame.get("generation") != self.state.generation
                    or frame.get("sceneRevision") != self.state.revision):
                raise ValueError("Reference frame expired; refresh before selecting")
        for key in ("viewMatrix", "projectionMatrix"):
            values = frame.get(key)
            if not isinstance(values, list) or len(values) != 16 or any(type(v) not in (int, float) or not math.isfinite(v) for v in values):
                raise ValueError("Invalid frozen view matrix")
        return frame

"""Command-driven camera preview of one immutable Blender scene frame."""
import json
import hashlib
import math
import shutil
import sys
import time
from pathlib import Path

import bpy
from mathutils import Matrix, Quaternion, Vector

sys.path.insert(0, str(Path(__file__).parent))
from preview import prepare
from blender_picking import FrozenMeshes


def atomic_json(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value), encoding="utf-8")
    temporary.replace(path)


def run():
    source, root, destination, width, height, session = sys.argv[sys.argv.index("--") + 1:]
    output = Path(destination)
    scene, _ = prepare(source, root, width, height)
    bpy.context.view_layer.update()
    camera = scene.camera
    if camera.data.type not in {"PERSP", "ORTHO"}:
        raise RuntimeError("BLENDER_PREVIEW_CAMERA_UNSUPPORTED")
    # Keep camera constraints/animation from overriding the independent observation camera.
    observer = bpy.data.objects.new("BeaverInteractiveCamera", camera.data.copy())
    scene.collection.objects.link(observer)
    observer.matrix_world = camera.matrix_world.copy()
    scene.camera = observer
    initial = observer.matrix_world.copy()
    basis = initial.to_quaternion()
    right, up, back = (basis @ Vector(axis) for axis in (
        (1, 0, 0), (0, 1, 0), (0, 0, 1)))
    points = [obj.matrix_world @ Vector(corner)
              for obj in scene.objects if obj.type == "MESH" and not obj.hide_render
              for corner in obj.bound_box]
    distance = 10.0
    if points:
        low = Vector(tuple(min(p[i] for p in points) for i in range(3)))
        high = Vector(tuple(max(p[i] for p in points) for i in range(3)))
        distance = (initial.translation - (low + high) / 2).dot(back)
    distance = max(observer.data.clip_start * 10, min(distance, observer.data.clip_end / 2))
    ortho = observer.data.ortho_scale
    revision, sequence, previous = -1, 0, None
    frame, meshes, digest, handled = None, None, None, None
    while True:
        try:
            command = json.loads((output / "command.json").read_text(encoding="utf-8"))
        except (OSError, ValueError):
            time.sleep(0.1)
            continue
        request = command.get("pick")
        if (command.get("active") and frame and frame["frozen"] and request
                and request != handled and command["revision"] == revision
                and all(request.get(key) == frame[key] for key in ("sessionId", "revision", "sequence"))
                and request.get("sha256") == digest):
            receipt = dict(request)
            receipt.update(meshes.pick(request["point"], request.get("rectangle")))
            atomic_json(output / "pick.json", receipt)
            handled = request
        if not command.get("active") or command["revision"] == revision:
            time.sleep(0.1)
            continue
        view = command["camera"]
        yaw = Quaternion(up, math.radians(view["yaw"]))
        pitch = Quaternion(yaw @ right, math.radians(view["pitch"]))
        rotation = pitch @ yaw
        scale = math.exp(view["zoom"])
        pivot = initial.translation - back * distance
        pivot += right * view["panX"] * distance + up * view["panY"] * distance
        position = pivot + (rotation @ back) * distance * scale
        observer.matrix_world = Matrix.LocRotScale(position, rotation @ basis, None)
        if observer.data.type == "ORTHO":
            observer.data.ortho_scale = ortho * scale
        scene.render.resolution_x, scene.render.resolution_y = command["width"], command["height"]
        bpy.context.view_layer.update()
        sequence += 1
        scene.render.filepath = str(output / f"frame-{sequence}.png")
        identity = {key: command[key] for key in ("camera", "width", "height")}
        if identity == previous:
            # Freeze/unfreeze acknowledges the existing pixels, including PNG metadata.
            shutil.copyfile(output / f"frame-{sequence - 1}.png", scene.render.filepath)
        else:
            bpy.ops.render.render(write_still=True)
        projection = observer.calc_matrix_camera(bpy.context.evaluated_depsgraph_get(),
            x=command["width"], y=command["height"],
            scale_x=scene.render.pixel_aspect_x, scale_y=scene.render.pixel_aspect_y)
        matrix = observer.matrix_world
        meshes = FrozenMeshes(scene, observer, projection)
        digest = hashlib.sha256(Path(scene.render.filepath).read_bytes()).hexdigest()
        frame = {"sessionId": session, "sequence": sequence,
                 "revision": command["revision"], "frozen": command["frozen"],
                 "width": command["width"], "height": command["height"],
                 "engine": bpy.app.version_string,
                 "picking": meshes.metadata(),
                 "camera": {"transform": [[matrix[row][col] for row in range(3)] for col in range(4)],
                            "projection": [[projection[row][col] for row in range(4)] for col in range(4)],
                            "near": observer.data.clip_start, "far": observer.data.clip_end,
                            "mode": 1 if observer.data.type == "ORTHO" else 0}}
        atomic_json(output / "frame.json", frame)
        revision = command["revision"]
        previous = identity
        # Bound disk usage; the host consumes frame bytes before acknowledging a new view.
        for stale in output.glob("frame-*.png"):
            if stale.name != f"frame-{sequence}.png":
                stale.unlink(missing_ok=True)


try:
    run()
except Exception as error:
    destination, session = sys.argv[-4], sys.argv[-1]
    atomic_json(Path(destination) / "frame.json", {"sessionId": session, "error": str(error)})
    raise

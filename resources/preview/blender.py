import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


def prepare(source, root, width, height):
    root = Path(root).resolve()
    bpy.ops.wm.open_mainfile(filepath=source, load_ui=False, use_scripts=False)
    # A frozen preview must never silently render resources from a mutable workspace.
    for dependency in bpy.utils.blend_paths(absolute=True, packed=False):
        path = Path(dependency).resolve()
        if not path.is_relative_to(root) or not path.is_file():
            raise RuntimeError("BLENDER_PREVIEW_DEPENDENCY_NOT_FROZEN: " + dependency)
    scene = bpy.context.scene
    camera_source = "frozen"
    if scene.camera is None:
        points = [obj.matrix_world @ Vector(corner)
                  for obj in scene.objects if obj.type == "MESH" and not obj.hide_render
                  for corner in obj.bound_box]
        if not points:
            raise RuntimeError("BLENDER_PREVIEW_NO_CAMERA_OR_MESH")
        low = Vector(tuple(min(p[i] for p in points) for i in range(3)))
        high = Vector(tuple(max(p[i] for p in points) for i in range(3)))
        center, radius = (low + high) / 2, max((high - low).length / 2, 0.1)
        data = bpy.data.cameras.new("BeaverPreviewCamera")
        camera = bpy.data.objects.new("BeaverPreviewCamera", data)
        scene.collection.objects.link(camera)
        data.sensor_fit = "HORIZONTAL"
        half_x = math.atan(data.sensor_width / (2 * data.lens))
        half_y = math.atan(math.tan(half_x) * int(height) / int(width))
        distance = radius / math.sin(min(half_x, half_y)) * 1.1
        camera.location = center + Vector((1, -1, 0.75)).normalized() * distance
        camera.rotation_euler = (center - camera.location).to_track_quat("-Z", "Y").to_euler()
        data.clip_start, data.clip_end = max(radius / 1000, 0.001), radius * 20
        scene.camera, camera_source = camera, "automatic-bounds-v1"
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x, scene.render.resolution_y = int(width), int(height)
    scene.render.resolution_percentage = 100
    scene.render.use_border = False
    scene.render.use_crop_to_border = False
    scene.render.film_transparent = False
    scene.render.use_compositing = False
    scene.render.use_sequencer = False
    scene.render.use_multiview = False
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "MATERIAL"
    scene.display.shading.background_type = "WORLD"
    return scene, camera_source


def render():
    source, root, output, width, height = sys.argv[sys.argv.index("--") + 1:]
    output = Path(output)
    scene, camera_source = prepare(source, root, width, height)
    scene.render.filepath = str(output / "blender.png")
    bpy.ops.render.render(write_still=True)
    state = {"complete": True, "renderer": "BLENDER_WORKBENCH",
             "cameraSource": camera_source, "camera": scene.camera.name,
             "cameraMatrix": [list(row) for row in scene.camera.matrix_world],
             "frame": scene.frame_current, "autoexec": False}
    (output / "blender.json").write_text(json.dumps(state), encoding="utf-8")


if __name__ == "__main__":
    render()

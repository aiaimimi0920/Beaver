"""Read stored Blender dependencies without saving or enabling file scripts."""

import json
import os

import bpy


def main():
    with open(os.environ["BEAVER_CONTEXT"], encoding="utf-8") as stream:
        context = json.load(stream)
    paths = set()
    for relative in context["paths"]:
        source = os.path.join(context["workspace"], *relative.split("/"))
        bpy.ops.wm.open_mainfile(filepath=source, load_ui=False, use_scripts=False)
        paths.update(bpy.utils.blend_paths(absolute=True, packed=False, local=False))
    print("BEAVER_RESULT=" + json.dumps({
        "scannerVersion": 1,
        "hostVersion": bpy.app.version_string,
        "paths": sorted(os.path.normpath(path) for path in paths if path),
    }))


main()

"""Export one named, unmodified Blender base mesh to a new private QA input."""
import argparse
import json
from pathlib import Path
import sys

import bpy


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("object")
    parser.add_argument("output", type=Path)
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    obj = bpy.data.objects.get(args.object)
    if obj is None or obj.type != "MESH":
        parser.error("named mesh object not found")
    mesh = obj.data
    mesh.calc_loop_triangles()
    document = {
        "vertices": [list(v.co) for v in mesh.vertices],
        "polygons": [list(p.vertices) for p in mesh.polygons],
        "triangles": [list(t.vertices) for t in mesh.loop_triangles],
    }
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(document, output, separators=(",", ":"), allow_nan=False)


if __name__ == "__main__":
    main()

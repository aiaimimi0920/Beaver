"""Replay a specific static Faye review mesh, never a production asset."""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tempfile
from pathlib import Path


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def validate_paths(source, target, checkout, expected_hash):
    source, target, checkout = source.resolve(), target.resolve(), checkout.resolve()
    if source.suffix.lower() != ".blend" or target.suffix.lower() != ".blend":
        raise ValueError("source and output must have .blend extensions")
    if target.is_relative_to(checkout) or target.exists() or not target.parent.is_dir():
        raise ValueError("output must be a new .blend in an existing directory outside the checkout")
    if not source.is_file() or sha256_file(source) != expected_hash:
        raise ValueError("source is not the authorized, exact V170 static review file")
    return source, target


def geometry(mesh):
    mesh.calc_loop_triangles()
    return [
        [list(v.co) for v in mesh.vertices],
        [list(p.vertices) for p in mesh.polygons],
        [list(t.vertices) for t in mesh.loop_triangles],
    ]


def replay_rows(positions, faces, triangles, recipe):
    station, i = recipe["station"], recipe["carrier_triangle_index"]
    carrier = recipe["carrier_triangle"]
    if list(triangles[i]) != carrier or station not in carrier:
        raise ValueError("carrier topology differs from the recipe")
    weighted = dict(zip(carrier, recipe["weights"]))
    base = positions[station]
    output = list(positions)
    output[station] = [
        value + sum(weighted[i] * (positions[i][axis] - value) for i in carrier if i != station)
        for axis, value in enumerate(base)
    ]
    removed = set(recipe["removed_authored_faces"])
    kept = [i for i in range(len(faces)) if i not in removed]
    return (output, [faces[i] for i in kept] + recipe["replacement_faces"],
            kept + recipe["replacement_donors"])


def verify_geometry(mesh, recipe):
    records = geometry(mesh)
    counts = dict(zip(("vertices", "author_faces", "native_triangles"), map(len, records)))
    counts["author_edges"] = len(mesh.edges)
    payload = json.dumps(records, separators=(",", ":"), allow_nan=False).encode("ascii")
    if counts != recipe["expected_counts"] or hashlib.sha256(payload).hexdigest() != recipe["geometry_sha256"]:
        raise ValueError("evaluated topology/coordinates do not match the reviewed V204 fingerprint")
    return counts


def write_new(staged, output):
    # Stage on the destination filesystem; a failed copy must not reserve output.
    with tempfile.TemporaryDirectory(prefix=".faye-v204-", dir=output.parent) as directory:
        pending = Path(directory) / "verified.blend"
        with staged.open("rb") as src, pending.open("xb") as dst:
            shutil.copyfileobj(src, dst)
        if sha256_file(staged) != sha256_file(pending):
            raise OSError("staged V204 output does not match the verified file")
        try:
            os.link(pending, output)
        except FileExistsError:
            raise
        except OSError as exc:
            raise OSError("cannot publish V204 without overwriting; use a directory "
                          "on a filesystem that supports hard links") from exc


def static_object(bpy):
    objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if len(objects) != 1:
        raise ValueError("expected precisely one static review mesh object")
    obj = objects[0]
    mesh = obj.data
    if (mesh.uv_layers or mesh.shape_keys or mesh.has_custom_normals
            or obj.vertex_groups or obj.modifiers or mesh.materials):
        raise ValueError("input is not the unrigged, untextured static review mesh")
    return obj


def apply_blender(bpy, source, target, recipe):
    if list(bpy.app.version) != recipe["blender"]:
        raise ValueError("the review fingerprint requires Blender 4.3.2")
    bpy.ops.wm.open_mainfile(filepath=str(source), use_scripts=False)
    obj = static_object(bpy)
    src = obj.data
    positions, faces, owners = replay_rows(*geometry(src), recipe)
    mesh = bpy.data.meshes.new("Faye V204 static topology review")
    mesh.from_pydata(positions, [], faces)
    for p, donor in zip(mesh.polygons, owners):
        p.use_smooth = src.polygons[donor].use_smooth
        p.material_index = src.polygons[donor].material_index
    old_flags = {tuple(sorted(e.vertices)): (e.use_edge_sharp, e.use_seam) for e in src.edges}
    transferred = set()
    for edge in mesh.edges:
        key = tuple(sorted(edge.vertices))
        edge.use_edge_sharp, edge.use_seam = old_flags.get(key, (False, False))
        transferred.add(key)
    if any(any(flags) and key not in transferred for key, flags in old_flags.items()):
        raise ValueError("replay would remove a tagged sharp edge or seam")
    mesh.update()
    if mesh.validate():
        raise ValueError("Blender corrected invalid geometry")
    counts = verify_geometry(mesh, recipe)
    obj.data = mesh
    bpy.context.scene["ReviewOnly"] = "Faye V204 static topology only; not for runtime or production."
    with tempfile.TemporaryDirectory(prefix="faye-v204-") as work:
        staged = Path(work) / "verified.blend"
        bpy.ops.wm.save_as_mainfile(filepath=str(staged), copy=True)
        bpy.ops.wm.open_mainfile(filepath=str(staged), use_scripts=False)
        verify_geometry(static_object(bpy).data, recipe)
        write_new(staged, target)
    print(json.dumps({"review": "V204-static-only", "geometry": counts}, sort_keys=True))


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("v170", type=Path)
    parser.add_argument("new_v204", type=Path)
    args = parser.parse_args(argv)
    script = Path(__file__).resolve()
    recipe = json.loads(script.with_name("v204_recipe.json").read_text(encoding="utf-8"))
    source, target = validate_paths(args.v170, args.new_v204, script.parents[3], recipe["input_sha256"])
    import bpy
    apply_blender(bpy, source, target, recipe)


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])

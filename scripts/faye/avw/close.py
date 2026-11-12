import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from paths import candidate_root, validation_root
import bpy, numpy as np, math
from pathlib import Path
from mathutils import Vector

V = Path(str(validation_root()))
R = V / str(candidate_root())
g = np.load(V / "apk-coverage-correction-20261112/new.npz")
P = g["positions"]
F = g["faces"]
a = np.load(R / "full-fields.npz")
F = np.load(R / "full-geometry.npz")["faces"]
P = (a["POSITION"] - np.array([0, 1.5581640002441406, 0.0863528327703476])) / 0.000708
D = np.stack([a[f"morph{k}.POSITION"] for k in range(7)]) / 0.000708
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete(use_global=False)
sc = bpy.context.scene
sc.render.engine = "CYCLES"
sc.cycles.samples = 8
sc.cycles.seed = 0
sc.world.use_nodes = True
sc.world.node_tree.nodes["Background"].inputs[1].default_value = 0.12
sc.render.resolution_x = 800
sc.render.resolution_y = 700
sc.render.resolution_percentage = 100
cd = bpy.data.cameras.new("cam")
cam = bpy.data.objects.new("cam", cd)
bpy.context.collection.objects.link(cam)
sc.camera = cam
cd.type = "ORTHO"
cd.ortho_scale = 75
ld = bpy.data.lights.new("fixed_key", "AREA")
lo = bpy.data.objects.new("fixed_key", ld)
bpy.context.collection.objects.link(lo)
lo.location = (-90, 80, 140)
ld.energy = 180000
ld.shape = "DISK"
ld.size = 80
lo.rotation_euler = (
    (Vector((0, -15, 0)) - lo.location).to_track_quat("-Z", "Y").to_euler()
)
mat = bpy.data.materials.new("gray")
mat.diffuse_color = (0.5, 0.5, 0.5, 1)
for state, w in {"neutral": [0] * 7}.items():
    p = P + np.einsum("m,mvc->vc", w, D)
    for angle in [0, 45]:
        center = Vector([26.064581227571285, -26.42787082795393, -16.640859542192356])
        theta = math.radians(angle)
        cam.location = center + Vector(
            (180 * math.sin(theta), 0, 180 * math.cos(theta))
        )
        cam.rotation_euler = (0, theta, 0)
        for shading in ["smooth"]:
            m = bpy.data.meshes.new("diagnostic")
            m.from_pydata(p.tolist(), [], F.tolist())
            m.materials.append(mat)
            m.update()
            for poly in m.polygons:
                poly.use_smooth = True
            normals = a["NORMAL"].copy() + sum(
                (w[k] * a[f"morph{k}.NORMAL"] for k in range(7))
            )
            normals /= np.maximum(np.linalg.norm(normals, axis=1)[:, None], 1e-30)
            m.normals_split_custom_set_from_vertices(normals.tolist())
            ob = bpy.data.objects.new("diagnostic", m)
            bpy.context.collection.objects.link(ob)
            overlay = []
            edges = __import__("json").loads((R / "ACTUAL-EDGES.json").read_text())
            for edge in edges:
                cr = bpy.data.curves.new("actual-edge", "CURVE")
                cr.dimensions = "3D"
                cr.bevel_depth = 0.075
                sp = cr.splines.new("POLY")
                sp.points.add(1)
                for j, rid in enumerate(edge):
                    sp.points[j].co = (*list(p[rid] + np.array([0, 0, 0.025])), 1)
                line = bpy.data.objects.new("actual-edge", cr)
                bpy.context.collection.objects.link(line)
                mt = bpy.data.materials.new("edge")
                mt.diffuse_color = (0.02, 0.02, 0.02, 1)
                cr.materials.append(mt)
                overlay.append(line)
            sc.render.filepath = str(
                V / str(candidate_root()) / f"close-{state}-{angle}-{shading}.png"
            )
            bpy.ops.render.render(write_still=True)
            bpy.data.objects.remove(ob, do_unlink=True)
            for line in overlay:
                bpy.data.objects.remove(line, do_unlink=True)
print("Requested diagnostic renders complete; visual acceptance not implied")

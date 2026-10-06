"""Specification-authored asymmetric pinna; no reference asset access."""

from face import *


def outline(t):
    stations = C["ear_outline_stations"]
    u = t % (2 * pi) / (2 * pi) * len(stations)
    i = int(u)
    f = u - i
    a, b, c, d = [stations[k % len(stations)] for k in (i - 1, i, i + 1, i + 2)]
    return tuple(
        (
            0.5
            * (
                2 * b[k]
                + (-a[k] + c[k]) * f
                + (2 * a[k] - 5 * b[k] + 4 * c[k] - d[k]) * f * f
                + (-a[k] + 3 * b[k] - 3 * c[k] + d[k]) * f * f * f
            )
            for k in range(3)
        )
    )


def build_ears():
    import bmesh

    for side in [-1, 1]:
        vs = []
        fs = []
        n = int(C["ear_ring_samples"])
        center = C["ear_concha_center"]
        bands = C["ear_relief_bands"]
        for r, relief in bands:
            for k in range(n):
                a = 2 * pi * k / n
                edge = outline(a)
                x, y, z = [center[d] * (1 - r) + edge[d] * r for d in range(3)]
                ridge = 1.0 if r != 0.52 else max(0.0, min(1.0, 0.5 + 0.7 * sin(a)))
                z += relief * ridge
                root = max(max(0.0, -cos(a)) ** 2, 0.75 * max(0.0, -sin(a)) ** 4) * r**3
                actual_y = Y0 + H * y
                root_x = width(actual_y) / H - C["ear_root_overlap"]
                root_z = (depth(width(actual_y), actual_y) - Z0) / H - 0.003
                x = x * (1 - root) + root_x * root
                z = z * (1 - root) + root_z * root
                vs.append(coord(side * H * x, actual_y, Z0 + H * z))
        for j in range(len(bands) - 1):
            for k in range(n):
                q = (k + 1) % n
                fs.append((j * n + k, (j + 1) * n + k, (j + 1) * n + q, j * n + q))
        fs.append(tuple(range(n)))
        last = (len(bands) - 1) * n
        front_count = len(vs)
        front_faces = list(fs)
        vs.extend(((x, negz + H * C["ear_back_thickness"], y) for x, negz, y in list(vs)))
        fs.extend((tuple((front_count + i for i in reversed(f))) for f in front_faces))
        for k in range(n):
            q = (k + 1) % n
            fs.append((last + k, front_count + last + k, front_count + last + q, last + q))
        obj = mesh("Ear asymmetric pinna " + str(side), vs, fs, SKIN)
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
        bm.normal_update()

        def signed_volume():
            origin = sum((v.co for v in bm.verts), Vector()) / len(bm.verts)
            return sum(
                (
                    (t[0].vert.co - origin).dot(
                        (t[1].vert.co - origin).cross(t[2].vert.co - origin)
                    )
                    / 6
                    for t in bm.calc_loop_triangles()
                )
            )

        before = signed_volume()
        if before < 0:
            for f in bm.faces:
                f.normal_flip()
            bm.normal_update()
        volume = signed_volume()
        print("PINNA_VOLUME", side, before, volume, bm.calc_volume(signed=True))
        assert all((f.calc_area() > 1e-12 for f in bm.faces)), "Ear has zero area faces"
        assert all((e.is_manifold for e in bm.edges)), "Ear shell must be closed"
        assert volume > 0, f"Nonpositive ear volume: {side} {volume}"
        bm.to_mesh(obj.data)
        bm.free()
        obj["ear_detail"] = True
        obj["closed_pinna_volume"] = volume
        obj["ear_front_vertex_count"] = front_count
        obj["ear_ring_samples"] = n
        obj["ear_band_count"] = len(bands)
        detail = obj.data.uv_layers.new(name="DetailUV")
        xs = [abs(v.co.x) / H for v in obj.data.vertices]
        ys = [(v.co.z - Y0) / H for v in obj.data.vertices]
        for loop in obj.data.loops:
            i = loop.vertex_index
            u = (xs[i] - min(xs)) / (max(xs) - min(xs))
            w = (ys[i] - min(ys)) / (max(ys) - min(ys))
            detail.data[loop.index].uv = (0.02 + 0.96 * u, 0.02 + 0.96 * w)


def fit_ear_roots(shell):
    from mathutils.bvhtree import BVHTree

    bpy.context.view_layer.update()
    evaluated = shell.evaluated_get(bpy.context.evaluated_depsgraph_get())
    data = evaluated.to_mesh()
    try:
        tree = BVHTree.FromPolygons(
            [v.co.copy() for v in data.vertices],
            [list(p.vertices) for p in data.polygons],
            all_triangles=False,
        )
        reports = []
        for obj in PARTS:
            if not obj.get("ear_detail"):
                continue
            n = obj["ear_ring_samples"]
            bands = obj["ear_band_count"]
            front = obj["ear_front_vertex_count"]
            last = (bands - 1) * n
            moves = []
            for k in range(n):
                angle = 360 * k / n
                if not 120 <= angle <= 270:
                    continue
                original = obj.data.vertices[last + k].co.copy()
                point, normal, _, distance = tree.find_nearest(original)
                assert point is not None, "Missing ear-root skin sample"
                delta = point - normal * (H * C["ear_root_clearance"]) - original
                assert (
                    delta.length < H * C["ear_root_max_fit"]
                ), "Ear-root fit exceeds structural tolerance"
                for j in range(bands):
                    weight = (j / (bands - 1)) ** 3
                    for offset in [0, front]:
                        obj.data.vertices[offset + j * n + k].co += delta * weight
                moves.append(delta.length)
            obj.data.update()
            assert all((math.isfinite(c) for v in obj.data.vertices for c in v.co))
            assert all((p.area > 1e-12 for p in obj.data.polygons)), "Degenerate fitted ear"
            reports.append(
                {
                    "name": obj.name,
                    "fitted_boundary_vertices": len(moves),
                    "max_move_m": max(moves, default=0),
                    "margin_m": H * C["ear_root_clearance"],
                }
            )
        return reports
    finally:
        evaluated.to_mesh_clear()

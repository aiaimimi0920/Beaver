"""Two procedural source families and an index-permuting native API stand-in."""

from copy import deepcopy
import hashlib
from math import sin
from shared_capture_contract import KEYS, POLICY, freeze_source


def layout(rows, cols, bend):
    points = [[x * .008, y * .01, bend * sin(x * .4) * sin(y * .3)]
              for y in range(rows) for x in range(cols)]
    vertices = [{"co": p, "groups": [["Continuous facial shell", 1.0]],
                 "select": False, "hide": False} for p in points]
    faces, corners, uv = [], [], {"UVMap": [], "UV2": [], "DetailUV": []}
    for y in range(rows - 1):
        for x in range(cols - 1):
            ids = [y * cols + x, y * cols + x + 1, (y + 1) * cols + x + 1, (y + 1) * cols + x]
            loops = list(range(len(corners), len(corners) + 4))
            faces.append({"vertices": ids, "loops": loops, "material": 0,
                          "smooth": True, "select": False, "hide": False})
            corners.extend({"vertex": i} for i in ids)
            for layer, values in uv.items():
                values.extend({"uv": [points[i][0] + x * .1, points[i][1] + len(layer) * .01],
                               "pin": bool(x % 2), "select": False, "select_edge": False} for i in ids)
    edges = sorted({tuple(sorted((a, b))) for f in faces for a, b in
                    zip(f["vertices"], f["vertices"][1:] + f["vertices"][:1])})
    mesh = {"vertices": vertices, "faces": faces, "corners": corners,
            "edges": [{"vertices": list(e), "seam": False, "sharp": False,
                       "select": False, "hide": False} for e in edges],
            "object_matrix_world": [[int(i == j) for j in range(4)] for i in range(4)],
            "materials": ["Face"], "uv_layers": uv, "attributes": {
                "NormalRegion": {"domain": "FACE", "type": "INT", "values": [7] * len(faces)},
                "FacialCreaseGroup": {"domain": "POINT", "type": "INT", "values": [2] * len(points)},
                "MouthOpenDelta": {"domain": "POINT", "type": "FLOAT_VECTOR",
                                   "values": [[0., 0., i * .00001] for i in range(len(points))]},
                "SourceAuthorSharp": {"domain": "EDGE", "type": "BOOLEAN", "values": [False] * len(edges)},
                "crease_edge": {"domain": "EDGE", "type": "FLOAT", "values": [0.] * len(edges)}}}
    bind_corner_edges(mesh)
    owner = {"schema_version": 1, "role": "authored_continuous_facial_shell",
             "validated": True, "issues": [], "group_name": "Continuous facial shell",
             "object_name": "Generated shell", "skin_regions": [7], "other_members": []}
    recipe = {"policy": POLICY, "source_sha256": hashlib.sha256(str((rows, cols, bend)).encode()).hexdigest()}
    return mesh, owner, recipe


def triangulate(source):
    """Simulate allowed vertex/face storage permutations; never geometry matching."""
    mesh = deepcopy(source)
    point_ids = list(reversed(range(len(source["vertices"]))))
    inverse = {original: runtime for runtime, original in enumerate(point_ids)}
    mesh["vertices"] = [deepcopy(source["vertices"][i]) for i in point_ids]
    mesh["faces"], mesh["corners"] = [], []
    face_ids, corner_ids = [], []
    for owner, face in reversed(list(enumerate(source["faces"]))):
        for local in ((0, 1, 2), (0, 2, 3)):
            ids = [inverse[face["vertices"][i]] for i in local]
            loops = list(range(len(corner_ids), len(corner_ids) + 3))
            mesh["faces"].append({**face, "vertices": ids, "loops": loops})
            mesh["corners"].extend({"vertex": i} for i in ids)
            face_ids.append(owner)
            corner_ids.extend(face["loops"][i] for i in local)
    edges = sorted({tuple(sorted((a, b))) for f in mesh["faces"] for a, b in
                    zip(f["vertices"], f["vertices"][1:] + f["vertices"][:1])})
    original_edges = {tuple(sorted(e["vertices"])): i for i, e in enumerate(source["edges"])}
    edge_ids = [original_edges.get(tuple(sorted(point_ids[i] for i in edge))) for edge in edges]
    mesh["edges"] = [{"vertices": list(e), "seam": False, "sharp": False,
                      "select": False, "hide": False} for e in edges]
    maps = {"POINT": point_ids, "FACE": face_ids, "CORNER": corner_ids, "EDGE": edge_ids}
    for name, attr in source["attributes"].items():
        mesh["attributes"][name]["values"] = [deepcopy(attr["values"][i]) if i is not None else (False if attr["type"] == "BOOLEAN" else 0.)
                                               for i in maps[attr["domain"]]]
    mesh["uv_layers"] = {name: [deepcopy(rows[i]) for i in corner_ids]
                         for name, rows in source["uv_layers"].items()}
    bind_corner_edges(mesh)
    shapes = {"Basis": [v["co"][:] for v in mesh["vertices"]]}
    for key, name in enumerate(KEYS):
        shapes[name] = [[p[0], p[1] + .0001 * (key + 1) * sin(i + 1), p[2]]
                        for i, p in enumerate(shapes["Basis"])]
    return mesh, {k: v for k, v in maps.items() if k != "EDGE"}, shapes, [[0., 0., 1.]] * len(corner_ids)


def captured(rows=3, cols=4, bend=.002):
    source, owners, recipe = layout(rows, cols, bend)
    return freeze_source(source, owners, recipe), triangulate(source)


def bind_corner_edges(mesh):
    lookup = {frozenset(row["vertices"]): i for i, row in enumerate(mesh["edges"])}
    for face in mesh["faces"]:
        vertices = face["vertices"]
        for index, loop in enumerate(face["loops"]):
            pair = frozenset((vertices[index], vertices[(index + 1) % len(vertices)]))
            mesh["corners"][loop]["edge"] = lookup[pair]
    mesh["native_corner_normals"] = [[0., 0., 1.] for _ in mesh["corners"]]

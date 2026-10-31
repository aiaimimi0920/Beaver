"""Read the bounded, neutral, identity-node Face export for density review."""
import hashlib
from pathlib import Path
import sys

import numpy as np

_support = str(Path(__file__).resolve().parent.parent / "general_face")
sys.path.insert(0, _support)
try:
    from glb_affine import read_glb, require
    from glb_components import values
finally:
    sys.path.pop(0)


def read(path):
    raw = Path(path).read_bytes()
    doc, binary = read_glb(raw)
    meshes = [(i, m) for i, m in enumerate(doc["meshes"]) if m.get("name") == "Face"]
    require(len(meshes) == 1, "Expected exactly one Face mesh")
    mesh_id, mesh = meshes[0]
    nodes = [(i, n) for i, n in enumerate(doc["nodes"]) if n.get("mesh") == mesh_id]
    require(len(nodes) == 1, "Expected one Face instance")
    node_id, node = nodes[0]
    require(not any(node_id in n.get("children", []) for n in doc["nodes"]),
            "Parented Face transforms are outside this review contract")
    require("skin" not in node and not doc.get("animations"), "Expected unskinned neutral Face")
    identity = {
        "translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1],
        "matrix": [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
    }
    require(all(node.get(k, v) == v for k, v in identity.items()), "Nonidentity Face transform")
    require(all(x == 0 for x in node.get("weights", []) + mesh.get("weights", [])),
            "Expected neutral morph weights")
    require(len(mesh["primitives"]) == 1, "Expected single-surface Face")
    primitive = mesh["primitives"][0]
    require(primitive.get("mode", 4) == 4, "Expected triangle primitive")

    def array(index, kind, allowed):
        actual_kind, component, normalized, rows = values(doc, binary, index)
        require(actual_kind == kind and component in allowed and not normalized,
                "Unsupported density accessor")
        dtype = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[component]
        width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3}[kind]
        return np.frombuffer(b"".join(rows), dtype=dtype).reshape(-1, width).copy()

    positions = array(primitive["attributes"]["POSITION"], "VEC3", (5126,)).astype(float)
    uv = array(primitive["attributes"]["TEXCOORD_0"], "VEC2", (5126,)).astype(float)
    indices = array(primitive["indices"], "SCALAR", (5121, 5123, 5125)).ravel()
    require(len(indices) % 3 == 0 and len(indices) > 0, "Invalid triangle count")
    require(len(uv) == len(positions) and int(indices.max()) < len(positions), "Invalid vertex indices")
    require(np.isfinite(positions).all() and np.isfinite(uv).all(), "Nonfinite geometry")
    return {
        "v": positions, "f": indices.reshape(-1, 3).astype(int), "uv": uv,
        "sha256": hashlib.sha256(raw).hexdigest(),
        "morphs": mesh.get("extras", {}).get("targetNames", []),
    }

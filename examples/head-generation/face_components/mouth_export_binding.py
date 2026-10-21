"""Compare full exported positions and six targets to native vertices, including sparse GLB."""

from collections import Counter
import itertools
import json
import math
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "general_face"))
from glb_components import values


def validate_export(raw, report, names):
    if len(raw) < 28 or struct.unpack_from("<III", raw, 0) != (1179937895, 2, len(raw)):
        raise ValueError("Invalid GLB envelope")
    length, kind = struct.unpack_from("<II", raw, 12)
    if kind != 1313821514 or length + 28 > len(raw):
        raise ValueError("Invalid GLB JSON chunk")
    doc = json.loads(raw[20 : 20 + length])
    size, kind = struct.unpack_from("<II", raw, 20 + length)
    binary = raw[28 + length :]
    if kind != 5130562 or size != len(binary):
        raise ValueError("Invalid GLB binary chunk")
    face = next((m for m in doc.get("meshes", []) if m.get("name") == "Face"), None)
    if face is None or face.get("extras", {}).get("targetNames") != names:
        raise ValueError("Exported mouth targets differ")
    rows = report["mouth_motion"].get("native_vertex_binding")
    if not isinstance(rows, list) or not rows:
        raise ValueError("Missing full native vertex binding")
    cells = {}
    step = 1e-6
    for i, row in enumerate(rows):
        if len(row) != 21 or not all(
            type(v) in (int, float) and math.isfinite(v) for v in row
        ):
            raise ValueError("Invalid native vertex binding")
        # Blender Z-up -> glTF Y-up, applied equally to positions and deltas.
        transformed = []
        for j in range(0, 21, 3):
            transformed.extend((row[j], row[j + 2], -row[j + 1]))
        key = tuple(round(v / step) for v in transformed[:3])
        cells.setdefault(key, []).append((i, transformed))
    canonical = {}
    aliases = []
    for i, row in enumerate(rows):
        aliases.append(canonical.setdefault(tuple(row), i))
    exported_triangles = []
    seen = set()
    maxima = {name: 0.0 for name in names[3:]}

    def vectors(index):
        kind, component, normalized, data = values(doc, binary, index)
        if (kind, component, normalized) != ("VEC3", 5126, False):
            raise ValueError("Invalid exported vector accessor")
        result = [struct.unpack("<fff", r) for r in data]
        if any(not all(math.isfinite(v) for v in row) for row in result):
            raise ValueError("Nonfinite exported vector")
        return result

    for primitive in face["primitives"]:
        targets = primitive.get("targets", [])
        if len(targets) != 6:
            raise ValueError("Missing exported targets")
        positions = vectors(primitive["attributes"]["POSITION"])
        deltas = [vectors(t["POSITION"]) for t in targets]
        if any(len(d) != len(positions) for d in deltas):
            raise ValueError("Exported target count mismatch")
        triangles = check_exported_triangles(doc, binary, primitive, positions, deltas)
        mapping = []
        for index, position in enumerate(positions):
            observed = list(position)
            for delta in deltas:
                observed.extend(delta[index])
            key = tuple(round(v / step) for v in position)
            matched = []
            for shift in itertools.product((-1, 0, 1), repeat=3):
                for original, expected in cells.get(
                    tuple(a + b for a, b in zip(key, shift)), []
                ):
                    if max(abs(a - b) for a, b in zip(observed, expected)) <= 2e-7:
                        matched.append(original)
            if not matched:
                raise ValueError(
                    "Exported position or full morph vector differs from native geometry"
                )
            seen.update(matched)
            mapping.append(min(aliases[i] for i in matched))
            for name, delta in zip(names[3:], deltas[3:]):
                maxima[name] = max(
                    maxima[name], math.sqrt(sum(v * v for v in delta[index]))
                )
        exported_triangles.extend(tuple(mapping[i] for i in tri) for tri in triangles)
    native_triangles = report["mouth_motion"].get("native_triangles")
    if not isinstance(native_triangles, list) or not native_triangles:
        raise ValueError("Missing native triangle binding")
    if any(
        len(tri) != 3 or any(type(i) is not int or not 0 <= i < len(rows) for i in tri)
        for tri in native_triangles
    ):
        raise ValueError("Invalid native triangle binding")

    def oriented(tri):
        tri = tuple(tri)
        return min(tri, tri[1:] + tri[:1], tri[2:] + tri[:2])

    expected = Counter(
        oriented(tuple(aliases[i] for i in tri)) for tri in native_triangles
    )
    if Counter(oriented(tri) for tri in exported_triangles) != expected:
        raise ValueError("Exported oriented triangle topology differs from native mesh")
    if len(seen) != len(rows):
        raise ValueError("Native vertices missing from export")
    for name, value in maxima.items():
        if value <= 0 or not math.isclose(
            value,
            report["mouth_motion"]["max_displacement_m"][name],
            rel_tol=1e-4,
            abs_tol=2e-6,
        ):
            raise ValueError("Actual exported mouth magnitude mismatch")
    return maxima


def check_exported_triangles(doc, binary, primitive, positions, deltas):
    """Recheck actual exported triangles, not just native pre-export samples."""
    import numpy as np

    if primitive.get("mode", 4) != 4 or "indices" not in primitive:
        raise ValueError("Expected indexed exported triangles")
    kind, component, normalized, rows = values(doc, binary, primitive["indices"])
    formats = {5121: "<B", 5123: "<H", 5125: "<I"}
    if kind != "SCALAR" or component not in formats or normalized:
        raise ValueError("Invalid exported triangle indices")
    ids = [struct.unpack(formats[component], r)[0] for r in rows]
    if len(ids) % 3 or not ids or max(ids) >= len(positions):
        raise ValueError("Exported triangle index out of bounds")
    base = np.array(positions)
    ds = np.array(deltas)
    affected = np.any(np.abs(ds[3:]) > 1e-12, axis=(0, 2))
    all_triangles = np.array(ids).reshape(-1, 3)
    triangles = all_triangles[np.any(affected[all_triangles], axis=1)]
    if not len(triangles):
        return all_triangles.tolist()

    def normals(points):
        a, b, c = (points[triangles[:, i]] for i in range(3))
        return np.cross(b - a, c - a)

    neutral = normals(base)
    for weights in itertools.product((0.0, 0.5, 1.0), repeat=4):
        points = base + weights[0] * ds[0]
        for weight, delta in zip(weights[1:], ds[3:]):
            points = points + weight * delta
        normal = normals(points)
        if np.any(np.linalg.norm(normal, axis=1) * 0.5 <= 1e-12) or np.any(
            np.einsum("ij,ij->i", normal, neutral) <= 0
        ):
            raise ValueError(
                "Actual exported sampled mouth triangle folds or degenerates"
            )

    return all_triangles.tolist()

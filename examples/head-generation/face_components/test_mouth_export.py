"""Small synthetic GLB fixtures exercising full-vector and topology guards."""

import copy
import json
import struct
import unittest
from mouth_evidence import NAMES, validate_glb


def fixture(indices=(0, 1, 2), reverse=False):
    positions = [(0.0, 0.0, 0.0), (0.01, 0.0, 0.0), (0.0, 0.01, 0.0)]
    moves = [(0.0, 0.0, 0.0)] * 3 + [
        (0.001, 0.0, 0.0),
        (0.002, 0.0, 0.0),
        (0.003, 0.0, 0.0),
    ]
    raw = bytearray()
    views = []
    accessors = []

    def accessor(rows, kind, component, fmt):
        start = len(raw)
        for row in rows:
            raw.extend(struct.pack(fmt, *row))
        views.append({"buffer": 0, "byteOffset": start, "byteLength": len(raw) - start})
        accessors.append(
            {
                "bufferView": len(views) - 1,
                "componentType": component,
                "count": len(rows),
                "type": kind,
            }
        )
        raw.extend(b"\0" * (-len(raw) % 4))
        return len(accessors) - 1

    pos = accessor(positions, "VEC3", 5126, "<fff")
    targets = []
    for move in moves:
        delta = tuple(-v for v in move) if reverse else move
        targets.append({"POSITION": accessor([delta] * 3, "VEC3", 5126, "<fff")})
    idx = accessor([(i,) for i in indices], "SCALAR", 5123, "<H")
    doc = {
        "buffers": [{"byteLength": len(raw)}],
        "bufferViews": views,
        "accessors": accessors,
        "meshes": [
            {
                "name": "Face",
                "extras": {"targetNames": NAMES},
                "primitives": [
                    {
                        "attributes": {"POSITION": pos},
                        "targets": targets,
                        "indices": idx,
                    }
                ],
            }
        ],
    }
    body = json.dumps(doc).encode()
    body += b" " * (-len(body) % 4)
    glb = (
        struct.pack("<III", 1179937895, 2, 28 + len(body) + len(raw))
        + struct.pack("<II", len(body), 1313821514)
        + body
        + struct.pack("<II", len(raw), 5130562)
        + raw
    )
    rows = []
    for position in positions:
        row = []
        for xyz in [position, *moves]:
            row.extend((xyz[0], -xyz[2], xyz[1]))
        rows.append(row)
    report = {
        "mouth_motion": {
            "native_vertex_binding": rows,
            "native_triangles": [[0, 1, 2]],
            "max_displacement_m": dict(zip(NAMES[3:], [0.001, 0.002, 0.003])),
        }
    }
    return glb, report


class MouthExportTests(unittest.TestCase):
    def test_valid_full_binding(self):
        raw, report = fixture()
        self.assertEqual(set(validate_glb(raw, report)), set(NAMES[3:]))

    def test_reversed_vectors_rejected(self):
        raw, report = fixture(reverse=True)
        with self.assertRaisesRegex(ValueError, "full morph vector"):
            validate_glb(raw, report)

    def test_reversed_winding_rejected(self):
        raw, report = fixture((0, 2, 1))
        with self.assertRaisesRegex(ValueError, "topology"):
            validate_glb(raw, report)

    def test_degenerate_indices_rejected(self):
        raw, report = fixture((0, 0, 0))
        with self.assertRaisesRegex(ValueError, "degenerates"):
            validate_glb(raw, report)

    def test_missing_native_vertex_rejected(self):
        raw, report = fixture()
        report["mouth_motion"]["native_vertex_binding"].append([9.0] * 21)
        with self.assertRaisesRegex(ValueError, "missing from export"):
            validate_glb(raw, report)

    def test_native_target_corruption_rejected(self):
        raw, report = fixture()
        report["mouth_motion"]["native_vertex_binding"][1][-3] += 0.002
        with self.assertRaisesRegex(ValueError, "full morph vector"):
            validate_glb(raw, report)

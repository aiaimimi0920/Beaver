"""Asset-free contract tests; real model/native checks require private inputs."""

import json
from pathlib import Path
import struct
import tempfile
import unittest

import numpy as np

from bridge_common import Rejected, exact, exclusive_bytes, to_blender, to_gltf
from bridge_glb import JsonSpans, accessor, parse_glb
from position_bridge import patch_bytes, validate_positions


class BridgePrimitives(unittest.TestCase):
    def test_coordinate_transform_roundtrip(self):
        points = np.array([[1, 2, 3], [-4, 5, -6]], dtype=np.float32)
        self.assertTrue(
            exact(
                to_blender(points), np.array([[1, -3, 2], [-4, 6, 5]], dtype=np.float32)
            )
        )
        self.assertTrue(exact(to_gltf(to_blender(points)), points))

    def test_exact_requires_dtype_and_bit_pattern(self):
        value = np.array([0.0], dtype=np.float32)
        self.assertFalse(exact(value, value.astype(np.float64)))
        self.assertFalse(exact(value, np.array([-0.0], dtype=np.float32)))

    def test_exclusive_write_preserves_existing_file(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "output.bin"
            exclusive_bytes(path, b"original")
            with self.assertRaises(FileExistsError):
                exclusive_bytes(path, b"replacement")
            self.assertEqual(path.read_bytes(), b"original")

    def test_json_spans_preserve_nested_and_escaped_values(self):
        doc = {
            "label": 'mouth \\" [ ]',
            "data": [{"权重": 0.25}, None, True],
            "empty": {},
        }
        text = json.dumps(doc, ensure_ascii=False, indent=2)
        spans = JsonSpans(text).spans
        for path, expected in [
            ((), doc),
            (("label",), doc["label"]),
            (("data", 0, "权重"), 0.25),
            (("empty",), {}),
        ]:
            begin, end = spans[path]
            self.assertEqual(json.loads(text[begin:end]), expected)

    def test_glb_rejects_wrong_header_and_external_buffer(self):
        for data in [b"", b"not a model" * 3]:
            with self.assertRaises(Rejected):
                parse_glb(data)
        text = json.dumps({"buffers": [{"uri": "other.bin", "byteLength": 4}]}).encode()
        text += b" " * (-len(text) % 4)
        data = struct.pack("<III", 0x46546C67, 2, 28 + len(text) + 4)
        data += struct.pack("<II", len(text), 0x4E4F534A) + text
        data += struct.pack("<II", 4, 0x004E4942) + b"\0" * 4
        with self.assertRaisesRegex(Rejected, "embedded buffer"):
            parse_glb(data)

    def test_accessor_reads_strided_data_without_padding(self):
        binary = struct.pack("<8f", 1, 2, 3, 999, 4, 5, 6, 888)
        doc = {
            "bufferViews": [{"buffer": 0, "byteStride": 16}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 2, "type": "VEC3"}
            ],
        }
        np.testing.assert_array_equal(accessor(doc, binary, 0), [[1, 2, 3], [4, 5, 6]])

    def test_wrong_source_rejected_before_loading_private_baseline(self):
        with self.assertRaisesRegex(Rejected, "authoritative source"):
            patch_bytes(
                b"wrong model", np.zeros((8, 4, 3), dtype=np.float32), "/not/a/baseline"
            )

    def fixture(self):
        arrays = {
            "shape_" + str(i): np.zeros((4, 3), dtype=np.float32) for i in range(8)
        }
        manifest = {
            "rings": [
                {
                    "members": [0, 1],
                    "stations": [
                        {
                            "primary_source_vertex": 0,
                            "coincident_source_vertices": [0, 1],
                        }
                    ],
                }
            ]
        }
        return np.stack(list(arrays.values())), arrays, manifest

    def test_local_seam_preserving_position_change(self):
        positions, arrays, manifest = self.fixture()
        positions[2, :2, 0] = 0.00002
        _, changed, allowed = validate_positions(positions, {}, arrays, manifest)
        self.assertEqual(np.count_nonzero(changed), 2)
        np.testing.assert_array_equal(allowed, [True, True, False, False])

    def test_outside_mouth_and_seam_changes_rejected(self):
        for index, message in [(2, "outside allowed"), (0, "split-seam")]:
            positions, arrays, manifest = self.fixture()
            positions[0, index, 0] = 0.00002
            with self.assertRaisesRegex(Rejected, message):
                validate_positions(positions, {}, arrays, manifest)

    def test_nonfinite_wrong_shape_and_dtype_rejected(self):
        positions, arrays, manifest = self.fixture()
        bad = positions.copy()
        bad[0, 0, 0] = np.nan
        for value in [bad, positions.astype(np.float64), positions[:7]]:
            with self.assertRaises(Rejected):
                validate_positions(value, {}, arrays, manifest)


if __name__ == "__main__":
    unittest.main()

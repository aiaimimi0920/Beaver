"""Reject corruption isolated to independently stored blink export streams."""

import struct
import unittest

from export_contract import validate
from glb_affine import read_glb, write_glb
from glb_components import values
from test_export_contract import SOURCE, export


def independent_blinks():
    doc, binary = read_glb(export(False, False))
    data = bytearray(binary)
    mesh = doc["meshes"][0]
    mesh["extras"]["targetNames"] = ["MouthOpen", "blink.L", "blink.R"]
    mesh["weights"] = doc["nodes"][0]["weights"] = [0, 0, 0]
    primitive = mesh["primitives"][0]
    original = primitive["targets"][0]
    for _ in range(2):
        target = {}
        for name, accessor in original.items():
            _, _, _, rows = values(doc, data, accessor)
            data.extend(b"\0" * (-len(data) % 4))
            offset = len(data)
            packed = b"".join(rows)
            data.extend(packed)
            view = len(doc["bufferViews"])
            doc["bufferViews"].append(
                {"buffer": 0, "byteOffset": offset, "byteLength": len(packed)}
            )
            target[name] = len(doc["accessors"])
            doc["accessors"].append(
                {
                    "bufferView": view,
                    "componentType": 5126,
                    "type": "VEC3",
                    "count": len(rows),
                }
            )
        primitive["targets"].append(target)
    return doc, data


class BlinkExportCorruptionTests(unittest.TestCase):
    def test_independent_stream_fixture_passes(self):
        doc, data = independent_blinks()
        targets = doc["meshes"][0]["primitives"][0]["targets"]
        self.assertEqual(len({a for target in targets for a in target.values()}), 6)
        self.assertEqual(
            validate(write_glb(doc, data), SOURCE, with_blinks=True)["morph_names"],
            ["MouthOpen", "blink.L", "blink.R"],
        )

    def test_nonfinite_in_each_independent_blink_stream(self):
        for target in (1, 2):
            for semantic in ("POSITION", "NORMAL"):
                for value in (float("nan"), float("inf"), -float("inf")):
                    with self.subTest(target=target, semantic=semantic, value=value):
                        doc, data = independent_blinks()
                        primitive = doc["meshes"][0]["primitives"][0]
                        accessor = doc["accessors"][
                            primitive["targets"][target][semantic]
                        ]
                        view = doc["bufferViews"][accessor["bufferView"]]
                        struct.pack_into("<f", data, view["byteOffset"], value)
                        with self.assertRaisesRegex(ValueError, "Nonfinite"):
                            validate(write_glb(doc, data), SOURCE, with_blinks=True)

    def test_mismatched_names(self):
        for names in (
            ["MouthOpen", "blink.R", "blink.L"],
            ["MouthOpen", "blink.L", "blink.L"],
            ["MouthOpen", "blink.L"],
        ):
            with self.subTest(names=names):
                doc, data = independent_blinks()
                doc["meshes"][0]["extras"]["targetNames"] = names
                with self.assertRaisesRegex(ValueError, "morph target names"):
                    validate(write_glb(doc, data), SOURCE, with_blinks=True)

    def test_mismatched_or_nonneutral_weights(self):
        for owner in ("nodes", "meshes"):
            for weights in ([0], [0, 0], [0, 0, 0, 0], [0, 0, 0.25]):
                with self.subTest(owner=owner, weights=weights):
                    doc, data = independent_blinks()
                    doc[owner][0]["weights"] = weights
                    with self.assertRaisesRegex(ValueError, "Nonneutral default morph"):
                        validate(write_glb(doc, data), SOURCE, with_blinks=True)

    def test_index_equal_to_vertex_count_is_rejected(self):
        doc, data = independent_blinks()
        primitive = doc["meshes"][0]["primitives"][0]
        count = doc["accessors"][primitive["attributes"]["POSITION"]]["count"]
        accessor = doc["accessors"][primitive["indices"]]
        view = doc["bufferViews"][accessor["bufferView"]]
        struct.pack_into("<H", data, view["byteOffset"], count)
        with self.assertRaisesRegex(ValueError, "Triangle indices outside Face"):
            validate(write_glb(doc, data), SOURCE, with_blinks=True)

    def test_incomplete_triangle_is_rejected(self):
        doc, data = independent_blinks()
        primitive = doc["meshes"][0]["primitives"][0]
        doc["accessors"][primitive["indices"]]["count"] = 2
        with self.assertRaisesRegex(ValueError, "Triangle indices outside Face"):
            validate(write_glb(doc, data), SOURCE, with_blinks=True)


if __name__ == "__main__":
    unittest.main()

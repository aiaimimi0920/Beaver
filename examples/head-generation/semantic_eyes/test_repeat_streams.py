"""CPU fixtures: equal marginal streams must not hide changed vertex associations."""

import struct
import unittest
from repeat_streams import compare
from glb_affine import read_glb, write_glb
from glb_components import values
from test_glb_affine import fixture


def repack(order=(0, 1, 2), uv_order=None, winding=False, duplicate=False):
    doc, old = read_glb(fixture(sparse=False, strided=False))
    binary = bytearray()
    doc["bufferViews"] = []
    # Expand using the original descriptor before replacing storage.
    original_doc, old = read_glb(fixture(sparse=False, strided=False))
    for index, accessor in enumerate(doc["accessors"]):
        _, _, _, rows = values(original_doc, old, index)
        permutation = uv_order if index == 3 and uv_order is not None else order
        data = b"".join(rows[i] for i in permutation)
        if duplicate:
            data += rows[order[0]]
        offset = len(binary)
        binary.extend(data)
        doc["bufferViews"].append(
            {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
        )
        accessor["bufferView"] = index
        accessor["count"] = len(permutation) + int(duplicate)
    indices = (
        [order.index(i) for i in (0, 2, 1) if winding]
        if winding
        else [order.index(i) for i in (0, 1, 2)]
    )
    offset = len(binary)
    binary.extend(struct.pack("<HHH", *indices))
    doc["bufferViews"].append({"buffer": 0, "byteOffset": offset, "byteLength": 6})
    doc["accessors"].append(
        {
            "bufferView": len(doc["bufferViews"]) - 1,
            "componentType": 5123,
            "count": 3,
            "type": "SCALAR",
        }
    )
    doc["meshes"][0]["primitives"][0]["indices"] = len(doc["accessors"]) - 1
    return write_glb(doc, binary)


class RepeatStreamsTests(unittest.TestCase):
    def test_joint_permutation_and_storage_changes(self):
        self.assertTrue(
            compare(fixture(), repack((2, 0, 1)))["exact_permutation_equivalent"]
        )
        self.assertFalse(compare(fixture(), repack((2, 0, 1)))["raw_bytes_equal"])

    def test_uv_association_cannot_be_hidden_by_equal_marginals(self):
        self.assertFalse(
            compare(repack(), repack(uv_order=(1, 0, 2)))[
                "exact_permutation_equivalent"
            ]
        )

    def test_winding_and_unreferenced_duplicate_are_retained(self):
        for raw in (repack(winding=True),):
            self.assertFalse(compare(repack(), raw)["exact_permutation_equivalent"])

    def test_ambiguous_duplicate_identity_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "Duplicate joint"):
            compare(repack(duplicate=True), repack(duplicate=True))

    def test_unsupported_behavior_fails_closed(self):
        for key, value in (
            ("animations", [{"channels": []}]),
            ("extensions", {"Example": {"value": 1}}),
            ("skins", []),
            ("extras", {"x": 1}),
        ):
            doc, binary = read_glb(repack())
            doc[key] = value
            with self.assertRaisesRegex(ValueError, "Unsupported top-level"):
                compare(repack(), write_glb(doc, binary))

    def test_morph_change_and_material_change_are_retained(self):
        for morph in (True, False):
            doc, binary = read_glb(repack())
            if morph:
                binary = bytearray(binary)
                offset = doc["bufferViews"][doc["accessors"][4]["bufferView"]][
                    "byteOffset"
                ]
                struct.pack_into("<f", binary, offset, 0.001)
            else:
                doc["materials"] = [{"pbrMetallicRoughness": {"metallicFactor": 0.4}}]
                doc["meshes"][0]["primitives"][0]["material"] = 0
            self.assertFalse(
                compare(repack(), write_glb(doc, binary))[
                    "exact_permutation_equivalent"
                ]
            )

    def test_non_face_and_transform_rejected(self):
        for key, value in (("name", "Body"), ("translation", [0, 0, 0.01])):
            doc, binary = read_glb(repack())
            doc["nodes"][0][key] = value
            with self.assertRaises(ValueError):
                compare(repack(), write_glb(doc, binary))


if __name__ == "__main__":
    unittest.main()

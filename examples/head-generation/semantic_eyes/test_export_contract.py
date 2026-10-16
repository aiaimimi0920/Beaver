import io
import json
import struct
import unittest
from PIL import Image
from export_contract import validate
from glb_affine import read_glb, write_glb
from test_glb_affine import fixture


def png(color):
    out = io.BytesIO()
    Image.new("RGBA", (2, 2), color).save(out, format="PNG")
    return out.getvalue()


SOURCE = png((40, 120, 80, 255))


def export(sparse=True, strided=True, image=SOURCE):
    doc, raw = read_glb(fixture(sparse, strided))
    data = bytearray(raw)
    p = doc["meshes"][0]["primitives"][0]
    p["targets"][0].pop("TANGENT")
    data.extend(b"\0" * (-len(data) % 4))
    offset = len(data)
    data.extend(struct.pack("<HHH", 0, 1, 2))
    doc["bufferViews"].append({"buffer": 0, "byteOffset": offset, "byteLength": 6})
    p["indices"] = len(doc["accessors"])
    doc["accessors"].append(
        {
            "bufferView": len(doc["bufferViews"]) - 1,
            "componentType": 5123,
            "type": "SCALAR",
            "count": 3,
        }
    )
    data.extend(b"\0" * (-len(data) % 4))
    offset = len(data)
    data.extend(image)
    doc["bufferViews"].append(
        {"buffer": 0, "byteOffset": offset, "byteLength": len(image)}
    )
    doc["images"] = [
        {"bufferView": len(doc["bufferViews"]) - 1, "mimeType": "image/png"}
    ]
    doc["textures"] = [{"source": 0, "sampler": 0}]
    doc["samplers"] = [{}]
    doc["materials"] = [
        {"pbrMetallicRoughness": {"baseColorTexture": {"index": 0, "texCoord": 0}}}
    ]
    p["material"] = 0
    return write_glb(doc, data)


class ExportTests(unittest.TestCase):
    def test_native_packager_keeps_same_job_image_binding(self):
        import ast
        import tempfile
        from pathlib import Path
        from bundle import build_request, REQUIRED

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            request = build_request(
                {}, "assets/aster/general_face_eyes_export_test/", root
            )
        sources = [
            node.value
            for node in ast.walk(ast.parse(request["code"]))
            if isinstance(node, ast.Constant)
            and isinstance(node.value, str)
            and "def bake_base_atlas(" in node.value
        ]
        self.assertTrue(sources)
        for source in sources:
            self.assertIn("image.pack()", source)
            self.assertNotIn('image.filepath = "//" + role.lower()', source)

    def test_dense_sparse_interleaved(self):
        for sparse in (False, True):
            for strided in (False, True):
                self.assertEqual(
                    validate(export(sparse, strided), SOURCE)["triangles"], 1
                )

    def test_stale_source_atlas(self):
        with self.assertRaisesRegex(ValueError, "same-job atlas"):
            validate(export(image=png((120, 40, 80, 255))), SOURCE)

    def test_invalid_references(self):
        for path in [
            ("scene",),
            ("scenes", 0, "nodes", 0),
            ("nodes", 0, "mesh"),
            ("meshes", 0, "primitives", 0, "material"),
            ("textures", 0, "source"),
            ("textures", 0, "sampler"),
        ]:
            doc, data = read_glb(export())
            target = doc
            for part in path[:-1]:
                target = target[part]
            target[path[-1]] = 99
            with self.assertRaises(ValueError):
                validate(write_glb(doc, data), SOURCE)

    def test_morph_name_and_uv(self):
        doc, data = read_glb(export())
        doc["meshes"][0]["extras"]["targetNames"] = ["Wrong"]
        with self.assertRaises(ValueError):
            validate(write_glb(doc, data), SOURCE)
        doc, data = read_glb(export())
        doc["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["texCoord"] = 2
        with self.assertRaises(ValueError):
            validate(write_glb(doc, data), SOURCE)

    def test_nonfinite(self):
        doc, data = read_glb(export(False, False))
        data = bytearray(data)
        off = doc["bufferViews"][doc["accessors"][0]["bufferView"]]["byteOffset"]
        struct.pack_into("<f", data, off, float("nan"))
        with self.assertRaisesRegex(ValueError, "Nonfinite"):
            validate(write_glb(doc, data), SOURCE)

    def test_invalid_attribute_encoding(self):
        doc, data = read_glb(export(False, False))
        doc["accessors"][0]["componentType"] = 5123
        with self.assertRaisesRegex(ValueError, "encoding"):
            validate(write_glb(doc, data), SOURCE)

    def test_unknown_extensions_and_slots(self):
        for mutate in (
            lambda d: d.update(extensionsUsed=["Unknown"]),
            lambda d: d["meshes"][0].update(extensions={"Unknown": {}}),
            lambda d: d["materials"][0].update(normalTexture={"index": 999}),
            lambda d: d["bufferViews"][d["images"][0]["bufferView"]].update(
                byteStride=4
            ),
            lambda d: d["materials"][0]["pbrMetallicRoughness"][
                "baseColorTexture"
            ].update(extensions={"KHR_texture_transform": {}}),
        ):
            doc, data = read_glb(export())
            mutate(doc)
            with self.assertRaises(ValueError):
                validate(write_glb(doc, data), SOURCE)

    def test_png_decode_failure(self):
        with self.assertRaises(Exception):
            validate(export(image=b"not a png"), SOURCE)

    def test_external_image(self):
        doc, data = read_glb(export())
        doc["images"][0]["uri"] = "elsewhere.png"
        with self.assertRaises(ValueError):
            validate(write_glb(doc, data), SOURCE)


if __name__ == "__main__":
    unittest.main()

"""Narrow standalone Face GLB contract, separate from repeatability/beauty checks."""

import hashlib
import io
import math
import struct
import sys
from pathlib import Path

sys.path.append(str(Path(__file__).resolve().parents[1] / "general_face"))
from PIL import Image
from glb_affine import read_glb, require
from glb_components import values


def index(value, items, label):
    require(type(value) is int and 0 <= value < len(items), "Invalid " + label)
    return items[value]


def rgba(raw):
    with Image.open(io.BytesIO(raw)) as image:
        require(image.format == "PNG", "Expected PNG bytes")
        image.load()
        return image.size, image.convert("RGBA").tobytes()


def validate(raw, source_png):
    doc, binary = read_glb(raw)
    require(
        doc.get("asset", {}).get("version") == "2.0", "Unsupported glTF asset version"
    )
    require(
        not any(k in doc for k in ("animations", "skins", "cameras", "extensions")),
        "Unsupported behavior",
    )
    for key in ("nodes", "meshes", "scenes", "materials", "textures", "images"):
        require(len(doc.get(key, [])) == 1, "Expected singleton " + key)
    scene = index(doc.get("scene", 0), doc["scenes"], "scene")
    require(
        scene.get("nodes") == [0] and type(scene["nodes"][0]) is int,
        "Face must be the only reachable node",
    )
    node = doc["nodes"][0]
    require(
        node.get("name") == "Face"
        and type(node.get("mesh")) is int
        and node["mesh"] == 0,
        "Expected Face node",
    )
    require(
        not any(k in node for k in ("children", "skin", "extensions", "matrix")),
        "Unsupported node graph",
    )
    for key, default in [
        ("translation", [0, 0, 0]),
        ("rotation", [0, 0, 0, 1]),
        ("scale", [1, 1, 1]),
    ]:
        require(node.get(key, default) == default, "Nonidentity Face transform")
    mesh = doc["meshes"][0]
    require(not mesh.get("extensions"), "Unsupported mesh extension")
    require(
        mesh.get("name") == "Face" and len(mesh.get("primitives", [])) == 1,
        "Expected one Face primitive",
    )
    require(
        mesh.get("extras", {}).get("targetNames") == ["MouthOpen"],
        "Expected MouthOpen target name",
    )
    require(
        mesh.get("weights", [0]) == [0] and node.get("weights", [0]) == [0],
        "Nonneutral default morph",
    )
    primitive = mesh["primitives"][0]
    require(
        primitive.get("mode", 4) == 4 and not primitive.get("extensions"),
        "Expected ordinary triangles",
    )
    attrs = primitive.get("attributes", {})
    allowed = {
        "POSITION": 3,
        "NORMAL": 3,
        "TANGENT": 4,
        "TEXCOORD_0": 2,
        "TEXCOORD_1": 2,
        "COLOR_0": 4,
        "COLOR_1": 4,
        "TEXCOORD_2": 2,
    }
    require(
        set(attrs) <= set(allowed)
        and {"POSITION", "NORMAL", "TEXCOORD_0"} <= set(attrs),
        "Unsupported or missing Face attributes",
    )
    counts = []
    streams = []
    for name, accessor_id in attrs.items():
        kind, component, normalized, rows = values(doc, binary, accessor_id)
        require(kind == "VEC" + str(allowed[name]), "Wrong attribute width")
        if name in ("POSITION", "NORMAL", "TANGENT"):
            require(component == 5126 and not normalized, "Invalid geometry encoding")
        else:
            require(
                (component == 5126 and not normalized)
                or (component in (5121, 5123) and normalized),
                "Invalid UV/color encoding",
            )
        counts.append(len(rows))
        streams.append((component, rows))
    require(
        counts and counts[0] > 0 and len(set(counts)) == 1, "Attribute count mismatch"
    )
    targets = primitive.get("targets", [])
    require(
        len(targets) == 1 and set(targets[0]) == {"POSITION", "NORMAL"},
        "Expected position/normal MouthOpen",
    )
    for accessor_id in targets[0].values():
        kind, component, normalized, rows = values(doc, binary, accessor_id)
        require(
            component == 5126
            and kind == "VEC3"
            and not normalized
            and len(rows) == counts[0],
            "Invalid morph stream",
        )
        streams.append((component, rows))
    for component, rows in streams:
        if component == 5126:
            require(
                all(
                    math.isfinite(v)
                    for row in rows
                    for v in struct.unpack("<" + "f" * (len(row) // 4), row)
                ),
                "Nonfinite geometry stream",
            )
    kind, component, normalized, rows = values(doc, binary, primitive["indices"])
    require(
        component in (5121, 5123, 5125) and kind == "SCALAR" and not normalized,
        "Invalid indices",
    )
    fmt = {5121: "<B", 5123: "<H", 5125: "<I"}[component]
    ids = [struct.unpack(fmt, r)[0] for r in rows]
    require(
        ids and len(ids) % 3 == 0 and max(ids) < counts[0],
        "Triangle indices outside Face",
    )
    material = index(primitive.get("material"), doc["materials"], "material")
    require(not material.get("extensions"), "Unsupported material extension")
    require(
        not any(k.endswith("Texture") for k in material),
        "Unsupported material texture slot",
    )
    require(
        not any(
            k.endswith("Texture") and k != "baseColorTexture"
            for k in material.get("pbrMetallicRoughness", {})
        ),
        "Unsupported PBR texture slot",
    )
    texture_info = material.get("pbrMetallicRoughness", {}).get("baseColorTexture", {})

    def extension_check(value):
        if isinstance(value, dict):
            if "extensions" in value:
                require(
                    value is texture_info
                    and set(value["extensions"]) <= {"KHR_texture_transform"},
                    "Unsupported nested extension",
                )
            for item in value.values():
                extension_check(item)
        elif isinstance(value, list):
            for item in value:
                extension_check(item)

    extension_check(doc)
    texture = index(texture_info.get("index"), doc["textures"], "base texture")
    require(not texture.get("extensions"), "Unsupported texture extension")
    image_info = index(texture.get("source"), doc["images"], "texture image")
    if "sampler" in texture:
        index(texture["sampler"], doc.get("samplers", []), "sampler")
    uv = texture_info.get("texCoord", 0)
    extensions = texture_info.get("extensions", {})
    require(
        set(extensions) <= {"KHR_texture_transform"}, "Unsupported texture transform"
    )
    transform = extensions.get("KHR_texture_transform", {})
    uv = transform.get("texCoord", uv)
    require(
        type(uv) is int and "TEXCOORD_" + str(uv) in attrs,
        "Missing required texture coordinates",
    )
    for key, size in [("offset", 2), ("scale", 2)]:
        if key in transform:
            require(
                len(transform[key]) == size
                and all(
                    type(v) in (int, float) and math.isfinite(v) for v in transform[key]
                ),
                "Invalid texture transform",
            )
    require(
        type(transform.get("rotation", 0)) in (int, float)
        and math.isfinite(transform.get("rotation", 0)),
        "Invalid texture rotation",
    )
    require(
        set(doc.get("extensionsUsed", [])) <= {"KHR_texture_transform"},
        "Unsupported declared extension",
    )
    require(
        set(doc.get("extensionsRequired", [])) <= {"KHR_texture_transform"},
        "Unsupported required extension",
    )
    require(
        set(doc.get("extensionsRequired", [])) <= set(doc.get("extensionsUsed", [])),
        "Undeclared required extension",
    )
    if "KHR_texture_transform" in extensions:
        require(
            "KHR_texture_transform" in doc.get("extensionsUsed", []),
            "Undeclared texture transform",
        )
    require(
        "uri" not in image_info and image_info.get("mimeType") == "image/png",
        "Expected embedded PNG",
    )
    view = index(image_info.get("bufferView"), doc["bufferViews"], "image view")
    require(
        view.get("buffer") == 0
        and not view.get("extensions")
        and "byteStride" not in view,
        "Unsupported image storage",
    )
    start, length = view.get("byteOffset", 0), view["byteLength"]
    require(
        type(start) is int
        and type(length) is int
        and start >= 0
        and length > 0
        and start + length <= len(binary),
        "Invalid image range",
    )
    embedded = binary[start : start + length]
    size, pixels = rgba(embedded)
    source_size, source_pixels = rgba(source_png)
    require(
        size == source_size and pixels == source_pixels,
        "Embedded image differs from same-job atlas",
    )
    return {
        "schema_version": 1,
        "vertices": counts[0],
        "triangles": len(ids) // 3,
        "glb_sha256": hashlib.sha256(raw).hexdigest(),
        "embedded_png_sha256": hashlib.sha256(embedded).hexdigest(),
        "source_png_sha256": hashlib.sha256(source_png).hexdigest(),
        "image_size": list(size),
        "decoded_rgba_equal": True,
        "encoded_png_equal": embedded == source_png,
        "scope": "Face-only static assembly with MouthOpen and base-color PBR; NPR shaders/control maps/SDF require separate definition. Structural validation is not visual acceptance.",
    }


if __name__ == "__main__":
    import argparse
    import json

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("face_glb", type=Path)
    parser.add_argument("same_job_base_png", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            validate(args.face_glb.read_bytes(), args.same_job_base_png.read_bytes()),
            indent=2,
        )
        + "\n"
    )

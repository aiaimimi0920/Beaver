"""Exact joint Face-export comparison, allowing only vertex/triangle reordering."""

import argparse
import hashlib
import json
from pathlib import Path
import struct
import sys

sys.path.append(str(Path(__file__).resolve().parents[1] / "general_face"))
from glb_affine import read_glb, require
from glb_components import values


def record(parts):
    return b"".join(len(part).to_bytes(8, "little") + part for part in parts)


def encoded(value):
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode()


def canonical_face(raw):
    doc, binary = read_glb(raw)
    allowed = {
        "asset",
        "buffers",
        "bufferViews",
        "accessors",
        "nodes",
        "meshes",
        "scenes",
        "scene",
        "materials",
        "textures",
        "samplers",
        "images",
        "extensionsUsed",
        "extensionsRequired",
    }
    require(not (set(doc) - allowed), "Unsupported top-level GLB behavior or metadata")

    def reject_extensions(value, material=False):
        if isinstance(value, dict):
            if "extensions" in value:
                require(
                    material and set(value["extensions"]) <= {"KHR_texture_transform"},
                    "Unsupported nested GLB extensions",
                )
            for item in value.values():
                reject_extensions(item, material)
        elif isinstance(value, list):
            for item in value:
                reject_extensions(item, material)

    for key, value in doc.items():
        reject_extensions(value, key == "materials")
    for key in ("extensionsUsed", "extensionsRequired"):
        require(
            set(doc.get(key, [])) <= {"KHR_texture_transform"},
            "Unsupported declared extensions",
        )
    require(
        len(doc.get("nodes", [])) == 1 and len(doc.get("meshes", [])) == 1,
        "Expected a standalone same-job Face export, not a full-character GLB",
    )
    node = doc["nodes"][0]
    require(
        node.get("name") == "Face" and node.get("mesh") == 0,
        "Expected unique Face node and mesh",
    )
    require(
        not any(
            k in node
            for k in (
                "matrix",
                "translation",
                "rotation",
                "scale",
                "skin",
                "extensions",
            )
        ),
        "Face export must have an identity transform and no skin/extensions",
    )
    mesh = doc["meshes"][0]
    require(
        mesh.get("name") == "Face" and not mesh.get("extensions"),
        "Unsupported Face mesh",
    )
    primitives = []
    triangle_count = vertex_count = 0
    for primitive in mesh["primitives"]:
        require(
            primitive.get("mode", 4) == 4 and not primitive.get("extensions"),
            "Expected triangle primitive without extensions",
        )
        fields = [
            ("attribute/" + key, index)
            for key, index in sorted(primitive["attributes"].items())
        ]
        for target_index, target in enumerate(primitive.get("targets", [])):
            fields.extend(
                (f"morph/{target_index}/{key}", index)
                for key, index in sorted(target.items())
            )
        require("POSITION" in primitive["attributes"], "Missing positions")
        count = doc["accessors"][primitive["attributes"]["POSITION"]]["count"]
        descriptors, rows = [], []
        for name, index in fields:
            kind, component, normalized, expanded = values(doc, binary, index)
            require(len(expanded) == count, "Attribute/morph vertex counts differ")
            accessor = doc["accessors"][index]
            descriptors.append(
                [
                    name,
                    kind,
                    component,
                    normalized,
                    {k: accessor[k] for k in ("min", "max", "extras") if k in accessor},
                ]
            )
            rows.append(expanded)
        vertices = [record(parts) for parts in zip(*rows)]
        require(
            len(set(vertices)) == len(vertices),
            "Duplicate joint vertex rows: topology bijection is not proven",
        )
        if "indices" in primitive:
            kind, component, normalized, indices = values(
                doc, binary, primitive["indices"]
            )
            require(
                kind == "SCALAR" and component in (5121, 5123, 5125) and not normalized,
                "Invalid triangle indices",
            )
            fmt = {5121: "<B", 5123: "<H", 5125: "<I"}[component]
            ids = [struct.unpack(fmt, row)[0] for row in indices]
        else:
            ids = list(range(count))
        require(
            len(ids) % 3 == 0 and all(0 <= i < count for i in ids),
            "Invalid triangle index range",
        )
        triangles = []
        for start in range(0, len(ids), 3):
            triangle = [vertices[i] for i in ids[start : start + 3]]
            # Cyclic rotation preserves orientation; reversing winding is not allowed.
            triangles.append(min(record(triangle[k:] + triangle[:k]) for k in range(3)))
        primitives.append(
            (
                encoded(descriptors),
                encoded(
                    {k: primitive[k] for k in ("material", "extras") if k in primitive}
                ),
                tuple(sorted(vertices)),
                tuple(sorted(triangles)),
            )
        )
        vertex_count += count
        triangle_count += len(triangles)
    images = []
    for image in doc.get("images", []):
        require(
            "bufferView" in image and "uri" not in image,
            "Only embedded image payloads are compared",
        )
        view_id = image["bufferView"]
        require(
            type(view_id) is int and 0 <= view_id < len(doc["bufferViews"]),
            "Invalid image buffer view",
        )
        view = doc["bufferViews"][view_id]
        require(
            view.get("buffer", 0) == 0 and "byteStride" not in view,
            "Unsupported image storage",
        )
        start, size = view.get("byteOffset", 0), view["byteLength"]
        require(
            start >= 0 and size > 0 and start + size <= len(binary),
            "Invalid image range",
        )
        images.append(
            (
                encoded({k: v for k, v in image.items() if k != "bufferView"}),
                binary[start : start + size],
            )
        )
    metadata = {
        k: doc.get(k)
        for k in (
            "nodes",
            "scenes",
            "scene",
            "materials",
            "textures",
            "samplers",
            "asset",
            "extensionsUsed",
            "extensionsRequired",
        )
    }
    metadata["mesh_metadata"] = {k: v for k, v in mesh.items() if k != "primitives"}
    return (
        (encoded(metadata), tuple(primitives), tuple(images)),
        vertex_count,
        triangle_count,
    )


def digest(value):
    if isinstance(value, bytes):
        return hashlib.sha256(record([b"bytes", value])).digest()
    return hashlib.sha256(record([b"tuple", *(digest(v) for v in value)])).digest()


def compare(first, second):
    a, av, at = canonical_face(first)
    b, bv, bt = canonical_face(second)
    return {
        "schema": "exact-face-export-permutation-v1",
        "scope": "Joint attributes, morphs, oriented triangles, uniquely identified/unreferenced vertices, materials and embedded images; ordering alone ignored",
        "exact_permutation_equivalent": a == b,
        "raw_bytes_equal": first == second,
        "first": {
            "sha256": hashlib.sha256(first).hexdigest(),
            "canonical_sha256": digest(a).hex(),
            "vertices": av,
            "triangles": at,
        },
        "second": {
            "sha256": hashlib.sha256(second).hexdigest(),
            "canonical_sha256": digest(b).hex(),
            "vertices": bv,
            "triangles": bt,
        },
        "not_proven": [
            "visual quality",
            "collision freedom",
            "all-parameter stability",
            "engine import behavior",
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--first", type=Path, required=True)
    parser.add_argument("--second", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = compare(args.first.read_bytes(), args.second.read_bytes())
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    if not result["exact_permutation_equivalent"]:
        raise SystemExit(
            "Face exports differ beyond ordering; inspect before claiming repeatability"
        )

"""Check a pinned Faye capture, then export its original PNG bytes.

Usage: python verify_capture.py WORKSPACE MANIFEST [OUTPUT_DIRECTORY]
This is consistency checking, not signed renderer attestation or visual approval.
"""
from pathlib import Path
import hashlib
import json
import math
import re
import shutil
import struct
import sys
import zlib

MODEL = "ee91af5f00f1b1d267c0ac6a46c5227a7f7ba6fad2390a5f30bac228415551a4"
COMPOSITION = "8e9e09d6d02c6ddc46de45fc6c19896d1f0dded2d3c727649701a24833b04871"
RUNTIME_ROOTS = ["res://addons/npr_character_frame", "res://showcase_extensions"]
MOUTH_CHANNELS = {"MouthOpen", "MouthPucker", "MouthRound", "MouthSpread"}
REQUIRED_TEXTURES = {"face_base", "face_ilm", "face_map", "face_ramp", "face_sdf360"}
MATERIAL_SCRIPTS = [
    "res://addons/npr_character_frame/npr_character_materials.gd",
    "res://addons/npr_character_frame/shaders/face/sdf360/sdf_materials.gd",
]
RUNTIME_SENTINELS = [
    "res://addons/npr_character_frame/npr_character.gd",
    "res://addons/npr_character_frame/shaders/face/sdf360/sdf_materials.gd",
    "res://addons/npr_character_frame/shaders/face/sdf360/face_base.gdshader",
    "res://showcase_extensions/faye_capture_probe.gd",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1048576), b""):
            digest.update(block)
    return digest.hexdigest()


def finite_number(value):
    return type(value) in (int, float) and math.isfinite(value)


def verify_vector(value):
    require(isinstance(value, list) and len(value) == 3 and all(map(finite_number, value)),
            "Camera vector must contain three finite numbers")


def verify_neutral(mouth, blink):
    require(isinstance(mouth, dict) and set(mouth) == MOUTH_CHANNELS,
            "Missing or unexpected mouth channels")
    require(all(finite_number(value) and value == 0 for value in mouth.values()), "Non-neutral mouth")
    require(isinstance(blink, list) and len(blink) == 2
            and all(finite_number(value) and value == 0 for value in blink), "Non-neutral blink")


def resource_path(workspace, value):
    require(isinstance(value, str) and value.startswith("res://"), "Invalid resource path")
    result = (workspace / value[6:]).resolve()
    require(result.is_relative_to(workspace), "Resource escapes workspace")
    return result


def verify_textures(workspace, before, after):
    require(isinstance(before, dict) and before and before == after, "Texture provenance changed")
    fields = set()
    for script in MATERIAL_SCRIPTS:
        text = resource_path(workspace, script).read_text()
        fields.update(re.findall(r"@export\s+var\s+(\w+)\s*:\s*Texture2D\b", text))
    require(REQUIRED_TEXTURES <= fields and set(before) == fields, "Incomplete texture field snapshot")
    for field, item in before.items():
        if item is None:
            require(field not in REQUIRED_TEXTURES, "Missing required texture: " + field)
            continue
        require(isinstance(item, dict) and set(item) == {"path", "sha256"}, "Malformed texture binding")
        require(sha(resource_path(workspace, item["path"])) == item["sha256"], "Texture bytes changed: " + field)


def verify_scale(base, size):
    require(all(isinstance(value, (int, float)) and not isinstance(value, bool)
                and math.isfinite(value) and value > 0 for value in [base, size]),
            "Invalid camera scale")
    require(math.isclose(size, base * .85, rel_tol=1e-6, abs_tol=1e-8), "Wrong camera scale")


def verify_runtime(workspace, bindings):
    for root in RUNTIME_ROOTS:
        folder = (workspace / root[6:]).resolve()
        require(folder.is_relative_to(workspace) and folder.is_dir(), "Missing runtime directory: " + root)
    for key in RUNTIME_SENTINELS:
        require(key in bindings and (workspace / key[6:]).is_file(), "Missing required renderer: " + key)
    for root in RUNTIME_ROOTS:
        folder = workspace / root[6:]
        for path in folder.rglob("*"):
            if path.is_file() and not any(part.startswith(".") for part in path.relative_to(folder).parts):
                require(path.resolve().is_relative_to(workspace), "Runtime dependency escapes workspace")
                key = "res://" + path.relative_to(workspace).as_posix()
                require(bindings.get(key) == sha(path), "Missing runtime dependency: " + key)


def verify_contract(data):
    require(isinstance(data, dict) and type(data.get("schema_version")) is int
            and data["schema_version"] == 1, "Unsupported manifest schema")
    require(data.get("rendering_method") == "forward_plus"
            and data.get("rendering_driver") in ("vulkan", "metal", "d3d12"), "Unsupported capture renderer")
    require(data.get("speech_aligned") is False and data.get("continuous_animation") is False,
            "Unsupported capture scope")


def frame_path(workspace, manifest, value, index):
    path = resource_path(workspace, value)
    require(path == manifest.parent / f"frame_{index:03d}.png", "Image is not the indexed capture frame")
    return path


def verify_png(path):
    """Fully decode the bounded, noninterlaced 8-bit RGB/RGBA PNGs emitted here."""
    require(path.stat().st_size <= 64 * 1024 * 1024, "PNG is too large")
    data = path.read_bytes()
    require(data[:8] == b"\x89PNG\r\n\x1a\n", "Invalid PNG signature")
    pos, header, ended, idat_closed, palette = 8, None, False, False, False
    chunks = []
    srgb = False
    while pos < len(data):
        require(pos + 12 <= len(data), "Truncated PNG chunk")
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        require(re.fullmatch(b"[A-Za-z]{4}", kind) and 65 <= kind[2] <= 90, "Invalid PNG chunk type")
        end = pos + 12 + length
        require(end <= len(data), "Truncated PNG payload")
        payload = data[pos + 8:end - 4]
        require(zlib.crc32(kind + payload) & 0xffffffff == struct.unpack(">I", data[end - 4:end])[0],
                "PNG CRC mismatch")
        require(header is not None or kind == b"IHDR", "PNG must begin with IHDR")
        if kind == b"IHDR":
            require(header is None and pos == 8 and length == 13, "Invalid PNG IHDR")
            header = struct.unpack(">IIBBBBB", payload)
            width, height, depth, color, compression, filtering, interlace = header
            require(0 < width <= 8192 and 0 < height <= 8192 and width * height <= 16777216,
                    "Invalid PNG dimensions")
            require(depth == 8 and color in (2, 6) and (compression, filtering, interlace) == (0, 0, 0),
                    "Unsupported PNG encoding; expected noninterlaced 8-bit RGB/RGBA")
        elif kind == b"PLTE":
            require(not palette and not chunks and 0 < length <= 768 and length % 3 == 0, "Invalid PNG palette")
            palette = True
        elif kind == b"sRGB":
            require(not srgb and not palette and not chunks and length == 1 and payload[0] <= 3,
                    "Invalid PNG sRGB chunk")
            srgb = True
        elif kind == b"IDAT":
            require(not idat_closed, "Noncontiguous PNG IDAT")
            chunks.append(payload)
        elif kind == b"IEND":
            require(length == 0 and chunks and end == len(data), "Invalid PNG ending")
            ended = True
        else:
            raise ValueError("Unsupported PNG chunk in capture output")
        pos = end
        if ended:
            break
    require(ended and header is not None, "Missing PNG ending")
    width, height, _, color, *_ = header
    channels = 3 if color == 2 else 4
    stride = width * channels
    expected = (stride + 1) * height
    decoder = zlib.decompressobj()
    try:
        raw = decoder.decompress(b"".join(chunks), expected + 1)
    except zlib.error as error:
        raise ValueError("Invalid PNG compressed pixels") from error
    require(len(raw) == expected and decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail,
            "Invalid PNG decompressed payload")
    previous = bytearray(stride)
    for row_index in range(height):
        offset = row_index * (stride + 1)
        method = raw[offset]
        require(method <= 4, "Invalid PNG row filter")
        row = bytearray(raw[offset + 1:offset + 1 + stride])
        if method:
            for index in range(stride):
                left = row[index - channels] if index >= channels else 0
                up = previous[index]
                corner = previous[index - channels] if index >= channels else 0
                if method == 1:
                    predictor = left
                elif method == 2:
                    predictor = up
                elif method == 3:
                    predictor = (left + up) // 2
                else:
                    center = left + up - corner
                    distances = (abs(center - left), abs(center - up), abs(center - corner))
                    predictor = (left, up, corner)[distances.index(min(distances))]
                row[index] = (row[index] + predictor) & 255
        previous = row
    return width, height


def verify_entry_scene(workspace, data):
    require(data["entry_scene_sha256_before"] == data["entry_scene_sha256_after"]
            == data["entry_scene_sha256"], "Entry scene changed")
    require(data["entry_scene_sha256"]
            == data["bindings_before"]["res://showcase/faye_face_capture.tscn"], "Wrong capture scene")
    entry = resource_path(workspace, data.get("entry_scene_at_capture"))
    require(entry.is_file() and sha(entry) == data["entry_scene_sha256"],
            "Actual capture entry scene is missing or changed")


def verify(workspace, manifest):
    workspace = Path(workspace).resolve()
    manifest = Path(manifest).resolve()
    require(manifest.is_relative_to(workspace), "Manifest is outside workspace")
    data = json.loads(manifest.read_text())
    verify_contract(data)

    def resource(value):
        return resource_path(workspace, value)

    require(type(data.get("expected")) is int and data["expected"] == 9
            and isinstance(data.get("captures"), list) and len(data["captures"]) == 9, "Expected nine captures")
    for group in ["source", "bindings"]:
        before, after = data[group + "_before"], data[group + "_after"]
        require(before and before == after, "Provenance changed: " + group)
        require(all(isinstance(v, str) and re.fullmatch(r"[a-f0-9]{64}", v)
                    for v in before.values()), "Missing provenance digest")
    verify_entry_scene(workspace, data)
    for kind, key in [("model", "source"), ("definition", "definition"),
                      ("report", "report"), ("atlas", "atlas")]:
        require(sha(resource(data[key])) == data["source_before"][kind], "Source mismatch: " + kind)
    require(data["source_before"]["model"] == MODEL, "Wrong model identity")
    require(data["composition_sha256"] == COMPOSITION, "Wrong composition identity")
    report = json.loads(resource(data["report"]).read_text())
    require(report["face_components"]["composition_sha256"] == COMPOSITION, "Wrong report composition")
    require(report["new_glb_sha256"] == MODEL, "Report belongs to another model")
    require(report["semantic_eyes"]["sdf"]["atlas_sha256"] == data["source_before"]["atlas"],
            "Report belongs to another atlas")
    require(re.fullmatch(r"[a-f0-9]{64}", data["engine_sha256"]), "Missing engine identity")
    require(data["runtime_roots"] == RUNTIME_ROOTS, "Wrong runtime roots")
    for key, value in data["bindings_before"].items():
        require(sha(resource(key)) == value, "Runtime binding mismatch: " + key)
    require(data["bindings_before"].get("res://project.godot") == sha(resource("res://project.godot")),
            "Project settings are not bound")
    verify_runtime(workspace, data["bindings_before"])
    verify_textures(workspace, data["textures_before"], data["textures_after"])
    require(data["textures_before"]["face_sdf360"] == {"path": data["atlas"],
            "sha256": data["source_before"]["atlas"]}, "Atlas field binding mismatch")

    rows = []
    reference = data["captures"][0]["actual_view"]
    for index, capture in enumerate(data["captures"]):
        mode = ["render", "white", "edges"][index // 3]
        view = ["front", "right45", "right90"][index % 3]
        yaw = [0, 45, 90][index % 3]
        expected = {"mode": mode, "mouth": {}, "pitch": 0, "scale": .85, "view": view, "yaw": yaw}
        require(capture["requested"] == expected, "Wrong capture order or request")
        require(all(finite_number(capture["requested"][key]) for key in ["yaw", "pitch", "scale"]),
                "Invalid requested camera number")
        actual = capture["actual_view"]
        verify_scale(data["base_view_size"], actual["size"])
        require(all(finite_number(actual[key]) for key in ["yaw", "pitch", "camera_pitch"]),
                "Invalid actual camera number")
        require(type(actual["projection"]) is int, "Invalid projection type")
        for vector in [actual["target"], actual["position"], capture["expected_target"], capture["expected_position"]]:
            verify_vector(vector)
        require(actual["mode"] == mode and actual["yaw"] == yaw and actual["pitch"] == 0
                and actual["camera_pitch"] == 0 and actual["projection"] == 1, "Wrong actual view")
        require(actual["face_visible"] is True and actual["body_visible"] is False and actual["hair_visible"] is False,
                "Unexpected visibility")
        require(actual["target"] == capture["expected_target"]
                and actual["position"] == capture["expected_position"], "Camera transform mismatch")
        require(actual["size"] == reference["size"] and actual["viewport_size"] == reference["viewport_size"],
                "Capture framing changed")
        verify_neutral(capture["actual_mouth"], capture["actual_blink"])
        path = frame_path(workspace, manifest, capture["image"], index)
        require(sha(path) == capture["sha256"], "Image digest mismatch")
        viewport = actual["viewport_size"]
        require(isinstance(viewport, list) and len(viewport) == 2
                and all(type(value) is int and value > 0 for value in viewport), "Invalid viewport dimensions")
        require(verify_png(path) == tuple(viewport), "PNG dimensions mismatch")
        rows.append({"name": f"{index + 1:02d}_Faye_{mode}_{view}.png", "source": path,
                     "sha256": capture["sha256"]})
    require(len({row["sha256"] for row in rows}) == 9, "Duplicate images")
    return data, rows


def main():
    require(len(sys.argv) in [3, 4], "Usage: verify_capture.py WORKSPACE MANIFEST [OUTPUT_DIRECTORY]")
    data, rows = verify(sys.argv[1], sys.argv[2])
    output = Path(sys.argv[3]).resolve() if len(sys.argv) == 4 else Path.cwd() / "faye-capture-verified"
    exports = output / "images"
    exports.mkdir(parents=True, exist_ok=True)
    files = []
    for row in rows:
        destination = exports / row["name"]
        shutil.copyfile(row["source"], destination)
        require(sha(destination) == row["sha256"], "Export changed image bytes")
        files.append({"name": row["name"], "path": str(destination), "sha256": row["sha256"]})
    result = {"verified": True, "manifest_sha256": sha(Path(sys.argv[2])),
              "same_model": data["source_before"]["model"], "files": files,
              "visual_review": "Manual inspection required; consistency checks do not accept artistic likeness."}
    (output / "verification.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()

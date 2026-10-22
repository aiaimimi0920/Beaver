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

MODEL = "ee91af5f00f1b1d267c0ac6a46c5227a7f7ba6fad2390a5f30bac228415551a4"
COMPOSITION = "8e9e09d6d02c6ddc46de45fc6c19896d1f0dded2d3c727649701a24833b04871"
RUNTIME_ROOTS = ["res://addons/npr_character_frame", "res://showcase_extensions"]
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


def verify(workspace, manifest):
    workspace = Path(workspace).resolve()
    manifest = Path(manifest).resolve()
    require(manifest.is_relative_to(workspace), "Manifest is outside workspace")
    data = json.loads(manifest.read_text())

    def resource(value):
        require(isinstance(value, str) and value.startswith("res://"), "Invalid resource path")
        result = (workspace / value[6:]).resolve()
        require(result.is_relative_to(workspace), "Resource escapes workspace")
        return result

    require(data["expected"] == len(data["captures"]) == 9, "Expected nine captures")
    for group in ["source", "bindings"]:
        before, after = data[group + "_before"], data[group + "_after"]
        require(before and before == after, "Provenance changed: " + group)
        require(all(isinstance(v, str) and re.fullmatch(r"[a-f0-9]{64}", v)
                    for v in before.values()), "Missing provenance digest")
    require(data["entry_scene_sha256_before"] == data["entry_scene_sha256_after"]
            == data["entry_scene_sha256"], "Entry scene changed")
    require(data["entry_scene_sha256"]
            == data["bindings_before"]["res://showcase/faye_face_capture.tscn"], "Wrong capture scene")
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

    rows = []
    reference = data["captures"][0]["actual_view"]
    for index, capture in enumerate(data["captures"]):
        mode = ["render", "white", "edges"][index // 3]
        view = ["front", "right45", "right90"][index % 3]
        yaw = [0, 45, 90][index % 3]
        expected = {"mode": mode, "mouth": {}, "pitch": 0, "scale": .85, "view": view, "yaw": yaw}
        require(capture["requested"] == expected, "Wrong capture order or request")
        actual = capture["actual_view"]
        verify_scale(data["base_view_size"], actual["size"])
        require(actual["mode"] == mode and actual["yaw"] == yaw and actual["pitch"] == 0
                and actual["camera_pitch"] == 0 and actual["projection"] == 1, "Wrong actual view")
        require(actual["face_visible"] and not actual["body_visible"] and not actual["hair_visible"],
                "Unexpected visibility")
        require(actual["target"] == capture["expected_target"]
                and actual["position"] == capture["expected_position"], "Camera transform mismatch")
        require(actual["size"] == reference["size"] and actual["viewport_size"] == reference["viewport_size"],
                "Capture framing changed")
        require(capture["actual_blink"] == [0, 0]
                and all(value == 0 for value in capture["actual_mouth"].values()), "Non-neutral expression")
        path = resource(capture["image"])
        require(sha(path) == capture["sha256"], "Image digest mismatch")
        with path.open("rb") as stream:
            header = stream.read(24)
        require(header[:8] == b"\x89PNG\r\n\x1a\n" and header[12:16] == b"IHDR", "Invalid PNG header")
        require(struct.unpack(">II", header[16:24]) == tuple(actual["viewport_size"]), "PNG dimensions mismatch")
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

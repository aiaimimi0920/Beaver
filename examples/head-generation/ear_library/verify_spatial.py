"""Bind supplementary still evidence; not aesthetic or continuous-animation approval."""

import hashlib
import json
import math
from pathlib import Path
import re

from PIL import Image

HERE = Path(__file__).resolve().parent
MOUTH = {"MouthOpen", "MouthRound", "MouthSpread", "MouthPucker"}
BINDINGS = {"res://showcase_extensions/" + name for name in (
    "ear_spatial_probe.gd", "ear_spatial_schedule.json", "ear_capture_view.gd",
    "researched_mouth_review.gd", "head_review.gd", "head_stage.gd", "head_display.gd",
    "head_edges.gdshader", "head_facets.gdshader",
)} | {"res://addons/npr_character_frame/showcase/wardrobe.gd"}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def resource(root, name):
    if not isinstance(name, str) or not name.startswith("res://"):
        raise ValueError("Expected project resource")
    path = (root / name[6:]).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        raise ValueError("Missing or escaping resource")
    return path


def same_number(actual, expected):
    return (type(actual) in (int, float) and math.isfinite(actual)
            and abs(actual - expected) < 0.0001)


def verify_spatial(workspace, manifest_path, prefix, selection):
    root = Path(workspace).resolve()
    if not re.fullmatch(r"assets/aster/general_face_eyes_combo[0-9]+/", prefix):
        raise ValueError("Unexpected model prefix")
    manifest = Path(manifest_path).resolve()
    if (not manifest.is_relative_to(root / "artifacts/ear_spatial")
            or manifest.name != "manifest.json"):
        raise ValueError("Unexpected capture manifest")
    data = json.loads(manifest.read_text())
    if not isinstance(data, dict):
        raise ValueError("Invalid capture manifest")
    if (type(data.get("schema_version")) is not int or data.get("schema_version") != 1 or data.get("expected") != 18
            or data.get("speech_aligned") is not False
            or data.get("continuous_animation") is not False):
        raise ValueError("Unsupported capture contract")
    paths = {"model": "res://" + prefix + "aster_head.glb",
             "definition": "res://" + prefix + "aster_definition.tres",
             "report": "res://" + prefix + "generation_report.json"}
    if (data.get("source") != paths["model"] or data.get("definition") != paths["definition"]
            or data.get("report") != paths["report"]):
        raise ValueError("Wrong captured model identity")
    hashes = {key: digest(resource(root, path)) for key, path in paths.items()}
    if data.get("source_before") != hashes or data.get("source_after") != hashes:
        raise ValueError("Capture inputs changed")
    report = json.loads(resource(root, paths["report"]).read_text())
    components = report.get("face_components") if isinstance(report, dict) else None
    if (not isinstance(components, dict) or report.get("new_glb_sha256") != hashes["model"]
            or components.get("selection") != selection):
        raise ValueError("Report/model/selection mismatch")
    bindings = {path: digest(resource(root, path)) for path in BINDINGS}
    for name in ("ear_spatial_probe.gd", "ear_spatial_schedule.json", "ear_capture_view.gd"):
        if bindings["res://showcase_extensions/" + name] != digest(HERE / name):
            raise ValueError("Supplementary probe source mismatch")
    if data.get("bindings_before") != bindings or data.get("bindings_after") != bindings:
        raise ValueError("Controller or probe bindings changed")
    schedule = json.loads((HERE / "ear_spatial_schedule.json").read_text())
    captures = data.get("captures")
    if not isinstance(captures, list) or len(captures) != len(schedule):
        raise ValueError("Incomplete supplementary captures")
    base = data.get("base_view_size")
    if type(base) not in (int, float) or not math.isfinite(base) or base <= 0:
        raise ValueError("Invalid base view size")
    for index, (row, expected) in enumerate(zip(captures, schedule)):
        if not isinstance(row, dict) or row.get("requested") != expected:
            raise ValueError("Capture schedule differs")
        mouth = row.get("actual_mouth", {})
        if not isinstance(mouth, dict) or set(mouth) != MOUTH or any(not same_number(mouth[k], expected["mouth"].get(k, 0.)) for k in MOUTH):
            raise ValueError("Actual mouth differs")
        blink = row.get("actual_blink")
        if not isinstance(blink, list) or len(blink) != 2 or any(not same_number(x, 0.) for x in blink):
            raise ValueError("Actual blink differs")
        view = row.get("actual_view", {})
        if not isinstance(view, dict):
            raise ValueError("Missing actual camera")
        for key, value in {"size": base * expected["scale"], "yaw": expected["yaw"],
                           "pitch": expected["pitch"], "camera_pitch": expected["pitch"]}.items():
            if not same_number(view.get(key), value):
                raise ValueError("Actual camera differs: " + key)
        for key in ("target", "position"):
            expected_vector = row.get("expected_" + key)
            actual_vector = view.get(key)
            if (not isinstance(expected_vector, list) or len(expected_vector) != 3
                    or any(type(x) not in (int, float) or not math.isfinite(x) for x in expected_vector)
                    or not isinstance(actual_vector, list) or len(actual_vector) != 3
                    or any(not same_number(a, b) for a, b in zip(actual_vector, expected_vector))):
                raise ValueError("Camera framing changed during capture")
        dimensions = view.get("viewport_size")
        if (view.get("mode") != expected["mode"] or type(view.get("projection")) is not int or view.get("projection") != 1
                or view.get("face_visible") is not True or view.get("body_visible") is not False
                or view.get("hair_visible") is not False or not isinstance(dimensions, list)
                or len(dimensions) != 2 or any(type(x) is not int or x <= 100 for x in dimensions)):
            raise ValueError("Actual visible viewport differs")
        image = resource(root, row.get("image"))
        if image != manifest.parent / f"frame_{index:03d}.png" or digest(image) != row.get("sha256"):
            raise ValueError("Changed or misplaced image")
        try:
            with Image.open(image) as png:
                if png.format != "PNG" or list(png.size) != dimensions:
                    raise ValueError("PNG dimensions differ from captured viewport")
                png.verify()
            with Image.open(image) as png:
                png.load()
        except (OSError, SyntaxError) as error:
            raise ValueError("Invalid captured PNG") from error
    return {"status": "technically_verified", "captures_verified": len(captures),
            "model_sha256": hashes["model"], "manifest_sha256": digest(manifest),
            "scope": "Profile edges and two orthographic zoom scales only; visual review remains separate"}

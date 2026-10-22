"""Fail-closed verification of the sampled native mouth controller evidence."""

import hashlib
import json
import math
from pathlib import Path

from capture_schedule import schedule
from capture_bindings import verify_bindings
from mouth_motion import validate_weights

HERE = Path(__file__).resolve().parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def finite(value):
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(value)
    )


def verify_capture(workspace, manifest, expected_source, probe_profile="mouth"):
    from PIL import Image

    root = Path(workspace).resolve()
    data = json.loads(Path(manifest).read_text())

    def resource(value):
        if not isinstance(value, str) or not value.startswith("res://"):
            raise ValueError("Expected resource path")
        path = (root / value[6:]).resolve()
        if not path.is_relative_to(root):
            raise ValueError("Resource escapes workspace")
        return path

    if data["source"] != expected_source:
        raise ValueError("Unexpected model source")
    source = digest(resource(expected_source))
    if data["source_before"] != source or data["source_after"] != source:
        raise ValueError("Changed model source")
    verify_bindings(root, data, probe_profile)
    rows = data["captures"]
    if (
        data.get("expected") != 92
        or len(rows) != 92
        or data.get("speech_aligned") is not False
    ):
        raise ValueError("Invalid capture scope")
    if [row["requested"] for row in rows] != schedule():
        raise ValueError("Capture schedule mismatch")
    seen = set()
    size = None
    dimensions = None
    for row in rows:
        expected = row["requested"]
        weights = row["actual_mouth"]
        wanted = validate_weights(expected["mouth"])
        if set(weights) != set(wanted) or any(
            not finite(weights[k]) or abs(weights[k] - wanted[k]) > 1e-6 for k in wanted
        ):
            raise ValueError("Actual mouth weights mismatch")
        blink = row["actual_blink"]
        if len(blink) != 2 or any(
            not finite(a) or abs(a - b) > 1e-6 for a, b in zip(blink, expected["blink"])
        ):
            raise ValueError("Actual blink weights mismatch")
        view = row["actual_view"]
        if (
            view.get("face_visible") is not True
            or view.get("body_visible") is not False
            or view.get("hair_visible") is not False
        ):
            raise ValueError("Invalid head visibility")
        if view.get("mode") != expected["mode"] or view.get("projection") != 1:
            raise ValueError("Invalid display mode or projection")
        for key in ("yaw", "pitch", "camera_pitch"):
            target = expected["pitch" if key == "camera_pitch" else key]
            if not finite(view.get(key)) or abs(view[key] - target) > 1e-4:
                raise ValueError("Actual camera pose mismatch")
        if not finite(view.get("size")) or view["size"] <= 0:
            raise ValueError("Invalid camera size")
        if size is None:
            size = view["size"]
        if abs(view["size"] - size) > 1e-6:
            raise ValueError("Camera size changed")
        path = resource(row["image"])
        if path in seen or digest(path) != row["sha256"]:
            raise ValueError("Duplicate or changed image")
        seen.add(path)
        with Image.open(path) as image:
            if image.format != "PNG" or list(image.size) != view.get("viewport_size"):
                raise ValueError("Capture dimensions mismatch")
            if dimensions is None:
                dimensions = image.size
            if image.size != dimensions or min(image.size) <= 0:
                raise ValueError("Viewport dimensions changed")
            image.verify()
    return {
        "captures_verified": 92,
        "source_sha256": source,
        "scope": "Sampled manual controller and image integrity; not aesthetic, speech or continuous collision acceptance",
    }

"""Verify source-bound native captures, including actual camera/visibility state."""

import argparse
import hashlib
import json
import math
import struct
from pathlib import Path


def expected_views(profile="eye"):
    """The exact 84-view native eye-family schedule, including wireframe views."""
    if profile == "mouth":
        return {
            (yaw, 0, 0, 0, weight, mode)
            for yaw in (0, -45, 45)
            for weight in (0.6, 1)
            for mode in ("render", "white", "edges")
        }
    if profile != "eye":
        raise ValueError("Unknown capture profile")
    rows = set()
    for yaw, pitch in ((0, 0), (-45, 0), (45, 0), (90, 0), (0, -20), (0, 20)):
        for weight in (0, 0.25, 0.5, 0.75, 1):
            for mode in ("render", "white"):
                rows.add((yaw, pitch, weight, weight, 0, mode))
    for left, right in ((1, 0), (0, 1), (1, 1)):
        for yaw in (0, -45, 45):
            rows.add((yaw, 0, left, right, 0.6, "render"))
    for yaw in (-60, -30, 30, 60):
        for mode in ("render", "white", "edges"):
            rows.add((yaw, 0, 0, 0, 0, mode))
    for yaw in (0, -45, 45):
        rows.add((yaw, 0, 0, 0, 0, "edges"))
    return rows


def verify_capture(workspace, manifest, expected_source=None, profile="eye"):
    root = Path(workspace).resolve()
    data = json.loads(Path(manifest).read_text())

    def resolve_resource(value):
        if not isinstance(value, str) or not value.startswith("res://"):
            raise ValueError("Expected project resource path")
        path = (root / value[6:]).resolve()
        if not path.is_relative_to(root):
            raise ValueError("Resource escapes workspace")
        return path

    def sha(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    if expected_source is not None and data["source"] != expected_source:
        raise ValueError("Unexpected model source")
    source = sha(resolve_resource(data["source"]))
    if source != data["source_before"] or source != data["source_after"]:
        raise ValueError("Model source hash mismatch")
    captures = data["captures"]
    if len(captures) != len(expected_views(profile)):
        raise ValueError("Capture count does not match profile")
    actual_schedule = [
        tuple(
            c["requested"][k]
            for k in ("yaw", "pitch", "left", "right", "mouth", "mode")
        )
        for c in captures
    ]
    if set(actual_schedule) != expected_views(profile):
        raise ValueError("Capture schedule coverage mismatch")
    size = None
    viewport_size = None
    seen = set()
    for capture in captures:
        image = capture["image"]
        if image in seen or sha(resolve_resource(image)) != capture["sha256"]:
            raise ValueError("Duplicate or changed capture image")
        seen.add(image)
        requested = capture["requested"]
        weights = capture["actual_weights"]
        numeric = weights + [
            requested[k] for k in ("left", "right", "mouth", "yaw", "pitch")
        ]
        if any(
            isinstance(v, bool)
            or not isinstance(v, (int, float))
            or not math.isfinite(v)
            for v in numeric
        ):
            raise ValueError("Expected finite capture values")
        if len(weights) != 3 or any(
            abs(a - requested[key]) > 1e-6
            for a, key in zip(weights, ("left", "right", "mouth"))
        ):
            raise ValueError("Expression weights mismatch")
        view = capture.get("actual_view")
        if (
            not view
            or view["body_visible"] is not False
            or view["hair_visible"] is not False
        ):
            raise ValueError("Missing or non-head-only visibility evidence")
        if any(
            isinstance(view[k], bool)
            or not isinstance(view[k], (int, float))
            or not math.isfinite(view[k])
            for k in ("size", "yaw", "pitch", "projection")
        ):
            raise ValueError("Expected finite view values")
        if view.get("mode") != requested["mode"]:
            raise ValueError("Actual display mode mismatch")
        if view.get("face_visible") is not True:
            raise ValueError("Face is not visible")
        actual_size = view.get("viewport_size")
        if (
            not isinstance(actual_size, list)
            or len(actual_size) != 2
            or any(
                isinstance(v, bool)
                or not isinstance(v, (int, float))
                or not math.isfinite(v)
                or v <= 0
                or int(v) != v
                for v in actual_size
            )
        ):
            raise ValueError("Invalid viewport dimensions")
        if viewport_size is None:
            viewport_size = actual_size
        if actual_size != viewport_size:
            raise ValueError("Viewport dimensions changed")
        png = resolve_resource(image).read_bytes()
        if len(png) < 24 or png[:8] != b"\x89PNG\r\n\x1a\n" or png[12:16] != b"IHDR":
            raise ValueError("Expected PNG capture header")
        if tuple(actual_size) != struct.unpack(">II", png[16:24]):
            raise ValueError("PNG dimensions mismatch")
        pitch = view.get("camera_pitch")
        if (
            isinstance(pitch, bool)
            or not isinstance(pitch, (int, float))
            or not math.isfinite(pitch)
        ):
            raise ValueError("Missing actual camera pitch")
        if abs(pitch - requested["pitch"]) > 1e-4:
            raise ValueError("Actual camera pitch mismatch")
        if view["projection"] != 1:
            raise ValueError("Expected orthographic projection")
        if any(abs(view[key] - requested[key]) > 1e-4 for key in ("yaw", "pitch")):
            raise ValueError("Camera pose mismatch")
        if size is None:
            size = view["size"]
        if size <= 0 or abs(view["size"] - size) > 1e-6:
            raise ValueError("Camera size changed")
    return {
        "captures_verified": len(captures),
        "source_sha256": source,
        "camera_size": size,
        "scope": "Integrity, expression and view state; not aesthetic acceptance",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--expected-source", required=True)
    parser.add_argument("--profile", choices=("eye", "mouth"), default="eye")
    args = parser.parse_args()
    print(
        json.dumps(
            verify_capture(
                args.workspace, args.manifest, args.expected_source, args.profile
            )
        )
    )

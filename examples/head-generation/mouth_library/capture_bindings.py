"""Known capture implementations; never accept a manifest-supplied code path."""

import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_bindings(root, data, profile="mouth"):
    if profile not in ("mouth", "ear"):
        raise ValueError("Unknown mouth capture profile")
    installed = Path(root) / "showcase_extensions"
    runtime = HERE / "runtime"
    bindings = {
        "schedule_sha256": ("mouth_capture_schedule.json", runtime / "mouth_capture_schedule.json"),
        "controller_sha256": ("researched_mouth_review.gd", runtime / "mouth_review.gd"),
        "probe_sha256": ("researched_mouth_probe.gd", runtime / "mouth_probe.gd"),
    }
    if profile == "ear":
        ears = HERE.parent / "ear_library"
        bindings["probe_sha256"] = ("ear_motion_probe.gd", ears / "ear_motion_probe.gd")
        # The ear entry contains the complete capture implementation. Its
        # presentation helper and entry hash are recorded before/after capture,
        # not inferred merely from whichever files exist during verification.
        helper_hash = digest(ears / "ear_capture_view.gd")
        if (
            digest(installed / "ear_capture_view.gd") != helper_hash
            or data.get("capture_view_before_sha256") != helper_hash
            or data.get("capture_view_after_sha256") != helper_hash
            or data.get("probe_before_sha256") != digest(ears / "ear_motion_probe.gd")
        ):
            raise ValueError("Ear capture-time implementation identity mismatch")
    for key, (name, canonical) in bindings.items():
        expected = digest(canonical)
        if data.get(key) != expected or digest(installed / name) != expected:
            raise ValueError("Capture implementation identity mismatch: " + key)

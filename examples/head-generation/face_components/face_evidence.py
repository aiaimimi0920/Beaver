"""Bind face design, frozen derivation and actual emitted calibration."""

import copy
import hashlib
import json
from face_selection import module


def validate_face_report(snapshot, report, style, frozen):
    design = snapshot.get("face_design")
    record = report.get("face_design")
    analytic = report.get("facial_surface", {}).get("face_design")
    if design is None:
        if (
            record is not None or analytic is not None
            or style.get("face_design") is not None or frozen
        ):
            raise ValueError("Unexpected face design without composition binding")
        return None
    if (
        not isinstance(record, dict) or analytic != record
        or style.get("face_design") != design
    ):
        raise ValueError("Face composition/report/style mismatch")
    expected_hashes = snapshot.get("face_source_sha256")
    if (
        not isinstance(expected_hashes, dict)
        or set(expected_hashes) != {"face_design.py"}
        or record.get("source_sha256") != expected_hashes
    ):
        raise ValueError("Missing face source identity")
    if (
        not isinstance(frozen, dict)
        or set(frozen) != set(expected_hashes)
        or any(
            not isinstance(source, str)
            or hashlib.sha256(source.encode()).hexdigest() != expected_hashes[name]
            for name, source in frozen.items()
        )
    ):
        raise ValueError("Frozen face design source changed")
    before = copy.deepcopy(style)
    before.pop("face_design", None)
    c = before["calibration"]
    c.pop("researched_chin_underside_lift", None)
    fields = (
        ("width_stations", "retained_width_stations"),
        ("chin_return_lift", "retained_chin_return_lift"),
        ("continuous_front_profile", "retained_front_profile"),
        ("cross_sections", "retained_cross_sections"),
    )
    for key, old_key in fields:
        if old_key not in record:
            raise ValueError("Missing retained face calibration")
        c[key] = copy.deepcopy(record[old_key])
    # Execute only the trusted versioned validator, never embedded reference code.
    expected_style, expected = module().apply_design(before, design)
    expected["source_sha256"] = expected_hashes
    if record != expected or style != expected_style:
        raise ValueError("Face calibration differs from the selected authored design")
    return {
        "status": "native_face_calibration_bound",
        "scope": "Exact source/calibration binding, not global collision or visual acceptance",
    }


def verify_face_artifacts(snapshot, report, root, prefix, generated):
    root = root.resolve()
    artifacts = {}
    names = ["face_style.json"]
    if snapshot.get("face_design") is not None:
        names.append("facial_surface_preflight.json")
    for name in names:
        path = (root / prefix / name).resolve()
        if snapshot.get("face_design") is None and prefix + name not in generated:
            continue
        if (
            not path.is_relative_to(root) or not path.is_file()
            or generated.get(prefix + name) != hashlib.sha256(path.read_bytes()).hexdigest()
        ):
            raise ValueError("Missing or changed emitted face artifact: " + name)
        artifacts[name] = json.loads(path.read_text())
    frozen = artifacts.get("facial_surface_preflight.json", {}).get("face_library_sources", {})
    return validate_face_report(snapshot, report, artifacts.get("face_style.json", {}), frozen)

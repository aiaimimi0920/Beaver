"""Bind researched ears to emitted calibration and retained source identities."""

import copy
import hashlib
import json
from ear_selection import module


def validate_ear_report(snapshot, report, style, frozen):
    design = snapshot.get("ear_design")
    record = report.get("ear_design")
    analytic = report.get("facial_surface", {}).get("ear_design")
    if design is None:
        if record is not None or analytic is not None or style.get("ear_design") is not None or frozen:
            raise ValueError("Unexpected ear design without composition binding")
        return None
    if not isinstance(record, dict) or record != analytic or style.get("ear_design") != design:
        raise ValueError("Ear composition/report/style mismatch")
    hashes = snapshot.get("ear_source_sha256")
    if not isinstance(hashes, dict) or set(hashes) != {"ear_design.py", "ear_profile.py"}:
        raise ValueError("Missing ear source identity")
    if record.get("source_sha256") != hashes or not isinstance(frozen, dict) or set(frozen) != set(hashes):
        raise ValueError("Ear source identity mismatch")
    if any(not isinstance(source, str) or hashlib.sha256(source.encode()).hexdigest() != hashes[name]
           for name, source in frozen.items()):
        raise ValueError("Frozen ear source changed")
    before = copy.deepcopy(style)
    before.pop("ear_design", None)
    folds = record.get("retained_fold_strengths")
    keys = {"ear_inner_fold_strength", "ear_inner_fork_strength", "ear_lobe_curl_strength"}
    if not isinstance(folds, dict) or set(folds) != keys:
        raise ValueError("Missing retained ear fold calibration")
    before.update(folds)
    expected_style, expected = module().apply_design(before, design)
    expected["source_sha256"] = hashes
    if expected != record or expected_style != style:
        raise ValueError("Ear calibration differs from selected design")
    return {"status": "native_ear_calibration_bound",
            "scope": "Source and calibration binding only; not visual or collision acceptance"}


def verify_ear_artifacts(snapshot, report, root, prefix, generated):
    root = root.resolve()
    artifacts = {}
    names = ["face_style.json"]
    if snapshot.get("ear_design") is not None:
        names.append("facial_surface_preflight.json")
    for name in names:
        path = (root / prefix / name).resolve()
        if snapshot.get("ear_design") is None and prefix + name not in generated:
            continue
        if (not path.is_relative_to(root) or not path.is_file()
                or generated.get(prefix + name) != hashlib.sha256(path.read_bytes()).hexdigest()):
            raise ValueError("Missing or changed emitted ear artifact: " + name)
        artifacts[name] = json.loads(path.read_text())
    frozen = artifacts.get("facial_surface_preflight.json", {}).get("ear_library_sources", {})
    return validate_ear_report(snapshot, report, artifacts.get("face_style.json", {}), frozen)

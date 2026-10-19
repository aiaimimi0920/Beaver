"""Prepare a read-only six-slot work plan; never launch native tools or accept a model."""

import argparse
import json
from pathlib import Path

from compose import HERE, SLOTS, resolve
from inventory import family_inventory


def evidence_records():
    records = []
    for name in ("native-evidence.json", "component-variants-evidence.json"):
        value = json.loads((HERE / name).read_text())
        records.append({**value, "evidence_file": name})
    for name in (
        "cross-combination-evidence.json",
        "brow-family-evidence.json",
        "nose-variant-evidence.json",
        "ear-family-evidence.json",
    ):
        for value in json.loads((HERE / name).read_text())["candidates"]:
            records.append({**value, "evidence_file": name})
    return records


def build_plan(selection, fit_policy="legacy", records=None):
    snapshot = resolve(selection, fit_policy)
    matches = []
    for record in evidence_records() if records is None else records:
        if record.get("selection") != selection:
            continue
        exact = record.get("composition_sha256") == snapshot["composition_sha256"]
        matches.append(
            {
                "candidate": record.get("candidate"),
                "recorded_fit_policy": record.get("fit_policy", "legacy"),
                "evidence_file": record.get("evidence_file"),
                "match": "exact_snapshot" if exact else "historical_selection_only",
                "composition_sha256": record.get("composition_sha256"),
                "glb_sha256": record.get("glb_sha256"),
                "meaning": "Recorded evidence only; not verification of a fresh output or current generator",
            }
        )
    failure = json.loads((HERE / "cross-combination-evidence.json").read_text())[
        "failed_candidate"
    ]
    warnings = []
    if (
        failure.get("selection") == selection
        and failure.get("fit_policy") == fit_policy
    ):
        warnings.append(
            {
                "candidate": failure["candidate"],
                "reason": failure["reason"],
                "action": "Known native failure for this selection/policy; review explicit fit alternative before spending on regeneration. No policy changed automatically.",
            }
        )
    catalog = json.loads((HERE / "catalog.json").read_text())
    return {
        "schema_version": 1,
        "snapshot": snapshot,
        "family_inventory": family_inventory(),
        "available_option_counts": {slot: len(catalog[slot]) for slot in SLOTS},
        "count_scope": "Selectable engineering options, not completed research families or tested combinations",
        "recorded_evidence": matches,
        "compatibility_warnings": warnings,
        "current_output_status": "not_generated",
        "steps": [
            {
                "stage": "generate",
                "status": "pending",
                "entry": "bundle.py",
                "required": "Fresh combo prefix and workspace; run resulting request through Beaver blender.start and retain successful receipt",
            },
            {
                "stage": "sdf",
                "status": "pending",
                "entry": "sdf_request.py",
                "required": "Successful generation receipt bound to exact report and source assets; retain successful SDF receipt",
            },
            {
                "stage": "capture",
                "status": "pending",
                "entry": "../eye_presets/verify_capture.py",
                "required": "Native 84-view eye and 18-view mouth manifests with actual poses and unchanged GLB hashes",
            },
            {
                "stage": "review",
                "status": "pending",
                "required": "Inspect full face, front/oblique/profile, white/wireframe, closure and mouth views; record defects and acceptance scope",
            },
        ],
        "next_action": (
            "Review known failure and explicit fit alternative before generation; selected policy is unchanged"
            if warnings
            else "Prepare fresh generation request; historical evidence never skips native validation"
        ),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--selection", type=Path, required=True)
    parser.add_argument(
        "--fit-policy", choices=("legacy", "skin-seated-075"), default="legacy"
    )
    args = parser.parse_args()
    print(
        json.dumps(
            build_plan(json.loads(args.selection.read_text()), args.fit_policy),
            ensure_ascii=False,
            indent=2,
        )
    )

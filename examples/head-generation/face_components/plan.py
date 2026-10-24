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
        "nose-template-evidence.json",
        "ear-family-evidence.json",
        "mouth-variant-evidence.json",
        "mouth-template-evidence.json",
        "chin-variant-evidence.json",
        "template-pilot-evidence.json",
        "broad-chin-evidence.json",
    ):
        for value in json.loads((HERE / name).read_text())["candidates"]:
            records.append({**value, "evidence_file": name})
    name = "../brow_library/native-evidence.json"
    for value in json.loads((HERE / name).read_text())["samples"]:
        records.append({**value, "evidence_file": name})
    name = "../nose_library/native-evidence.json"
    noses = json.loads((HERE / name).read_text())
    for value in noses["samples"] + noses["cross_combinations"]:
        records.append({**value, "evidence_file": name})
    name = "../mouth_library/native-evidence.json"
    mouths = json.loads((HERE / name).read_text())
    for value in mouths["samples"] + mouths["cross_combinations"]:
        records.append({**value, "evidence_file": name})
    name = "../face_library/native-evidence.json"
    faces = json.loads((HERE / name).read_text())
    for value in faces["samples"] + faces["cross_combinations"]:
        records.append({**value, "evidence_file": name})
    return records


def build_plan(
    selection,
    fit_policy="legacy",
    records=None,
    nose_overrides=None,
    mouth_overrides=None,
    face_overrides=None,
    ear_overrides=None,
    brow_overrides=None,
    sampling_overrides=None,
):
    snapshot = resolve(
        selection,
        fit_policy,
        brow_overrides=brow_overrides,
        nose_overrides=nose_overrides,
        mouth_overrides=mouth_overrides,
        face_overrides=face_overrides,
        ear_overrides=ear_overrides,
        sampling_overrides=sampling_overrides,
    )
    matches = []
    for record in evidence_records() if records is None else records:
        if record.get("selection") != selection:
            continue
        exact = record.get("composition_sha256") == snapshot["composition_sha256"]
        matches.append(
            {
                "candidate": record.get("candidate"),
                "recorded_generation_status": record.get(
                    "generation_status", "not_recorded"
                ),
                "recorded_visual_status": record.get("visual_status", "not_recorded"),
                "recorded_sample_status": record.get("status", "not_recorded"),
                "recorded_scope": record.get("scope", "not_recorded"),
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
        "topology_policy": {
            "stage": "shared source sampling, never per-character GLB repair",
            "default": "graded mouth/chin/subnasal layout with shared boundary transitions",
            "preserve": ["feature boundary rays", "eye and mouth motion", "silhouette"],
            "measure": ["region triangles per surface area", "adjacent area ratios", "triangle quality"],
            "review": "Compare matched wireframe and white views; exceptions need local shape/deformation evidence, not uniform density everywhere",
        },
        "steps": [
            {
                "stage": "generate",
                "status": "pending",
                "entry": "bundle.py",
                "required": "Fresh combo prefix and workspace; retain companion .source-plan.json; run generation through Beaver blender.start and retain successful receipt",
            },
            {
                "stage": "source_packaging",
                "status": "pending",
                "entry": "../source_layout/pack_request.py",
                "required": "Use the companion source plan and successful generation receipt; execute the packaging request through Beaver and retain its separate successful native receipt",
            },
            {
                "stage": "sdf",
                "status": "pending",
                "entry": "sdf_request.py",
                "required": "Successful generation and source-packaging receipts bound to exact report and sources; pass --source-packaging-receipt and retain successful SDF receipt",
            },
            {
                "stage": "capture",
                "status": "pending",
                "entry": "../eye_presets/verify_capture.py",
                "required": "Native 84-view eye and 18-view mouth manifests; researched mouth additionally requires the 92-view mouth-motion manifest with actual poses and unchanged GLB hashes",
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
    parser.add_argument(
        "--nose-controls", type=Path, help="Bounded researched-nose overrides JSON"
    )
    parser.add_argument(
        "--mouth-controls", type=Path, help="Bounded researched-mouth overrides JSON"
    )
    parser.add_argument("--brow-controls", type=Path)
    parser.add_argument("--face-controls", type=Path)
    parser.add_argument("--ear-controls", type=Path)
    parser.add_argument("--sampling-controls", type=Path, help="Explicit first-pass lash sampling JSON")
    args = parser.parse_args()
    print(
        json.dumps(
            build_plan(
                json.loads(args.selection.read_text()),
                args.fit_policy,
                sampling_overrides=(json.loads(args.sampling_controls.read_text()) if args.sampling_controls else None),
                brow_overrides=(json.loads(args.brow_controls.read_text()) if args.brow_controls else None),
                face_overrides=(json.loads(args.face_controls.read_text()) if args.face_controls else None),
                ear_overrides=(json.loads(args.ear_controls.read_text()) if args.ear_controls else None),
                mouth_overrides=(
                    json.loads(args.mouth_controls.read_text())
                    if args.mouth_controls
                    else None
                ),
                nose_overrides=(
                    json.loads(args.nose_controls.read_text())
                    if args.nose_controls
                    else None
                ),
            ),
            ensure_ascii=False,
            indent=2,
        )
    )

"""Resolve six explicit component slots without changing retained recipes."""

import copy
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
EYES = HERE.parent / "eye_presets"
SURFACE = HERE.parent / "facial_surface"
SLOTS = ("eyes", "brows", "mouth", "nose", "ears", "face")
BROW_KEYS = {
    "brow_height_offset",
    "brow_rise_offset",
    "brow_thickness",
    "brow_arch",
    "brow_fullness_offset",
    "brow_softness",
    "brow_head_rounding",
    "brow_angularity",
    "brow_peak_position",
    "brow_band_fullness",
}


def digest(value):
    return hashlib.sha256(
        json.dumps(
            value, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode()
    ).hexdigest()


def resolve(selection, fit_policy="legacy"):
    if fit_policy not in ("legacy", "skin-seated-075"):
        raise ValueError("Unknown explicit fit policy")
    if not isinstance(selection, dict) or set(selection) != set(SLOTS):
        raise ValueError("Choose exactly eyes, brows, mouth, nose, ears and face")
    catalog = json.loads((HERE / "catalog.json").read_text())
    for slot in SLOTS:
        if not isinstance(selection[slot], str) or selection[slot] not in catalog[slot]:
            raise ValueError("Unknown component selection: " + slot)
    eye_id = catalog["eyes"][selection["eyes"]]
    if eye_id == "baseline-795":
        spec = json.loads((SURFACE / "face-baseline-795.json").read_text())
        spec["eye_parameters"] = json.loads(
            (SURFACE / "emerald-baseline-795-eyes.json").read_text()
        )
        guards = json.loads((EYES / "clean-downturned-810.json").read_text())
        spec["baseline_files_sha256"] = guards["baseline_files_sha256"]
    else:
        spec = json.loads((EYES / (eye_id + ".json")).read_text())
    for name, expected in spec["baseline_files_sha256"].items():
        if hashlib.sha256((SURFACE / name).read_bytes()).hexdigest() != expected:
            raise ValueError("Preserved baseline changed: " + name)
    parameters = copy.deepcopy(spec["eye_parameters"])
    brow = catalog["brows"][selection["brows"]]
    if set(brow) != BROW_KEYS:
        raise ValueError("Brow preset must own all and only brow parameters")
    parameters.update(brow)
    if fit_policy == "skin-seated-075":
        parameters["lateral_guide_regularization"] = 0.75
    owned_keys = {
        "mouth": {
            "mouth_width_scale",
            "upper_lip_relief_scale",
            "upper_lip_lateral_power",
            "lower_lip_relief_scale",
            "upper_lip_bow_height",
        },
        "nose": {
            "nose_bridge_scale",
            "nose_bridge_length_scale",
            "nose_tip_projection_scale",
        },
        "ears": {
            "ear_span_scale",
            "ear_tip_length_scale",
            "ear_point_strength",
            "ear_tip_lift",
        },
        "face": {"face_width", "jaw_width", "chin_width", "chin_softness"},
    }
    for slot, allowed in owned_keys.items():
        values = catalog[slot][selection[slot]]
        if not isinstance(values, dict) or set(values) - allowed:
            raise ValueError("Component overrides another slot: " + slot)
    mouth = catalog["mouth"][selection["mouth"]]
    components = {
        **catalog["nose"][selection["nose"]],
        **catalog["ears"][selection["ears"]],
    }
    if "mouth_width_scale" in mouth:
        components["mouth_width_scale"] = mouth["mouth_width_scale"]
    intents = json.loads((HERE / "authoring-intents.zh-CN.json").read_text())
    if set(intents) != set(SLOTS) or any(
        set(intents[k]) != set(catalog[k]) for k in SLOTS
    ):
        raise ValueError("Authoring intents must cover exactly the component catalog")
    selected_intents = {slot: intents[slot][selection[slot]] for slot in SLOTS}
    if any(
        not isinstance(value, str) or not value.strip()
        for value in selected_intents.values()
    ):
        raise ValueError("Each selected component requires a written intent")
    resolved = {
        "schema_version": 1,
        "selection": copy.deepcopy(selection),
        "fit_policy": fit_policy,
        "authoring_intents": selected_intents,
        "intent_catalog_sha256": digest(intents),
        "eye_source_id": eye_id,
        "surface_parameters": spec["surface_parameters"],
        "eye_parameters": parameters,
        "silhouette_parameters": catalog["face"][selection["face"]],
        "lip_parameters": {k: v for k, v in mouth.items() if k != "mouth_width_scale"},
        "component_parameters": components,
        "style_profile": spec["style_profile"],
        "blink": spec["blink"],
        "shading_policy": spec["shading_policy"],
        "reviewed_support_bound_m": spec["reviewed_support_bound_m"],
        "catalog_sha256": digest(catalog),
        "source_recipe_sha256": digest(spec),
        "scope": "Head only; lashes/iris follow eye family; component alternatives remain experimental",
        "acceptance": "New combination requires native generation, SDF and visual checks",
    }
    return {**resolved, "composition_sha256": digest(resolved)}


def verify_snapshot(snapshot):
    if not isinstance(snapshot, dict):
        raise ValueError("Expected composition snapshot")
    unsigned = {k: v for k, v in snapshot.items() if k != "composition_sha256"}
    if digest(unsigned) != snapshot.get("composition_sha256"):
        raise ValueError("Composition snapshot changed")
    if set(snapshot.get("selection", {})) != set(SLOTS):
        raise ValueError("Incomplete component selection")
    return snapshot

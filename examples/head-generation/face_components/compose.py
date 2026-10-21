"""Resolve six explicit component slots without changing retained recipes."""

import copy
import hashlib
import importlib.util
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


def resolved_brow_design(preset, overrides):
    design = preset.get("design")
    if design is None:
        if overrides is not None:
            raise ValueError("Brow controls require an explicit researched preset")
        return None
    if isinstance(design, str):
        designs = json.loads((HERE.parent / "brow_library/presets.json").read_text())
        if design not in designs:
            raise ValueError("Unknown researched brow design reference")
        design = designs[design]
    spec = importlib.util.spec_from_file_location(
        "component_brow_design", HERE.parent / "brow_library/brow_design.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    if overrides is not None and (
        not isinstance(overrides, dict) or set(overrides) - set(module.LIMITS)
    ):
        raise ValueError("Brow controls may only override bounded geometry parameters")
    return module.validate({**design, **(overrides or {})})


def resolve(
    selection,
    fit_policy="legacy",
    brow_overrides=None,
    nose_overrides=None,
    mouth_overrides=None,
    face_overrides=None,
):
    if fit_policy not in ("legacy", "skin-seated-075"):
        raise ValueError("Unknown explicit fit policy")
    if not isinstance(selection, dict) or set(selection) != set(SLOTS):
        raise ValueError("Choose exactly eyes, brows, mouth, nose, ears and face")
    catalog = json.loads((HERE / "catalog.json").read_text())
    for slot in SLOTS:
        if not isinstance(selection[slot], str) or selection[slot] not in catalog[slot]:
            raise ValueError("Unknown component selection: " + slot)
    from nose_selection import resolve_nose

    nose_design, nose_hashes = resolve_nose(
        catalog["nose"][selection["nose"]], nose_overrides
    )
    from mouth_selection import resolve_mouth

    mouth_design, mouth_hashes = resolve_mouth(
        catalog["mouth"][selection["mouth"]], mouth_overrides
    )
    from face_selection import resolve_face

    face_design, face_hashes = resolve_face(catalog["face"][selection["face"]], face_overrides)
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
    if set(brow) - {"design"} != BROW_KEYS:
        raise ValueError("Brow preset must own all and only brow parameters")
    design = resolved_brow_design(brow, brow_overrides)
    parameters.update({k: v for k, v in brow.items() if k != "design"})
    if fit_policy == "skin-seated-075":
        parameters["lateral_guide_regularization"] = 0.75
    owned_keys = {
        "mouth": {
            "design",
            "mouth_width_scale",
            "upper_lip_relief_scale",
            "upper_lip_lateral_power",
            "lower_lip_relief_scale",
            "upper_lip_bow_height",
            "template_id",
        },
        "nose": {
            "nose_bridge_scale",
            "nose_bridge_length_scale",
            "nose_tip_projection_scale",
            "template_id",
            "design",
        },
        "ears": {
            "ear_span_scale",
            "ear_tip_length_scale",
            "ear_point_strength",
            "ear_tip_lift",
        },
        "face": {
            "design",
            "face_width",
            "jaw_width",
            "chin_width",
            "chin_softness",
            "template_id",
        },
    }
    for slot, allowed in owned_keys.items():
        values = catalog[slot][selection[slot]]
        if not isinstance(values, dict) or set(values) - allowed:
            raise ValueError("Component overrides another slot: " + slot)
    mouth = catalog["mouth"][selection["mouth"]]
    components = {
        **{
            k: v
            for k, v in catalog["nose"][selection["nose"]].items()
            if k not in {"template_id", "design"}
        },
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
        "silhouette_parameters": {
            k: v
            for k, v in catalog["face"][selection["face"]].items()
            if k not in {"template_id", "design"}
        },
        "lip_parameters": {
            k: v
            for k, v in mouth.items()
            if k not in ("mouth_width_scale", "template_id", "design")
        },
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
    templates = []
    registry = json.loads((HERE / "family-templates.json").read_text())
    for slot in ("face", "mouth", "nose"):
        template_id = catalog[slot][selection[slot]].get("template_id")
        if template_id is not None:
            if template_id not in registry or registry[template_id]["slot"] != slot:
                raise ValueError("Unknown component template: " + slot)
            templates.append(copy.deepcopy(registry[template_id]))
    if templates:
        resolved["family_templates"] = templates
    if design is not None:
        resolved["brow_design"] = design
        resolved["brow_source_sha256"] = {
            name: hashlib.sha256(
                (HERE.parent / "brow_library" / name)
                .read_text(encoding="utf-8")
                .encode("utf-8")
            ).hexdigest()
            for name in (
                "brow_design.py",
                "brow_shapes.py",
                "brow_mesh.py",
                "brow_source.py",
            )
        }
    if nose_design is not None:
        resolved["nose_design"] = nose_design
        resolved["nose_source_sha256"] = nose_hashes
    if mouth_design is not None:
        resolved["mouth_design"] = mouth_design
        resolved["mouth_source_sha256"] = mouth_hashes
    if face_design is not None:
        resolved["face_design"] = face_design
        resolved["face_source_sha256"] = face_hashes
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

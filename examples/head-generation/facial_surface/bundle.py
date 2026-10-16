"""Package experimental fresh geometry; requires an explicitly reviewed bound."""

import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import sys
import tempfile
from preflight import prepare
from source_speed import skip_zero_weight_sections

HERE = Path(__file__).resolve().parent
EYES = HERE.parent / "semantic_eyes"


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Inherited source contract changed: " + old[:60])
    return source.replace(old, new, 1)


PROFILE_STYLE = {
    "accent_band_roll_m": 0.0006,
    "accent_cross_arch_ratio": 0.0,
    "accent_cross_section_profile": "rounded",
    "accent_final_crown_relief_m": 0.0005,
    "accent_main_lower_flank_drop_m": 0.0001,
    "accent_main_lower_flank_recede_m": 0.0002,
    "accent_outer_tint_rgb": [0.045, 0.023, 0.035],
    "accent_section_soft_curve": 1.0,
    "brow_arch_m": 0.0018,
    "brow_rgb": [0.055, 0.025, 0.04],
    "iris_crescent_rgb": [0.52, 0.91, 0.69],
    "iris_dark_rgb": [0.025, 0.11, 0.1],
    "iris_light_rgb": [0.28, 0.62, 0.46],
    "iris_rim_rgb": [0.014, 0.04, 0.035],
    "lash_rgb": [0.022, 0.013, 0.022],
    "lower_lash_width_scale": 0.4,
}
PROFILE_CALIBRATION = {
    "iris_concavity": 0.01,
    "iris_rim_depth": -0.0348,
    "pupil_half_height": 0.0243,
    "pupil_half_width": 0.014875000000000001,
}


def apply_style_profile(style, name):
    """Opt-in original eye-art direction; the inherited baseline stays intact."""
    if name is None:
        return json.loads(json.dumps(style))
    if name != "clean_emerald":
        raise ValueError("Unknown face style profile: " + str(name))
    result = json.loads(json.dumps(style))
    result.update(PROFILE_STYLE)
    result["calibration"].update(PROFILE_CALIBRATION)
    return result


def build_request(
    parameters, eyes, prefix, workspace, reviewed_support_bound_m, style_profile=None
):
    if (
        isinstance(reviewed_support_bound_m, bool)
        or not isinstance(reviewed_support_bound_m, (int, float))
        or not math.isfinite(reviewed_support_bound_m)
        or reviewed_support_bound_m <= 0
    ):
        raise ValueError(
            "Supply the explicitly reviewed positive support bound in meters"
        )
    sys.path.insert(0, str(EYES))
    try:
        spec = importlib.util.spec_from_file_location(
            "facial_eye_builder", EYES / "bundle.py"
        )
        builder = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(builder)
        recipe = builder.RECIPE
        baseline_style = apply_style_profile(
            json.loads((recipe / "face_style.json").read_text()), style_profile
        )
        style = builder.adapt_style(baseline_style, builder.validate(eyes))
        face = (recipe / "face.py").read_text()
        data, report = prepare(face, style["calibration"], parameters)
        bound = max(
            report["surface"]["maximum_depth_displacement_m"],
            report["surface"]["aperture_rim_displacement_bound_m"],
            data["ocular_plane"]["continuous_plane_displacement_bound_m"],
        )
        if bound > reviewed_support_bound_m:
            raise ValueError(
                f"Conservative support bound {bound:.9g}m exceeds explicitly reviewed {reviewed_support_bound_m:.9g}m"
            )
        face = skip_zero_weight_sections(face)
        face = once(face, "def base_depth(x, y):", "def inherited_base_depth(x, y):")
        face = once(face, "def rim_depth(x, y):", "def inherited_rim_depth(x, y):")
        wrappers = """def base_depth(x, y):
    return inherited_base_depth(x, y) + base_delta(x, y, C, curve, DATA)


def rim_depth(x, y, check_support=True):
    old = inherited_rim_depth(x, y)
    move = base_delta(x, y, C, curve, DATA)
    move += DATA['parameters']['orbit_shallowing'] * (inherited_base_depth(x, y) - old)
    if check_support and abs(move) > REVIEWED_BOUND:
        raise ValueError('Actual rim/support call exceeds reviewed displacement bound')
    return old + move


def depth(x, y):"""
        face = once(face, "def depth(x, y):", wrappers)
        face = once(
            face,
            "    return raw + (rim_depth(x, y) - raw) * weight",
            "    result = raw + (rim_depth(x, y, False) - raw) * weight\n"
            "    original = inherited_base_depth(x, y) + (inherited_rim_depth(x, y) - inherited_base_depth(x, y)) * weight\n"
            "    if abs(result - original) > REVIEWED_BOUND:\n"
            "        raise ValueError('Actual depth call exceeds reviewed displacement bound')\n"
            "    return result",
        )
        face = (
            "from facial_surface_runtime import base_delta, ocular_delta, DATA, REVIEWED_BOUND, blend_medial_normal\n"
            + face
        )
        socket = (recipe / "eye_socket.py").read_text()
        socket = once(
            socket,
            "    )\n\n\ndef iris_grid_point",
            "    ) + ocular_delta(x, y, DATA)\n\n\ndef iris_grid_point",
        )
        socket = once(
            socket,
            "            if accent:",
            "            z += ocular_delta(x, y, DATA)\n            if accent:",
        )
        with tempfile.TemporaryDirectory() as directory:
            snapshot = Path(directory)
            for path in recipe.iterdir():
                if path.is_file():
                    (snapshot / path.name).write_bytes(path.read_bytes())
            (snapshot / "face_style.json").write_text(
                json.dumps(baseline_style, indent=2)
            )
            pipeline = (recipe / "pipeline.py").read_text()
            pipeline = once(
                pipeline,
                "                if index in ear_root_ids:\n                    normal = normal.lerp",
                "                normal = blend_medial_normal(normal, normals[index], region, side, data.vertices[index].co.z, C, DATA)\n"
                "                if index in ear_root_ids:\n                    normal = normal.lerp",
            )
            (snapshot / "pipeline.py").write_text(pipeline)
            (snapshot / "face.py").write_text(face)
            (snapshot / "eye_socket.py").write_text(socket)
            builder.RECIPE = snapshot
            request = builder.build_request(eyes, prefix, workspace)
    finally:
        sys.path.remove(str(EYES))
    module = (HERE / "runtime_field.py").read_text()
    preamble = 'import types, sys\n_facial = types.ModuleType("facial_surface_runtime")\nsys.modules["facial_surface_runtime"] = _facial\n'
    preamble += f'exec(compile({module!r}, "facial_surface_runtime.py", "exec"), _facial.__dict__)\n'
    preamble += f"_facial.DATA = {data!r}\n_facial.REVIEWED_BOUND = {reviewed_support_bound_m!r}\n"
    request["code"] = preamble + request["code"]
    report.update(
        parameters=data["parameters"],
        style_profile=style_profile,
        reviewed_support_bound_m=reviewed_support_bound_m,
        reference_old_count_guard="Reference .5/.05/.25 with target670 eyes did not pass; experimental diagnostic exception only",
        reference_experiment_parameters=dict(
            axial_smoothing=0.5, cheek_roundness=0.05, orbit_shallowing=0.25
        ),
        reference_exterior_shoulder=dict(
            x_m=0.04,
            height_fraction=[0.2278, 0.2364],
            excursion_m=2.1913303e-6,
            tangent_degrees=[-0.0928939, 0.0007467],
        ),
        reference_domain_note="With target670 eyes, second sampled extremum at x=.06,h=.4354–.4373 is inside absent eye aperture",
        ear_policy="Unchanged topology and outer correction jets; exact final connectivity must be measured",
        mouth_policy="Inherited additive motion regenerated on new neutral support; posed clearances unverified",
    )
    request["code"] += f'report["facial_surface"] = {report!r}\n'
    request[
        "code"
    ] += f'Path(beaver_output({prefix+"generation_report.json"!r})).write_text(json.dumps(report, indent=2))\n'
    artifacts = {
        f"facial_surface_{path.name}": path.read_text()
        for path in HERE.glob("*.py")
        if not path.name.startswith("test_") and path.name != "source_speed.py"
    }
    for name in ("fit.py", "curve_profile.py", "diagnostics.py"):
        artifacts["coherent_dependency_" + name] = (
            HERE.parent / "coherent_surface" / name
        ).read_text()
    artifacts["facial_surface_preflight.json"] = json.dumps(
        dict(
            data=data,
            report=report,
            packager_helpers={
                "source_speed.py": (HERE / "source_speed.py").read_text()
            },
        ),
        indent=2,
    )
    artifacts["facial_surface_source_manifest.json"] = json.dumps(
        {
            name: hashlib.sha256(content.encode()).hexdigest()
            for name, content in artifacts.items()
        },
        indent=2,
    )
    for name, content in artifacts.items():
        path = prefix + name
        request["code"] += f"Path(beaver_output({path!r})).write_text({content!r})\n"
        previous = Path(workspace) / path
        request["outputs"].append(
            dict(
                path=path,
                expectedSha256=(
                    hashlib.sha256(previous.read_bytes()).hexdigest()
                    if previous.is_file()
                    else None
                ),
            )
        )
    paths = [output["path"] for output in request["outputs"]]
    if len(paths) > 64 or len(set(paths)) != len(paths):
        raise ValueError("Native request requires at most 64 unique retained outputs")
    compile(request["code"], "<experimental facial surface>", "exec")
    return request


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parameters", required=True)
    parser.add_argument("--eyes", default="{}")
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--reviewed-support-bound-m", type=float, required=True)
    parser.add_argument("--style-profile", choices=["clean_emerald"])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            build_request(
                json.loads(args.parameters),
                json.loads(args.eyes),
                args.prefix,
                args.workspace,
                args.reviewed_support_bound_m,
                args.style_profile,
            )
        )
    )

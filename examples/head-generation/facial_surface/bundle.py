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
from source_optional_bridges import adapt_bridge_pair_source, adapt_ear_boundary_source

HERE = Path(__file__).resolve().parent
EYES = HERE.parent / "semantic_eyes"


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Inherited source contract changed: " + old[:60])
    return source.replace(old, new, 1)


from source_styles import apply_style_profile


def build_request(
    parameters,
    eyes,
    prefix,
    workspace,
    reviewed_support_bound_m,
    style_profile=None,
    *,
    blink=False,
    silhouette_parameters=None,
    lip_parameters=None,
    normal_parameters=None,
    component_parameters=None,
    family_templates=None,
    brow_design=None,
    nose_design=None,
    mouth_design=None,
    face_design=None,
):
    if (
        silhouette_parameters
        or lip_parameters
        or normal_parameters
        or component_parameters
        or family_templates
        or brow_design
        or nose_design
        or mouth_design
        or face_design
    ) and prefix in {
        "assets/aster/general_face_eyes_795/",
        "assets/aster/general_face_eyes_796/",
    }:
        raise ValueError("Silhouette experiments must use a fresh output prefix")
    if (
        isinstance(reviewed_support_bound_m, bool)
        or not isinstance(reviewed_support_bound_m, (int, float))
        or not math.isfinite(reviewed_support_bound_m)
        or reviewed_support_bound_m <= 0
    ):
        raise ValueError(
            "Supply the explicitly reviewed positive support bound in meters"
        )
    from source_normals import validate as validate_normals, adapt_pipeline

    normal_amount = validate_normals(normal_parameters)
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
        from source_family_templates import (
            apply_family_templates,
            width_change_bound,
            instrument_ear_constraints,
            transport_ear_constraints,
        )

        retained_width_stations = [
            row[:] for row in baseline_style["calibration"]["width_stations"]
        ]
        from source_mouth_templates import (
            split_templates,
            apply_mouth_template,
            adapt_mouth_face,
            adapt_mouth_companions,
        )

        raw_templates = [] if family_templates is None else family_templates
        if not isinstance(raw_templates, list) or any(
            not isinstance(t, dict) for t in raw_templates
        ):
            raise ValueError("Expected component template objects")
        nose_templates = [t for t in raw_templates if t.get("slot") == "nose"]
        face_templates, mouth_templates = split_templates(
            [t for t in raw_templates if t.get("slot") != "nose"]
        )
        from source_nose_templates import (
            apply_nose_template,
            adapt_nose_face,
            adapt_nose_companion,
        )

        baseline_style, family_report = apply_family_templates(
            baseline_style, face_templates, silhouette_parameters
        )
        import source_face_design as face_adapter

        baseline_style, face_design_report = face_adapter.apply(
            baseline_style, face_design, silhouette_parameters, face_templates
        )
        from source_components import apply_components

        baseline_style, component_report = apply_components(
            baseline_style, component_parameters
        )
        baseline_style, nose_template_report = apply_nose_template(
            baseline_style, nose_templates
        )
        import source_nose_design as nose_adapter

        baseline_style, nose_design_report = nose_adapter.apply(
            baseline_style, nose_design, component_parameters, nose_templates
        )
        from source_silhouette import apply_silhouette

        baseline_style, silhouette_report = apply_silhouette(
            baseline_style, silhouette_parameters
        )
        if family_report is not None:
            family_report["final_width_stations"] = baseline_style["calibration"][
                "width_stations"
            ]
            family_report["residual_half_width_change_bound_m"] = (
                0
                if silhouette_report is None
                else silhouette_report["continuous_half_width_change_bound_m"]
            )
            family_report["total_half_width_change_bound_m"] = width_change_bound(
                retained_width_stations,
                baseline_style["calibration"]["width_stations"],
                baseline_style["calibration"]["height_m"],
            )
        from source_lips import apply_lip_relief, adapt_lip_source

        baseline_style, lip_report = apply_lip_relief(baseline_style, lip_parameters)
        baseline_style, mouth_template_report = apply_mouth_template(
            baseline_style, mouth_templates
        )
        import source_mouth_design as mouth_adapter

        baseline_style = mouth_adapter.apply(
            baseline_style,
            mouth_design,
            lip_parameters,
            component_parameters,
            mouth_templates,
        )
        style = builder.adapt_style(baseline_style, builder.validate(eyes))
        face = adapt_lip_source(
            (recipe / "face.py").read_text(), baseline_style["calibration"]
        )
        face = adapt_mouth_face(face, mouth_template_report is not None)
        face = mouth_adapter.module().adapt_face(face, mouth_design)
        face = adapt_nose_face(face, nose_template_report is not None)
        face = nose_adapter.module().adapt_face(face, nose_design)
        from source_upper_warp import adapt_face_source

        analysis_face = adapt_face_source(
            face,
            bool(
                style["calibration"].get("eye_upper_asymmetry", 0)
                or style["calibration"].get("eye_upper_angularity", 0)
                or style["calibration"].get("eye_lower_shoulder", 0)
            ),
        )
        data, report = prepare(
            analysis_face,
            style["calibration"],
            parameters,
            baseline_iris_plane_slope=baseline_style["calibration"]["iris_plane_slope"],
        )
        bound = report["surface"]["conservative_support_displacement_bound_m"]
        if lip_report is not None:
            report["surface"]["support_bound_before_lip_relief_m"] = bound
            bound += lip_report["analytic_depth_change_bound_m"]
            report["surface"]["conservative_support_displacement_bound_m"] = bound
            report["lip_relief"] = lip_report
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
    move += rim_slope_delta(x, C, DATA)
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
            "from facial_surface_runtime import base_delta, ocular_delta, DATA, REVIEWED_BOUND, blend_medial_normal, rim_slope_delta\n"
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
            for companion in ("mouth.py", "textures.py", "crease_rules.py"):
                (snapshot / companion).write_text(
                    adapt_mouth_companions(
                        companion,
                        (recipe / companion).read_text(),
                        mouth_template_report is not None,
                    )
                )
            for name in ("mouth.py", "textures.py", "crease_rules.py"):
                path = snapshot / name
                path.write_text(
                    mouth_adapter.module().adapt_companion(
                        name, path.read_text(), mouth_design
                    )
                )
            for companion in ("nose_refine.py", "nose_fair.py"):
                (snapshot / companion).write_text(
                    nose_adapter.module().companion(
                        companion,
                        adapt_nose_companion(
                            companion,
                            (recipe / companion).read_text(),
                            nose_template_report is not None,
                        ),
                        nose_design,
                    )
                )
            from source_ear_profile import adapt_ear_profile
            from source_ear_join import adapt_upper_join

            (snapshot / "ear_silhouette.py").write_text(
                adapt_upper_join(
                    (recipe / "ear_silhouette.py").read_text(),
                    baseline_style["calibration"].get("ear_point_strength", 0) > 0
                    or baseline_style["calibration"].get("ear_tip_length_scale", 1) != 1
                    or baseline_style["calibration"].get("ear_tip_lift", 0) != 0,
                )
            )

            (snapshot / "ears.py").write_text(
                adapt_ear_profile(
                    (recipe / "ears.py").read_text(), baseline_style["calibration"]
                )
            )
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
            (snapshot / "bridge_pairs.py").write_text(
                adapt_bridge_pair_source((recipe / "bridge_pairs.py").read_text())
            )
            (snapshot / "ear_boundary.py").write_text(
                adapt_ear_boundary_source((recipe / "ear_boundary.py").read_text())
            )
            if family_report is not None:
                from source_outline_trace import instrument_outline

                (snapshot / "shell.py").write_text(
                    instrument_outline((snapshot / "shell.py").read_text())
                )
                if family_report["template"]["id"] == "broad-blunt-chin-v3":
                    from source_outline_fairing import adapt_outline_fairing

                    shell_path = snapshot / "shell.py"
                    shell_path.write_text(adapt_outline_fairing(shell_path.read_text()))
                boundary_path = snapshot / "ear_boundary.py"
                boundary_source = instrument_ear_constraints(boundary_path.read_text())
                if (
                    family_report["template"]["attachment_interface"]
                    == "transported-ear-v2"
                ):
                    boundary_source = transport_ear_constraints(
                        boundary_source, retained_width_stations
                    )
                boundary_path.write_text(boundary_source)
            if face_design_report is not None:
                face_adapter.adapt_sources(snapshot, retained_width_stations)
            if silhouette_report is not None or family_report is not None or face_design_report is not None:
                from source_silhouette import adapt_ear_envelope

                (snapshot / "ear_width.py").write_text(
                    adapt_ear_envelope(
                        (recipe / "ear_width.py").read_text(),
                        retained_width_stations,
                    )
                )
            pipeline = adapt_pipeline(pipeline, normal_amount)
            pipeline = nose_adapter.module().adapt_pipeline(pipeline, nose_design)
            (snapshot / "pipeline.py").write_text(pipeline)
            (snapshot / "face.py").write_text(face)
            (snapshot / "eye_socket.py").write_text(socket)
            builder.RECIPE = snapshot
            request = builder.build_request(
                eyes,
                prefix,
                workspace,
                blink=blink,
                brow_design=brow_design,
                mouth_design=mouth_design,
            )
    finally:
        sys.path.remove(str(EYES))
    module = (HERE / "runtime_field.py").read_text()
    preamble = 'import types, sys\n_facial = types.ModuleType("facial_surface_runtime")\nsys.modules["facial_surface_runtime"] = _facial\n'
    preamble += f'exec(compile({module!r}, "facial_surface_runtime.py", "exec"), _facial.__dict__)\n'
    preamble += f"_facial.DATA = {data!r}\n_facial.REVIEWED_BOUND = {reviewed_support_bound_m!r}\n"
    request["code"] = (
        mouth_adapter.module().preamble(mouth_design)
        + nose_adapter.preamble(nose_design)
        + preamble
        + request["code"]
    )
    profile_scope = (
        "Layered eye art direction plus reduced nose-plane pigment; nose geometry unchanged"
        if style_profile in ("emerald_layered", "emerald_illustrated")
        else "Inherited or clean eye art direction"
    )
    if (
        nose_design_report
        or nose_template_report
        or (component_report and "nose_stations" in component_report["after"])
    ):
        profile_scope = profile_scope.replace(
            "nose geometry unchanged", "nose calibration altered"
        )
    if nose_design_report is not None:
        profile_scope += "; researched nasal geometry active; separate attachment normals excluded from current face SDF"
    if style_profile in ("emerald_layered", "emerald_illustrated"):
        request["code"] += f'report["semantic_eyes"]["scope"] = {profile_scope!r}\n'
    if component_report is not None:
        report["component_calibration"] = component_report
        profile_scope += (
            "; explicit component calibration changes are recorded separately"
        )
    if family_report is not None:
        report["family_template"] = family_report
    if face_design_report is not None:
        report["face_design"] = face_design_report
        request["code"] += f'report["face_design"] = {face_design_report!r}\n'
    if mouth_template_report is not None:
        report["mouth_template"] = mouth_template_report
    if nose_template_report is not None:
        report["nose_template"] = nose_template_report
    if nose_design_report is not None:
        report["nose_design"] = nose_design_report
        request[
            "code"
        ] += f'report["nose_design"] = {{"parameters": {nose_design!r}, "source_sha256": {nose_design_report["source_sha256"]!r}, "attachments": _nose_runtime.NOSE_MESH_REPORT}}\n'
    if mouth_design is not None:
        mouth_record = {
            "parameters": mouth_design,
            "source_sha256": mouth_adapter.module().provenance(),
            "authored_support_envelope_m": style["calibration"]["height_m"]
            * mouth_design["muzzle_projection"],
            "scope": "Authored support field envelope; not baseline displacement, collision or speech proof",
        }
        report["mouth_design"] = mouth_record
        request["code"] += f'report["mouth_design"] = {mouth_record!r}\n'
    if silhouette_report is not None:
        report["silhouette"] = silhouette_report
    if normal_amount:
        report["lower_medial_normals"] = dict(
            amount=normal_amount,
            height_window=[0.05, 0.10, 0.20, 0.23],
            regions=[1, 2],
            scope="Center vertices only, same-region area-weighted normals",
            geometry_displacement_m=0.0,
            validation="Native posed shading not yet verified",
        )
    report.update(
        style_profile_scope=profile_scope,
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
    from source_artifacts import append_artifacts

    append_artifacts(
        request, prefix, workspace, data, report,
        nose_adapter.retained(nose_design),
        {} if mouth_design is None else mouth_adapter.module().retained_sources(),
        {} if face_design is None else {
            "face_design.py": (HERE.parent / "face_library/face_design.py").read_text()
        },
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
    parser.add_argument("--silhouette", default="{}")
    parser.add_argument("--lips", default="{}")
    parser.add_argument("--normals", default="{}")
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--reviewed-support-bound-m", type=float, required=True)
    parser.add_argument(
        "--style-profile",
        choices=["clean_emerald", "emerald_layered", "emerald_illustrated"],
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--blink", action="store_true", help="Experimental independent eyelid closure"
    )
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
                blink=args.blink,
                silhouette_parameters=json.loads(args.silhouette),
                lip_parameters=json.loads(args.lips),
                normal_parameters=json.loads(args.normals),
            )
        )
    )

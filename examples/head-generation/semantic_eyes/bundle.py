"""Package a fresh semantic eye recipe for Beaver; never execute a model here."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
from parameters import adapt_style, validate, adapt_lower_clearance, adapt_brow_source
from source_upper_warp import adapt_upper_sources, HELPERS
from source_lateral_rim import adapt_lateral_rim, adapt_lateral_depth
from source_angular_samples import adapt_angular_samples
from source_return_order import adapt_return_order

HERE = Path(__file__).resolve().parent
RECIPE = HERE.parent / "checkpoints/current"
REQUIRED = ["project.godot", "authoring/head_generation_guide.md"] + [
    "assets/aster/head_recovery_49/" + name
    for name in (
        "aster_head_editable.blend",
        "aster_head.glb",
        "aster_definition.tres",
        "hair_base.png",
        "hair_ilm.png",
    )
]


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Inherited source contract changed: " + old[:70])
    return source.replace(old, new, 1)


def build_request(parameters, output_prefix, workspace, *, blink=False):
    p = validate(parameters)
    if type(blink) is not bool:
        raise ValueError("blink must be a boolean")
    if blink and (
        p["supplemental_lashes"] != 0
        or p["lid_fold"] != 0
        or p["lash_contour_follow"] != 1
    ):
        raise ValueError(
            "Blink requires contour-following main lashes without supplemental lashes or lid fold"
        )
    if not re.fullmatch(r"assets/aster/general_face_eyes_[a-z0-9_]+/", output_prefix):
        raise ValueError("Use assets/aster/general_face_eyes_<case>/")
    root = Path(workspace)
    missing = [name for name in REQUIRED if not (root / name).is_file()]
    if missing:
        raise ValueError("Missing fresh recipe inputs: " + ", ".join(missing))
    spec = importlib.util.spec_from_file_location(
        "semantic_recipe_builder", RECIPE / "bundle.py"
    )
    base = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(base)
    style = adapt_style(json.loads((RECIPE / "face_style.json").read_text()), p)
    # This recipe does not restore coordinates from any accepted final head.
    base.MODULES = [
        m for m in base.MODULES if m not in ("preserve_accepted", "eye_rim_refine")
    ]
    base.MODULES.append("semantic_eye_runtime")
    base.SOURCE_FILES = [
        n
        for n in base.SOURCE_FILES
        if n not in ("preserve_accepted.py", "eye_rim_refine.py")
    ]
    base.SOURCE_FILES.append("semantic_eye_runtime.py")
    pipeline = (RECIPE / "pipeline.py").read_text()
    pipeline = replace_once(
        pipeline, 'OUT = "' + base.OUTPUT_PREFIX + '"', "OUT = " + repr(output_prefix)
    )
    pipeline = replace_once(
        pipeline,
        "        from preserve_accepted import preserve_non_eye\n"
        '        report["accepted_non_eye_preservation"] = preserve_non_eye(data,ROOT,report)\n'
        "        from eye_rim_refine import refine_outer_eye\n"
        '        report["outer_eye_local_depth"] = refine_outer_eye(data)',
        '        report["semantic_eye_final_restore"] = "none; fresh procedural geometry"',
    )
    pipeline = replace_once(
        pipeline,
        "PARTS[:] = stable_part_order(PARTS)",
        "PARTS[:] = stable_part_order(PARTS)\n"
        "semantic_part_ordinals = {part.name: i + 10 for i, part in enumerate(PARTS)}\n"
        "from semantic_eye_runtime import select_components\n"
        f'report["semantic_eye_selection"] = select_components(PARTS, {p!r})\n'
        f'report["semantic_eyes"] = {{"parameters": {p!r}, "native_acceptance": "pending", '
        '"scope": "eye-family only; regenerated procedural non-eye baseline"}',
    )
    pipeline = replace_once(
        pipeline,
        "        region = part_index + 10",
        "        region = semantic_part_ordinals[part.name]",
    )
    if p["short_upper_tuft"] and not p["layered_lashes"]:
        pipeline = replace_once(
            pipeline,
            'report["eye_accent_connections"] = measure_accent_connections()',
            "from semantic_eye_runtime import validate_retained_tuft_roots\n"
            'report["eye_accent_connections"] = validate_retained_tuft_roots(PARTS)',
        )
    elif p["supplemental_lashes"] == 0 and not p["layered_lashes"]:
        pipeline = replace_once(
            pipeline,
            'report["eye_accent_connections"] = measure_accent_connections()',
            'report["eye_accent_connections"] = {"status": "not_applicable", "reason": "all supplemental child parts are removed before delivery; main band obstacle checks remain active"}',
        )
    guard = (HERE.parent / "general_face/definition_guard.py").read_text()
    pipeline = replace_once(
        pipeline,
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
        f'exec(compile({guard!r}, "definition_guard.py", "exec"))\n'
        "require_head_only_definition(definition)\n"
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
    )
    pipeline = replace_once(
        pipeline,
        'tmp = Path(beaver_output(OUT + "aster_head.glb")).parent / "runtime_new.glb"',
        'tmp = Path(beaver_output(OUT + "semantic_face_export.glb"))',
    )
    pipeline = replace_once(
        pipeline, "tmp.unlink()", "# Retain declared same-job fresh Face export."
    )
    pipeline = replace_once(
        pipeline,
        'Path(beaver_output(OUT + "generation_report.json")).write_text(json.dumps(report, indent=2))',
        "from semantic_eye_runtime import final_runtime_evidence\n"
        'report["semantic_eyes"].update(final_runtime_evidence(beaver_output(OUT + "aster_head_runtime_morph.blend")))\n'
        'Path(beaver_output(OUT + "generation_report.json")).write_text(json.dumps(report, indent=2))',
    )
    sources = {
        name: (RECIPE / name).read_text()
        for name in base.SOURCE_FILES
        if name != "semantic_eye_runtime.py"
    }
    sources.update(
        {
            "face_style.json": json.dumps(style, indent=2),
            "pipeline.py": pipeline,
            "semantic_eye_runtime.py": (HERE / "runtime.py").read_text(),
        }
    )
    sources["face.py"] = adapt_brow_source(sources["face.py"], p["brow_head_rounding"])
    if p["layered_lashes"]:
        root_fit = """        delta = target-center
        # Rigid minimax seating preserves the closed six-vertex root section.
        # The final clearance and 0.8 mm connection checks remain mandatory.
        def score(move):
            values = [tree.find_nearest(v.co + move)[3] for v in roots]
            return (max(values), sum(values))
        for step in (.0004, .0002, .0001, .00005, .000025):
            for iteration in range(8):
                choices = [delta.copy()]
                for axis in range(2):
                    for sign in (-1, 1):
                        candidate = delta.copy()
                        candidate[axis] += sign * step
                        if candidate.length < .004:
                            choices.append(candidate)
                best = min(choices, key=score)
                if score(best) >= score(delta):
                    break
                delta = best
"""
        sources["eye_accents.py"] = replace_once(
            sources["eye_accents.py"], "        delta = target-center\n", root_fit
        )
    if p["layered_lashes"]:
        seat = "        lateral_curve = regularize_lateral_sections(PARTS, lateral_sections(), STYLE, obstacle_trees[0][1])"
        slender = """
        for wing in PARTS:
            if not wing.name.startswith("Attached outer lash wing "):
                continue
            columns = {}
            for vertex in wing.data.vertices:
                columns.setdefault(round(abs(vertex.co.x), 8), []).append(vertex)
            for x, column in columns.items():
                t = (x / H - C["eye_center_x"]) / C["eye_half_width"]
                u = max(0.0, min(1.0, (t - 0.65) / 0.2))
                scale = 1.0 - 0.35 * u*u*(3-2*u)
                center = min(v.co.z for v in column)
                for vertex in column:
                    vertex.co.z = center + (vertex.co.z - center) * scale
                    tip_u = max(0.0, min(1.0, (t - 1.0) / 0.1))
                    vertex.co.z += H * 0.012 * STYLE.get("accent_wing_lift_scale", 1.0) * tip_u*tip_u*(3-2*tip_u)
            wing.data.update()
"""
        sources["eye_accents.py"] = replace_once(
            sources["eye_accents.py"], seat, seat + slender
        )
    if p["layered_lashes"]:
        sources["thin_ribbon.py"] = replace_once(
            sources["thin_ribbon.py"],
            '(b[0]-a[0])/STYLE["accent_max_station_gap"]',
            '(b[0]-a[0])/(min(STYLE["accent_max_station_gap"],0.025) if key == "outer_lash_wing_rails" else STYLE["accent_max_station_gap"])',
        )
    if p["upper_lash_layer"]:
        # The intermediate leaf is rooted on the final upper band, not on
        # the inherited medial skin support used by the old inner fork.
        seating = """
        for side in (-1, 1):
            parent = next(o for o in PARTS if o.name == "Upper lash " + str(side))
            leaf = next(o for o in PARTS if o.name == "Inner forked lash blade " + str(side))
            parent_tree = BVHTree.FromPolygons([v.co.copy() for v in parent.data.vertices], [tuple(p.vertices) for p in parent.data.polygons])
            root = leaf.data.vertices[0].co.copy()
            hit = parent_tree.find_nearest(root)[0]
            assert hit is not None, "Intermediate layer parent missing"
            delta = hit - root
            assert delta.length < .004, "Intermediate layer attachment exceeds design bound"
            lo = min(abs(v.co.x) for v in leaf.data.vertices)
            hi = max(abs(v.co.x) for v in leaf.data.vertices)
            for vertex in leaf.data.vertices:
                u = max(0., min(1., (abs(vertex.co.x)-lo)/(hi-lo)))
                vertex.co += delta * (1-u*u*(3-2*u))
            leaf.data.update()
"""
        seat_anchor = "        tuft_seating = seat_tuft_roots_on_parent(PARTS)"
        sources["eye_accents.py"] = replace_once(
            sources["eye_accents.py"], seat_anchor, seat_anchor + seating
        )
    if p["upper_lash_layer"]:
        anchor = "        maximum = max(distances)"
        check = """        fork = next(o for o in PARTS if o.name == "Inner forked lash blade "+str(side))
        fork_distance = tree.find_nearest(fork.data.vertices[0].co)[3]
        assert fork_distance is not None and fork_distance < .0008, "Intermediate upper layer root disconnected"
"""
        sources["eye_accents.py"] = replace_once(
            sources["eye_accents.py"], anchor, check + anchor
        )
    sources["eye_socket.py"] = adapt_lower_clearance(
        sources["eye_socket.py"],
        p["aperture_upper_share"] != 0
        or p["aperture_corner_taper"] != 0
        or p["lower_roundness"] != 1
        or p["lower_shoulder"] != 0
        or p["lower_horizontal_roundness"] != 1,
    )
    sources["lateral_fit.py"] = adapt_lateral_depth(
        sources["lateral_fit.py"], p["lateral_aperture_follow"]
    )
    from source_lateral_regularization import adapt_regularization

    sources["lateral_fit.py"] = adapt_regularization(
        sources["lateral_fit.py"], p["lateral_guide_regularization"]
    )
    sources["eye_accents.py"] = adapt_lateral_rim(
        sources["eye_accents.py"], p["lateral_aperture_follow"]
    )
    sources = adapt_upper_sources(
        sources,
        p["upper_asymmetry"] != 0
        or p["lower_shoulder"] != 0
        or p["upper_angularity"] != 0,
    )
    sources = adapt_angular_samples(sources, p["upper_angularity"])
    if blink:
        sources["semantic_eye_runtime.py"] += (
            "\n\n"
            + (
                HELPERS
                if p["upper_asymmetry"] or p["lower_shoulder"] or p["upper_angularity"]
                else ""
            )
            + (HERE / "blink_runtime.py")
            .read_text()
            .replace("from contour_warp import upper_shape, lower_shape\n", "")
        )
        sources["pipeline.py"] = replace_once(
            sources["pipeline.py"],
            "        opened.value = 0.0",
            "        opened.value = 0.0\n"
            "        from semantic_eye_runtime import build_blink_shapes\n"
            f'        report["eyelid_closure"] = build_blink_shapes(members[0], semantic_part_ordinals, C, ink_floor_closure={bool(style.get("illustrated_eye_art", False) and not p["short_upper_tuft"])!r})',
        )
        sources["glb_merge.py"] = replace_once(
            sources["glb_merge.py"],
            '== ["MouthOpen"]',
            '== ["MouthOpen", "blink.L", "blink.R"]',
        )
        sources["glb_merge.py"] = replace_once(
            sources["glb_merge.py"],
            'node["weights"] = [0.0]',
            'node["weights"] = [0.0, 0.0, 0.0]',
        )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        '    image.filepath = "//" + role.lower() + "_base.png"',
        "    # Keep the same-job absolute atlas path; the inherited blend is still open.\n"
        "    image.pack()",
    )
    sources["eye_socket.py"] = replace_once(
        sources["eye_socket.py"],
        "/ 0.044 + 0.5",
        f'/ (0.044 * {p["iris_height"]!r}) + 0.5',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "(v - 0.5) * 0.044",
        f'(v - 0.5) * (0.044 * {p["iris_height"]!r})',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "iv = y / 0.036 + 0.5",
        f'iv = y / (0.036 * {p["iris_height"]!r}) + 0.5',
    )
    sources["face.py"] = replace_once(
        sources["face.py"],
        '                        * C["lower_cover"]\n                        * H',
        '                        * C["lower_cover"]\n'
        '                        * STYLE.get("lower_lash_width_scale", 1.0)\n                        * H',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        "outline = 0.16 * exp(-(((disc - 1) / 0.055) ** 2))",
        'outline = 0.16 * min(1., STYLE["iris_graphic_disc_strength"] / .30) * exp(-(((disc - 1) / 0.055) ** 2))',
    )
    sources["textures.py"] = replace_once(
        sources["textures.py"],
        ")) / 0.2)) * 0.7",
        ')) / 0.2)) * STYLE.get("iris_accent_strength", .7)',
    )
    if style.get("illustrated_eye_art", False):
        from illustrated_art import adapt_sources

        sources = adapt_sources(sources)
    sources["ear_return.py"] = adapt_return_order(sources["ear_return.py"])
    base.OUTPUT_PREFIX = output_prefix
    with tempfile.TemporaryDirectory() as directory:
        snapshot = Path(directory)
        for name, source in sources.items():
            (snapshot / name).write_text(source)
        request = base.build_request(snapshot)
    input_hashes = {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in REQUIRED
    }
    verification = "import hashlib\nfrom pathlib import Path\n"
    verification += f"for _path, _sha in {input_hashes!r}.items():\n"
    verification += '    assert hashlib.sha256(Path(beaver_input(_path)).read_bytes()).hexdigest() == _sha, "Recipe input changed: " + _path\n'
    request["code"] = verification + request["code"]
    retained = {
        "semantic_parameters.json": json.dumps(p, indent=2),
        "semantic_bundle.py": Path(__file__).read_text()
        + "\n\n# Frozen helper provenance; replay also requires the checkpoint recipe.\nRETAINED_UPPER_SHAPE_SOURCES = "
        + repr(
            {
                "contour_warp.py": HELPERS,
                "source_upper_warp.py": (HERE / "source_upper_warp.py").read_text(),
                "source_return_order.py": (HERE / "source_return_order.py").read_text(),
                "source_lateral_rim.py": (HERE / "source_lateral_rim.py").read_text(),
                "source_lateral_regularization.py": (
                    HERE / "source_lateral_regularization.py"
                ).read_text(),
                "source_angular_samples.py": (
                    HERE / "source_angular_samples.py"
                ).read_text(),
            }
        )
        + "\n",
        "semantic_parameters.py": HELPERS
        + "\n\n"
        + (HERE / "parameters.py")
        .read_text()
        .replace("from contour_warp import upper_shape, upper_shape_derivative\n", ""),
        "definition_guard.py": guard,
        "semantic_input_hashes.json": json.dumps(
            {
                name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                for name in REQUIRED
            },
            indent=2,
        ),
    }
    for name, source in retained.items():
        request[
            "code"
        ] += f"Path(beaver_output({(output_prefix + name)!r})).write_text({source!r})\n"
    for name in [*retained, "semantic_face_export.glb"]:
        request["outputs"].append(
            {"path": output_prefix + name, "expectedSha256": None}
        )
    for entry in request["outputs"]:
        path = root / entry["path"]
        if path.exists():
            if not path.is_file():
                raise ValueError("Non-file output: " + entry["path"])
            entry["expectedSha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    compile(request["code"], "<semantic eye native job>", "exec")
    return request


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parameters", required=True, help="JSON numeric controls")
    parser.add_argument("--prefix", required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--blink", action="store_true", help="Experimental independent eyelid closure"
    )
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(
            build_request(
                json.loads(args.parameters),
                args.prefix,
                args.workspace,
                blink=args.blink,
            )
        )
    )

"""Read-only CPU hypothesis sampling against the current procedural recipe."""

import argparse
import ast
import json
from pathlib import Path
import sys
from hypothesis import validate, base_correction, orbital_support
from runtime_field import rim_slope_delta

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "coherent_surface"))
from fit import fit_profile
from curve_profile import evaluate
from diagnostics import scope, measures

sys.path.pop(0)
sys.path.insert(0, str(HERE.parent / "semantic_eyes"))
from parameters import adapt_style, validate as validate_eyes
from source_upper_warp import adapt_face_source

sys.path.pop(0)


def study(parameters, eyes):
    p = validate(parameters)
    recipe = HERE.parent / "checkpoints/current"
    source = (recipe / "face.py").read_text()
    style = adapt_style(
        json.loads((recipe / "face_style.json").read_text()), validate_eyes(eyes)
    )
    c = style["calibration"]
    source = adapt_face_source(source, bool(c.get("eye_upper_asymmetry", 0)))
    fit = fit_profile(source, c)
    base = scope(source, c, fit, 0)
    node = next(
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == "eye_contour"
    )
    exec(
        compile(ast.Module(body=[node], type_ignores=[]), "<eye contour>", "exec"), base
    )

    def correction(x, y):
        h = (y - c["origin_y_m"]) / c["height_m"]
        q = abs(x) / base["width"](y)
        return base_correction(
            q,
            h,
            c["height_m"],
            base["curve"](h, 1, c["continuous_front_profile"]),
            base["curve"](h, 1, c["continuous_back_profile"]),
            evaluate(h, fit["segments"]),
            p,
        )

    changes = []
    for j in range(101):
        h = 0.1 + 0.8 * j / 100
        y = c["origin_y_m"] + h * c["height_m"]
        for i in range(86):
            changes.append(abs(correction(base["width"](y) * i / 100, y)))
    aperture = []
    for upper in (False, True):
        for i in range(101):
            t = -1 + 2 * i / 100
            x, y = base["eye_contour"](1, t, upper)
            oldraw, oldrim = base["base_depth"](x, y), base["rim_depth"](x, y)
            raw, rim = orbital_support(oldraw, oldrim, correction(x, y), p)
            rim += rim_slope_delta(x, c, {"parameters": p})
            aperture.append(
                dict(
                    t=t,
                    upper=upper,
                    x_m=x,
                    height_m=y,
                    old_gap_m=oldraw - oldrim,
                    new_gap_m=raw - rim,
                    rim_displacement_m=rim - oldrim,
                )
            )
    changed = scope(source, c, fit, 0)
    changed["base_depth"] = lambda x, y: base["base_depth"](x, y) + correction(x, y)
    changed["rim_depth"] = lambda x, y: orbital_support(
        base["base_depth"](x, y), base["rim_depth"](x, y), correction(x, y), p
    )[1] + rim_slope_delta(x, c, {"parameters": p})
    cuts = []
    for axis, stations in (
        ("vertical", (0.03, 0.04, 0.05, 0.06)),
        ("transverse", (0.3, 0.4, 0.5, 0.6, 0.7)),
    ):
        for station in stations:
            coords = (
                [
                    (station, c["origin_y_m"] + (0.1 + 0.8 * i / 400) * c["height_m"])
                    for i in range(401)
                ]
                if axis == "vertical"
                else [
                    (i * 0.0002, c["origin_y_m"] + station * c["height_m"])
                    for i in range(401)
                ]
            )
            step = (
                (coords[1][0] - coords[0][0]) ** 2 + (coords[1][1] - coords[0][1]) ** 2
            ) ** 0.5
            before, after = [
                measures([s["depth"](x, y) for x, y in coords], step)
                for s in (base, changed)
            ]
            cuts.append(
                dict(
                    axis=axis,
                    station=station,
                    before=before,
                    after=after,
                    variation_ratio=after["curvature_variation"]
                    / before["curvature_variation"],
                )
            )
    maximum = max(changes)
    rim_max = max(abs(row["rim_displacement_m"]) for row in aperture)
    return dict(
        parameters=p,
        diagnostic_cuts=cuts,
        visible_iris_center="Not measured: analytic rim gaps are boundary support differences, not visible pupil/iris recess",
        scope="CPU hypothesis only; no native builder or acceptance",
        sampled_base_max_offset_m=maximum,
        sampled_rim_max_offset_m=rim_max,
        within_proposed_2mm_sample_budget=maximum <= 0.002 and rim_max <= 0.002,
        old_aperture_gap_range_m=[
            min(r["old_gap_m"] for r in aperture),
            max(r["old_gap_m"] for r in aperture),
        ],
        new_aperture_gap_range_m=[
            min(r["new_gap_m"] for r in aperture),
            max(r["new_gap_m"] for r in aperture),
        ],
        aperture_samples=aperture,
        limitations="No continuous bound or final-mesh/ocular clearance guarantee; frame and attachment rebuilding still required",
    )


def require_sample_budget(report):
    if not report["within_proposed_2mm_sample_budget"]:
        raise ValueError("Sampled base/rim offset exceeds proposed 2mm budget")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--parameters", required=True)
    parser.add_argument("--eyes", default="{}")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(
        json.dumps(study(json.loads(args.parameters), json.loads(args.eyes)), indent=2)
    )

"""CPU contracts and interval tests; these do not replace native validation."""

import ast
import copy
import json
import math
from pathlib import Path
import random
import sys
import unittest

HERE = Path(__file__).resolve().parent
SURFACE = HERE.parent / "facial_surface"
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(SURFACE))
from nose_design import validate, stations
from nose_source import apply_design, adapt_face, companion, adapt_pipeline
from interval_bounds import I, raw_without_origin

PRESETS = json.loads((HERE / "presets.json").read_text())
RECIPE = HERE.parent / "checkpoints/current"
STYLE = json.loads((RECIPE / "face_style.json").read_text())


def analytic(design):
    style, _ = apply_design(STYLE, design)
    c = style["calibration"]
    source = adapt_face((RECIPE / "face.py").read_text(), design)
    names = {
        "curve",
        "width",
        "nasal_projection_curve",
        "researched_muzzle",
        "nose_profile_factor",
        "nose",
        "lip_shoulder_support",
        "lip_detail_depth",
        "base_depth",
    }
    nodes = [
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name in names
    ]
    ns = {
        "C": c,
        "H": c["height_m"],
        "Y0": c["origin_y_m"],
        "Z0": c["chin_depth_m"],
        "exp": math.exp,
        "sqrt": math.sqrt,
    }
    exec(compile(ast.Module(body=nodes, type_ignores=[]), "nose analytic", "exec"), ns)
    return ns, c


class NoseDesignTests(unittest.TestCase):
    def test_strict_controls(self):
        for d in PRESETS.values():
            self.assertEqual(validate(d)["kind"], d["kind"])
        for key, value in [
            ("projection", True),
            ("projection", float("nan")),
            ("projection", float("inf")),
            ("half_width", 1),
            ("kind", "unknown"),
            ("version", True),
            ("eye_width", 1),
        ]:
            d = {**PRESETS["balanced"], key: value}
            with self.assertRaises(ValueError):
                validate(d)
        with self.assertRaises(ValueError):
            validate({**PRESETS["compact"], "muzzle_width": 0.1})

    def test_legacy_is_identity(self):
        out, record = apply_design(STYLE, None)
        self.assertEqual(out, STYLE)
        self.assertIsNone(record)
        source = (RECIPE / "face.py").read_text()
        self.assertEqual(adapt_face(source, None), source)
        self.assertEqual(
            companion("nose_refine.py", "legacy source", None), "legacy source"
        )

    def test_locality_symmetry_and_finite(self):
        for d in PRESETS.values():
            ns, c = analytic(d)
            H, Y = ns["H"], ns["Y0"]
            for h in (0.05, 0.15, 0.46, 0.7):
                self.assertEqual(ns["nose"](0, Y + H * h), 0.0)
            for h in (0.2, 0.25, 0.29, 0.35, 0.40):
                for x in (0, 0.002, 0.008, 0.018, 0.04):
                    a = ns["nose"](x, Y + H * h)
                    b = ns["nose"](-x, Y + H * h)
                    self.assertTrue(math.isfinite(a))
                    self.assertGreaterEqual(a, -1e-12)
                    self.assertAlmostEqual(a, b, 12)

    def test_continuous_presets(self):
        a, b = PRESETS["compact"], PRESETS["elongated"]
        last = None
        for i in range(41):
            t = i / 40
            d = {
                k: (
                    a[k] * (1 - t) + b[k] * t
                    if type(a[k]) in (int, float) and k != "version"
                    else a[k]
                )
                for k in a
            }
            ns, _ = analytic(d)
            v = ns["nose"](0, ns["Y0"] + 0.29 * ns["H"])
            if last is not None:
                self.assertLess(abs(v - last), 0.002)
            last = v
        d = PRESETS["balanced"]
        w = stations(d)
        self.assertEqual(
            [r[0] for r in w], [r[0] for r in stations({**d, "projection": 0.10})]
        )

    def test_interval_encloses_emitted_field(self):
        rng = random.Random(8471)
        for d in PRESETS.values():
            ns, c = analytic(d)
            H, Y, Z = ns["H"], ns["Y0"], ns["Z0"]
            for _ in range(80):
                x = rng.uniform(-0.04, 0.04)
                h = rng.uniform(0.16, 0.45)
                dx = rng.uniform(0.00001, 0.0007)
                dh = rng.uniform(0.00001, 0.002)
                bound = raw_without_origin(I(x - dx, x + dx), I(h - dh, h + dh), c)
                for xx in (x - dx, x, x + dx):
                    for hh in (h - dh, h, h + dh):
                        value = ns["base_depth"](xx, Y + hh * H) - Z
                        self.assertLessEqual(bound.lo - 1e-10, value)
                        self.assertGreaterEqual(bound.hi + 1e-10, value)

    def test_exact_support_and_station_contract(self):
        for d in PRESETS.values():
            rows = stations(d)
            self.assertEqual(len(rows), 7)
            self.assertTrue(all(a[0] < b[0] for a, b in zip(rows, rows[1:])))
            self.assertEqual(rows[0][1], 0)
            self.assertEqual(rows[-1][1], 0)
            ns, c = analytic(d)
            for h in (rows[0][0], rows[-1][0]):
                if d["kind"] == "fused":
                    self.assertAlmostEqual(ns["nose"](0, ns["Y0"] + ns["H"] * h), 0, 12)

    def test_attachment_order_extension_present(self):
        source = 'PARTS[:] = stable_part_order(PARTS)\ndef check(data):\n    if True:\n        report["ear_shared_seams"] = verify_ear_seams(data, report["ear_root_fit"])\n'
        out = adapt_pipeline(source, PRESETS["pad-muzzle"])
        self.assertLess(
            out.index("build_attachment(STYLE"), out.index("for _part in PARTS:")
        )
        self.assertIn("max(semantic_part_ordinals.values(), default=9)+1", out)
        self.assertIn("Original semantic regions changed", out)


if __name__ == "__main__":
    unittest.main()

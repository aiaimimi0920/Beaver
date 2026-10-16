import ast
import hashlib
import json
import unittest
from pathlib import Path
import numpy as np
from study import scope, fit_profile, adapt_style, validate_eyes
from interval_bounds import I, raw_without_origin, rim_without_origin, orbital_weight
from runtime_field import base_delta, ocular_delta, response
from ocular_fit import fit_plane, component_box
from transverse_fit import fit_response


class SupportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        recipe = Path(__file__).resolve().parent.parent / "checkpoints/current"
        source = (recipe / "face.py").read_text()
        cls.c = adapt_style(
            json.loads((recipe / "face_style.json").read_text()), validate_eyes({})
        )["calibration"]
        cls.fit = fit_profile(source, cls.c)
        cls.old = scope(source, cls.c, cls.fit, 0)
        node = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "eye_contour"
        )
        exec(
            compile(ast.Module(body=[node], type_ignores=[]), "<contour>", "exec"),
            cls.old,
        )

    def test_interval_contains_actual_field_including_low_sections(self):
        c = self.c
        old = self.old
        for h in (0.001, 0.04, 0.099, 0.3, 0.44, 0.7, 0.99):
            for x in (0.0, 0.01, 0.04, 0.07, 0.095):
                span = 0.00001
                raw = raw_without_origin(
                    I(max(0, x - span), x + span), I(max(0, h - span), h + span), c
                )
                actual = (
                    old["base_depth"](x, c["origin_y_m"] + h * c["height_m"])
                    - old["Z0"]
                )
                self.assertLessEqual(raw.lo, actual)
                self.assertGreaterEqual(raw.hi, actual)
                w = orbital_weight(
                    I(max(0, x - span), x + span), I(max(0, h - span), h + span), c
                )
                self.assertGreaterEqual(w.lo, 0)
                self.assertLessEqual(w.hi, 1)

    def test_runtime_basis_and_identity(self):
        fit = fit_response(0.0001)
        from transverse_fit import response as oracle

        for q in np.linspace(-1, 1, 101):
            for key in ("axial", "rounding"):
                self.assertAlmostEqual(
                    response(q, fit[key]), oracle(q, fit[key]), places=14
                )
        data = dict(
            parameters=dict(axial_smoothing=0.0, cheek_roundness=0.0),
            segments=self.fit["segments"],
            transverse=fit,
        )
        self.assertEqual(base_delta(0.04, 1.65, self.c, self.old["curve"], data), 0.0)

    def test_plane_recovers_affine_displacement_and_both_eye_domains(self):
        old = self.old
        c = self.c
        center = [
            c["height_m"] * c["iris_center_x"],
            c["origin_y_m"] + c["height_m"] * c["iris_center_y"],
        ]
        changed = dict(old)
        changed["rim_depth"] = (
            lambda x, y: old["rim_depth"](x, y)
            + 0.01 * (abs(x) - center[0])
            - 0.02 * (y - center[1])
            + 0.001
        )
        plane = fit_plane(c, old, changed)
        self.assertLess(plane["displacement_fit_residual_max_m"], 1e-14)
        for side in (-1, 1):
            x = side * center[0] + c["height_m"] * c["catchlight_offset_x"]
            y = center[1] + c["height_m"] * c["catchlight_offset_y"]
            self.assertAlmostEqual(
                ocular_delta(x, y, {"ocular_plane": plane}),
                0.01 * (abs(x) - center[0]) - 0.02 * (y - center[1]) + 0.001,
            )
        with self.assertRaises(ValueError):
            ocular_delta(center[0] + 1, center[1], {"ocular_plane": plane})

    def test_bundle_is_fresh_retained_and_rejects_unreviewed_bound(self):
        import tempfile
        from bundle import build_request

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + n
                for n in (
                    "aster_head_editable.blend",
                    "aster_head.glb",
                    "aster_definition.tres",
                    "hair_base.png",
                    "hair_ilm.png",
                )
            ]
            for name in paths:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("CPU fixture")
            request = build_request(
                {}, {}, "assets/aster/general_face_eyes_cpu/", root, 0.001
            )
            compile(request["code"], "<CPU packaged source>", "exec")
            outputs = [item["path"] for item in request["outputs"]]
            self.assertLessEqual(len(outputs), 64)
            self.assertEqual(len(outputs), len(set(outputs)))
            retained = {}
            for node in ast.walk(ast.parse(request["code"])):
                if not (
                    isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Attribute)
                    and node.func.attr == "write_text"
                    and node.args
                    and isinstance(node.args[0], ast.Constant)
                ):
                    continue
                path_call = node.func.value
                if not (
                    isinstance(path_call, ast.Call)
                    and path_call.args
                    and isinstance(path_call.args[0], ast.Call)
                ):
                    continue
                inner = path_call.args[0]
                if (
                    isinstance(inner.func, ast.Name)
                    and inner.func.id == "beaver_output"
                ):
                    retained[Path(ast.literal_eval(inner.args[0])).name] = node.args[
                        0
                    ].value
            manifest = json.loads(retained["facial_surface_source_manifest.json"])
            for name, digest in manifest.items():
                self.assertEqual(
                    hashlib.sha256(retained[name].encode()).hexdigest(), digest
                )
            preflight = json.loads(retained["facial_surface_preflight.json"])
            self.assertEqual(
                preflight["packager_helpers"]["source_speed.py"],
                Path(__file__).with_name("source_speed.py").read_text(),
            )
            self.assertIn("ocular_delta(x, y, DATA)", request["code"])
            self.assertIn("facial_surface_source_manifest.json", request["code"])
            self.assertNotIn("head_recovery_617", request["code"])
            with self.assertRaises(ValueError):
                build_request(
                    {}, {}, "assets/aster/general_face_eyes_cpu/", root, False
                )


if __name__ == "__main__":
    unittest.main()

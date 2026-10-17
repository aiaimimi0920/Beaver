"""Independent plane budget is retained, additive and enforced before packaging."""

import ast
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from study import adapt_style
from preflight import prepare
from bundle import build_request
from ocular_fit import component_box
import runtime_field


class IrisPlaneBoundTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        recipe = Path(__file__).parents[1] / "checkpoints/current"
        cls.source = (recipe / "face.py").read_text()
        cls.base = json.loads((recipe / "face_style.json").read_text())
        cls.params = dict(
            axial_smoothing=0.5, cheek_roundness=0.05, orbit_shallowing=0.15
        )

    def test_additive_bound_all_component_domain_and_default_identity(self):
        baseline = self.base["calibration"]["iris_plane_slope"]
        for scale in (1, 0.85):
            c = adapt_style(self.base, {"iris_plane_slope_scale": scale})["calibration"]
            data, report = prepare(
                self.source, c, self.params, baseline_iris_plane_slope=baseline
            )
            plane, surface = data["ocular_plane"], report["surface"]
            expected = abs(c["iris_plane_slope"] - baseline) * max(
                map(abs, component_box(c)[0])
            )
            self.assertEqual(
                surface["additional_iris_plane_displacement_bound_m"], expected
            )
            self.assertEqual(
                surface["conservative_support_displacement_bound_m"],
                surface["support_bound_before_iris_plane_change_m"] + expected,
            )
            for side in (-1, 1):
                for dx in plane["domain_relative_m"][0]:
                    for dy in plane["domain_relative_m"][1]:
                        x, y = side * (plane["center"][0] + dx), plane["center"][1] + dy
                        value = runtime_field.ocular_delta(x, y, data)
                        total = value + (c["iris_plane_slope"] - baseline) * dx
                        self.assertLessEqual(
                            abs(total), plane["combined_ocular_displacement_bound_m"]
                        )
                        with patch.object(
                            runtime_field, "REVIEWED_BOUND", 0.00000001, create=True
                        ):
                            with self.assertRaisesRegex(
                                ValueError, "reviewed displacement"
                            ):
                                runtime_field.ocular_delta(x, y, data)

    def test_bundle_rejects_old_budget_and_retains_new_budget(self):
        # A wider-aperture fixture crosses .0042 only after the plane budget is added.
        # Actual 726 has a smaller bound; do not claim it requires .0045.
        eyes = dict(
            iris_plane_slope_scale=0.85,
            upper_arch=1.4,
            lower_arch=1,
            corner_slope_offset=-0.014,
            lash_band_thickness=0.92,
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, "exceeds explicitly reviewed"):
                build_request(
                    self.params,
                    eyes,
                    "assets/aster/general_face_eyes_cpu/",
                    root,
                    0.0042,
                )
            for name in ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + name
                for name in (
                    "aster_head_editable.blend",
                    "aster_head.glb",
                    "aster_definition.tres",
                    "hair_base.png",
                    "hair_ilm.png",
                )
            ]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("CPU fixture")
            request = build_request(
                self.params,
                eyes,
                "assets/aster/general_face_eyes_cpu/",
                root,
                0.0045,
                "emerald_illustrated",
            )
        payload = None
        for node in ast.walk(ast.parse(request["code"])):
            if isinstance(node, ast.Constant) and isinstance(node.value, str):
                if node.value.startswith('{\n  "data":'):
                    payload = json.loads(node.value)
        self.assertIsNotNone(payload)
        report = payload["report"]
        self.assertEqual(report["reviewed_support_bound_m"], 0.0045)
        bound = report["surface"]["conservative_support_displacement_bound_m"]
        self.assertGreater(bound, 0.0042)
        self.assertLessEqual(bound, 0.0045)
        self.assertGreater(
            payload["data"]["ocular_plane"]["authored_horizontal_displacement_bound_m"],
            0,
        )
        self.assertIn(
            "Actual ocular support exceeds reviewed displacement bound", request["code"]
        )


if __name__ == "__main__":
    unittest.main()

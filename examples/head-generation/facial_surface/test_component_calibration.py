import copy
import json
import unittest
from pathlib import Path
from source_components import apply_components


class ComponentCalibrationTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads(
            (
                Path(__file__).parents[1] / "checkpoints/current/face_style.json"
            ).read_text()
        )

    def test_default_is_exact_identity_and_detached(self):
        result, report = apply_components(self.style)
        self.assertEqual(result, self.style)
        self.assertIsNone(report)
        self.assertIsNot(result["calibration"], self.style["calibration"])

    def test_slots_change_only_their_own_parameters(self):
        cases = [
            (
                {"mouth_width_scale": 1.08},
                {"mouth_half_width", "upper_lip_lateral_width"},
            ),
            ({"nose_bridge_scale": 1.2}, {"nose_stations"}),
            ({"ear_span_scale": 0.94}, {"ear_outward_span"}),
        ]
        original = copy.deepcopy(self.style)
        for parameters, keys in cases:
            result, report = apply_components(self.style, parameters)
            changed = {
                k
                for k in result["calibration"]
                if result["calibration"][k] != self.style["calibration"][k]
            }
            self.assertEqual(changed, keys)
            self.assertEqual(set(report["before"]), keys)
            self.assertEqual(self.style, original)

    def test_nose_tip_root_and_base_are_preserved(self):
        result, _ = apply_components(self.style, {"nose_bridge_scale": 1.2})
        a = self.style["calibration"]["nose_stations"]
        b = result["calibration"]["nose_stations"]
        for i in (0, 1, 2, 3, 4, 6):
            self.assertEqual(a[i], b[i])
        self.assertGreater(b[5][1], a[5][1])
        self.assertEqual(a[5][2], b[5][2])

    def test_unknown_nonfinite_and_out_of_bounds_fail(self):
        for parameters in (
            {"unknown": 1},
            {"ear_span_scale": True},
            {"mouth_width_scale": float("nan")},
            {"nose_bridge_scale": 1.3},
            {"ear_span_scale": 0.5},
        ):
            with self.assertRaises(ValueError):
                apply_components(self.style, parameters)

    def test_combination_commutes(self):
        combined, _ = apply_components(
            self.style,
            {
                "mouth_width_scale": 1.08,
                "nose_bridge_scale": 1.2,
                "ear_span_scale": 0.94,
            },
        )
        serial = self.style
        for parameters in (
            {"ear_span_scale": 0.94},
            {"nose_bridge_scale": 1.2},
            {"mouth_width_scale": 1.08},
        ):
            serial, _ = apply_components(serial, parameters)
        self.assertEqual(serial, combined)

    def test_long_bridge_has_independent_height_and_projection_controls(self):
        original = self.style["calibration"]["nose_stations"]
        result, report = apply_components(
            self.style,
            {"nose_bridge_length_scale": 1.2, "nose_tip_projection_scale": 1.08},
        )
        stations = result["calibration"]["nose_stations"]
        self.assertEqual(set(report["before"]), {"nose_stations"})
        for i in range(5):
            self.assertEqual(stations[i][0], original[i][0])
        for i in (0, 1, 5, 6):
            self.assertEqual(stations[i][1], original[i][1])
        self.assertEqual([r[2] for r in stations], [r[2] for r in original])
        self.assertGreater(stations[-1][0], original[-1][0])
        self.assertLess(stations[-1][0], 0.43)
        self.assertGreater(stations[3][1], original[3][1])
        self.assertEqual(stations[0], original[0])
        self.assertEqual(stations[1], original[1])
        result["calibration"]["nose_stations"] = original
        self.assertEqual(result, self.style)

    def test_new_nose_controls_reject_invalid_values(self):
        for key, high in [
            ("nose_bridge_length_scale", 1.2),
            ("nose_tip_projection_scale", 1.08),
        ]:
            for value in [True, 0.99, high + 0.001, float("nan"), float("inf")]:
                with self.assertRaises(ValueError):
                    apply_components(self.style, {key: value})

    def test_length_only_preserves_station_projections(self):
        result, _ = apply_components(self.style, {"nose_bridge_length_scale": 1.2})
        self.assertEqual(
            [r[1] for r in result["calibration"]["nose_stations"]],
            [r[1] for r in self.style["calibration"]["nose_stations"]],
        )

    def test_tip_only_preserves_station_heights(self):
        result, _ = apply_components(self.style, {"nose_tip_projection_scale": 1.08})
        self.assertEqual(
            [r[0] for r in result["calibration"]["nose_stations"]],
            [r[0] for r in self.style["calibration"]["nose_stations"]],
        )

    def test_new_identity_controls_preserve_old_bridge_shape(self):
        a, _ = apply_components(self.style, {"nose_bridge_scale": 1.2})
        b, _ = apply_components(
            self.style,
            {
                "nose_bridge_scale": 1.2,
                "nose_bridge_length_scale": 1,
                "nose_tip_projection_scale": 1,
            },
        )
        self.assertEqual(a, b)

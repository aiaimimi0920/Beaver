"""Exact numeric behavior of the derived-source zero-weight fast path."""

import ast
import json
from pathlib import Path
import unittest
import subprocess
import sys
from source_speed import skip_zero_weight_sections
from study import scope, fit_profile

RECIPE = Path(__file__).resolve().parents[1] / "checkpoints" / "current"


class SourceSpeedTests(unittest.TestCase):
    def test_depth_and_numerical_normals_exact(self):
        source = (RECIPE / "face.py").read_text()
        c = json.loads((RECIPE / "face_style.json").read_text())["calibration"]
        fit = fit_profile(source, c)
        old = scope(source, c, fit, 0)
        new = scope(skip_zero_weight_sections(source), c, fit, 0)
        for i in range(81):
            y = c["origin_y_m"] + c["height_m"] * (-0.1 + i / 60)
            for j in range(51):
                x = c["height_m"] * (j - 25) / 40
                for dx, dy in ((0, 0), (0.00001, 0), (0, 0.00001)):
                    self.assertEqual(
                        old["base_depth"](x + dx, y + dy),
                        new["base_depth"](x + dx, y + dy),
                    )

    def test_stdlib_profiler_does_not_shadow_curve_helper(self):
        command = "import cProfile,sys; sys.path.insert(0,sys.argv[1]); import study; assert callable(study.evaluate)"
        subprocess.run(
            [sys.executable, "-c", command, str(Path(__file__).resolve().parent)],
            check=True,
        )

    def test_contract_drift_rejected(self):
        with self.assertRaises(ValueError):
            skip_zero_weight_sections("def base_depth(x,y): return 0")


if __name__ == "__main__":
    unittest.main()

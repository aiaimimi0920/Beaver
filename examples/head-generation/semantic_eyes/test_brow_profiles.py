import ast
import math
from pathlib import Path
import unittest
from source_brow_profiles import adapt_brow_profiles


class BrowProfileTests(unittest.TestCase):
    def setUp(self):
        self.source = (
            Path(__file__).parents[1] / "checkpoints/current/face.py"
        ).read_text()

    def functions(self, **values):
        source = adapt_brow_profiles(self.source, **values)
        tree = ast.parse(source)
        nodes = [
            n
            for n in tree.body
            if isinstance(n, ast.FunctionDef)
            and n.name in ("brow_lane_offset", "brow_profile_arch")
        ]
        ns = {
            "H": 1.0,
            "C": {"brow_thickness": 0.01, "brow_inner_fullness": 0.45},
            "sin": math.sin,
            "pi": math.pi,
        }
        exec(compile(ast.Module(body=nodes, type_ignores=[]), "profile", "exec"), ns)
        return ns

    def test_default_exact_identity(self):
        self.assertEqual(adapt_brow_profiles(self.source), self.source)
        self.assertEqual(adapt_brow_profiles(self.source, peak=0.7), self.source)

    def test_peak_and_endpoints(self):
        for peak in (0.3, 0.5, 0.75):
            f = self.functions(angularity=1, peak=peak)["brow_profile_arch"]
            self.assertAlmostEqual(f(0), 0)
            self.assertAlmostEqual(f(1), 0)
            self.assertAlmostEqual(f(peak), 1)
            self.assertGreater(f(peak), f(peak - 0.01))
            self.assertGreater(f(peak), f(peak + 0.01))

    def test_band_preserves_centerline_and_caps(self):
        f = self.functions(band=1)["brow_lane_offset"]
        for t in (0, 0.01, 0.1, 0.5, 0.9, 0.99, 1):
            lo, hi = f(t, 0), f(t, 1)
            self.assertGreaterEqual(hi - lo, 0)
            self.assertAlmostEqual((hi + lo) / 2, 0.005 * math.sin(math.pi * t) ** 0.7)
            if t in (0, 1):
                self.assertAlmostEqual(hi - lo, 0, places=4)
        self.assertGreater(f(0.95, 1) - f(0.95, 0), 0.006)

    def test_other_functions_unchanged(self):
        before = ast.parse(self.source)
        after = ast.parse(
            adapt_brow_profiles(self.source, band=1).replace(
                'STYLE["brow_arch_m"] * brow_profile_arch(t)',
                'STYLE["brow_arch_m"] * sin(pi * t)',
            )
        )
        exclude = {"brow_lane_offset", "brow_profile_arch", "build_face"}
        a = {
            n.name: ast.dump(n)
            for n in before.body
            if isinstance(n, ast.FunctionDef) and n.name not in exclude
        }
        b = {
            n.name: ast.dump(n)
            for n in after.body
            if isinstance(n, ast.FunctionDef) and n.name not in exclude
        }
        self.assertEqual(a, b)

    def test_invalid_and_source_contract(self):
        for v in (True, float("nan"), float("inf"), -1, 2):
            for k in ("angularity", "band"):
                with self.assertRaises(ValueError):
                    adapt_brow_profiles(self.source, **{k: v})
        with self.assertRaises(ValueError):
            adapt_brow_profiles("bad", angularity=1)

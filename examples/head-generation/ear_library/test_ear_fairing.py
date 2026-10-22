"""Pure backtracking contracts; native geometry still needs Beaver validation."""

from pathlib import Path
import ast
import unittest

from ear_profile import adapt_boundary, adapt_topology, bounded_fairing, researched_ear_root_radius


class EarFairingTests(unittest.TestCase):
    def test_large_ear_locality_is_bounded_and_small_ears_unchanged(self):
        for kind in ("standard-oval", "compact-round", "slender-oval", "short-point"):
            self.assertEqual(researched_ear_root_radius(
                {"researched_ear_kind": kind, "ear_outward_span": .245}), .012)
        self.assertEqual(researched_ear_root_radius(
            {"researched_ear_kind": "long-leaf", "ear_outward_span": .245}), .020)
        self.assertAlmostEqual(researched_ear_root_radius(
            {"researched_ear_kind": "side-droop", "ear_outward_span": .17}), .012*.17/.118)

    def test_topology_quality_gates_retained_and_old_candidates_preferred(self):
        path = Path(__file__).resolve().parent.parent / "checkpoints/current/ear_topology.py"
        source = path.read_text()
        self.assertEqual(adapt_topology(source, None), source)
        changed = adapt_topology(source, {"kind": "long-leaf"})
        for gate in ("if q1<=q0+.05:continue", "if side_a*side_b>=-1e-18:continue",
                     "40<=faces[0][region]<50", "faces[0][region]==faces[1][region]",
                     "f.calc_area()>1e-12 and f.normal.dot(normal)>.1",
                     "assert actual>q0+.05", "all((v.co-p).length<1e-9"):
            self.assertIn(gate, changed)
        self.assertIn("if not candidates:candidates=expanded", changed)
        self.assertIn("assert fan_after>=fan_before-1e-9", changed)
        self.assertIn("'selected_radius_m':selected_radius", changed)

    def test_posterior_support_follows_the_authored_root(self):
        path = Path(__file__).resolve().parent.parent / "checkpoints/current/ear_boundary.py"
        source = adapt_boundary(path.read_text(), {"kind": "long-leaf"})
        function = next(node for node in ast.parse(source).body
                        if isinstance(node, ast.FunctionDef) and node.name == "support_offset")
        namespace = {}
        exec(ast.get_source_segment(source, function), namespace)
        support = namespace["support_offset"]
        self.assertAlmostEqual(support(.26, 1., .003), .003)
        self.assertAlmostEqual(support(.34, 1., .003, .34), .003)
        self.assertAlmostEqual(support(.26, 1., .003, .34), 0.)
        self.assertEqual(support(.34, .9, .003, .34), 0.)
        self.assertIn("C['ear_root_min_height']-.08", source)

    def test_full_valid_step_is_identical(self):
        original = [(0., 0., 0.), (1., 0., 0.), (0., 1., 0.)]
        proposed = [*original[:2], (0., 0.8, 0.)]
        result, scale = bounded_fairing(original, proposed, {2}, lambda p: True)
        self.assertEqual(result, proposed)
        self.assertEqual(scale, 1.)

    def test_inversion_and_collapse_are_backtracked_not_accepted(self):
        original = [(0., 0., 0.), (1., 0., 0.), (0., 1., 0.)]
        proposed = [*original[:2], (0., -1., 0.)]
        tried = []

        def positive_area(points):
            signed_area = points[2][1] * 0.5
            tried.append(signed_area)
            return signed_area > 1e-12

        result, scale = bounded_fairing(original, proposed, {2}, positive_area)
        self.assertEqual(tried, [-.5, 0., .25])
        self.assertEqual(scale, .25)
        self.assertEqual(result[:2], original[:2])
        self.assertEqual(result[2], (0., .5, 0.))

    def test_failure_restores_original_and_raises(self):
        original, proposed, visited = [(0., 0., 0.)], [(1., 0., 0.)], []

        def invalid(points):
            visited.append(points)
            return False

        with self.assertRaises(AssertionError):
            bounded_fairing(original, proposed, {0}, invalid)
        self.assertEqual(len(visited), 17)
        self.assertEqual(visited[-1], original)

    def test_only_researched_source_changes_and_original_gate_remains(self):
        path = Path(__file__).resolve().parent.parent / "checkpoints/current/ear_boundary.py"
        source = path.read_text()
        self.assertEqual(adapt_boundary(source, None), source)
        changed = adapt_boundary(source, {"kind": "long-leaf"})
        self.assertIn("Boundary fairing inverted or collapsed a face", changed)
        self.assertIn("assert all(tuple(data.vertices[i].co)==original[i] for i in locked)", changed)
        self.assertIn("assert maximum<=.004001", changed)
        self.assertIn("'fairing_scale':fairing_scale", changed)


if __name__ == "__main__":
    unittest.main()

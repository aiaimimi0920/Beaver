import itertools
import random
import unittest
from fractions import Fraction as F
from source_width_guard import limit_lateral_width_step, f32


def stored(points):
    return [tuple(map(f32, p)) for p in points]


def slope(points, ids, axes=(0, 1, 2), origin=0):
    x, d, h = axes
    a, b, c = [tuple(map(F, points[i])) for i in ids]
    nx = (b[d]-a[d])*(c[h]-a[h])-(b[h]-a[h])*(c[d]-a[d])
    if nx == 0:
        return None
    side = 1 if a[x] > origin else -1
    nz = (b[x]-a[x])*(c[d]-a[d])-(b[d]-a[d])*(c[x]-a[x])
    return -side*nz/nx


def assert_guard(test, old, result, faces, moved, axes=(0, 1, 2), origin=0):
    x, d, h = axes
    for i, p in enumerate(result):
        test.assertEqual((p[d], p[h]), (old[i][d], old[i][h]))
        if i not in moved:
            test.assertEqual(p, old[i])
    for face in faces:
        if not set(face).intersection(moved):
            continue
        for ids in itertools.combinations(face, 3):
            xs = [old[i][x]-origin for i in ids]
            if not (all(a > 0 for a in xs) or all(a < 0 for a in xs)):
                continue
            before = slope(old, ids, axes, origin)
            if before is not None and before > 0:
                test.assertGreaterEqual(slope(result, ids, axes, origin), 0)


class WidthGuardTests(unittest.TestCase):
    def setUp(self):
        self.old = stored([(1, 0, 0), (2, 0, 1), (1, 1, 0), (9, 2, 3)])
        self.new = stored([(3, 0, 0), (2, 0, 1), (3, 1, 0), (9, 2, 3)])
        self.faces = [(0, 1, 2)]
        self.moved = {0, 2}

    def test_known_triangle_limit_and_untouched_vertex(self):
        out, report = limit_lateral_width_step(self.old, self.new, self.faces, self.moved)
        self.assertEqual(report['analytic_scale_limit'], .5)
        self.assertEqual(report['applied_scale'], .5)
        self.assertEqual(slope(out, (0, 1, 2)), 0)
        assert_guard(self, self.old, out, self.faces, self.moved)

    def test_winding_and_mirror_invariance(self):
        for side in (-1, 1):
            old = [(side*p[0], p[1], p[2]) for p in self.old]
            new = [(side*p[0], p[1], p[2]) for p in self.new]
            for face in itertools.permutations((0, 1, 2)):
                out, report = limit_lateral_width_step(old, new, [face], self.moved)
                self.assertEqual(report['applied_scale'], .5)
                assert_guard(self, old, out, [face], self.moved)

    def test_axes_units_origin_and_height_translation(self):
        for scale in (2**-10, 1, 2**10):
            old = [(p[2]*scale+32*scale, p[0]*scale+4*scale, p[1]*scale) for p in self.old]
            new = [(p[2]*scale+32*scale, p[0]*scale+4*scale, p[1]*scale) for p in self.new]
            out, report = limit_lateral_width_step(old, new, self.faces, self.moved, axes=(1, 2, 0), origin=4*scale)
            self.assertEqual(report['applied_scale'], .5)
            assert_guard(self, old, out, self.faces, self.moved, axes=(1, 2, 0), origin=4*scale)

    def test_preserves_valid_full_step(self):
        new = [(p[0]+.25 if i in self.moved else p[0], p[1], p[2]) for i, p in enumerate(self.old)]
        out, report = limit_lateral_width_step(self.old, new, self.faces, self.moved)
        self.assertEqual(report['applied_scale'], 1)
        self.assertEqual(out, new)

    def test_existing_narrowing_not_forced_monotone(self):
        old = stored([(2, 0, 0), (1, 0, 1), (2, 1, 0)])
        new = stored([(3, 0, 0), (1, 0, 1), (3, 1, 0)])
        out, report = limit_lateral_width_step(old, new, self.faces, self.moved)
        self.assertEqual(report['protected_triangle_charts'], 0)
        self.assertEqual(report['applied_scale'], 1)
        self.assertEqual(out, new)

    def test_rejects_non_lateral_displacement(self):
        new = list(self.new)
        new[0] = (new[0][0], .25, new[0][2])
        with self.assertRaises(AssertionError):
            limit_lateral_width_step(self.old, new, self.faces, self.moved)

    def test_zero_projected_area_not_divided(self):
        old = stored([(1, 0, 0), (2, 1, 1), (4, 2, 2)])
        new = stored([(2, 0, 0), (3, 1, 1), (5, 2, 2)])
        out, report = limit_lateral_width_step(old, new, [(0, 1, 2)], {0, 1, 2})
        self.assertEqual(report['protected_triangle_charts'], 0)
        self.assertEqual(report['applied_scale'], 1)

    def test_near_singular_nonzero_projection_has_no_epsilon_blind_spot(self):
        old = stored([(1, 0, 0), (2, 0, 1), (1, 2**-80, 0)])
        new = stored([(3, 0, 0), (2, 0, 1), (3, 2**-80, 0)])
        out, report = limit_lateral_width_step(old, new, self.faces, self.moved)
        self.assertEqual(report['applied_scale'], .5)
        assert_guard(self, old, out, self.faces, self.moved)

    def test_stored_float32_rechecked_and_boundary_repaired(self):
        old = [(0.10000000149011612, 0., 0.), (0.13149699568748474, 0.7484998106956482, 1.), (0.10316493362188339, 1., 0.038831133395433426)]
        new = [(0.1229507252573967, 0., 0.), (0.15156863629817963, 0.7484998106956482, 1.), (0.18381382524967194, 1., 0.038831133395433426)]
        out, report = limit_lateral_width_step(old, new, self.faces, {0, 1, 2})
        self.assertGreater(report['float32_repair_steps'], 0)
        self.assertLessEqual(report['applied_scale'], report['analytic_scale_limit'])
        self.assertLess(report['analytic_scale_limit']-report['applied_scale'], 1e-7)
        assert_guard(self, old, out, self.faces, {0, 1, 2})

    def test_ngon_all_possible_triangles(self):
        yz = [(0, 0), (1, 0), (2, 1), (1, 2), (0, 2), (-1, 1)]
        old = stored([(10+h, d, h) for d, h in yz])
        new = stored([(p[0]+5 if i < 2 else p[0], p[1], p[2]) for i, p in enumerate(old)])
        out, report = limit_lateral_width_step(old, new, [tuple(range(6))], {0, 1})
        self.assertGreater(report['protected_triangle_charts'], 4)
        assert_guard(self, old, out, [tuple(range(6))], {0, 1})

    def test_random_quad_diagonals_and_displacement_bound(self):
        randomizer = random.Random(90117)
        for _ in range(200):
            base = randomizer.uniform(1, 10)
            up = randomizer.uniform(.01, 2)
            depth = randomizer.uniform(-.5, .5)
            old = stored([(base+up*h+depth*d, d, h) for d, h in [(0, 0), (1, 0), (1, 1), (0, 1)]])
            new = stored([(p[0]+randomizer.uniform(-.5, 3), p[1], p[2]) for p in old])
            out, report = limit_lateral_width_step(old, new, [(0, 1, 2, 3)], range(4))
            assert_guard(self, old, out, [(0, 1, 2, 3)], set(range(4)))
            for p, q, r in zip(old, new, out):
                self.assertLessEqual(abs(r[0]-p[0]), abs(q[0]-p[0]))


if __name__ == '__main__':
    unittest.main(verbosity=2)

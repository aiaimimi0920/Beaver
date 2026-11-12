import copy
import unittest
from types import SimpleNamespace
import numpy as np
from forehead_anchor_domain import ForeheadAnchorDomain
from forehead_author_gap_samples import ForeheadFrozenGapResidual


class ForeheadAnchorDomainTests(unittest.TestCase):
    def fixture(self):
        p = np.array([
            [0., 0., .003], [.001, 0., .003], [.002, 0., .002], [.003, 0., .001],
            [0., 0., .003], [.001, 0., .003], [.002, 0., .0004], [.003, 0., .0001],
        ])
        f = SimpleNamespace(
            pivot=np.array([0., .0005]), entry=0., height=.0005,
            names=["red", "blue"], paths={"red": [0, 1, 2, 3], "blue": [4, 5, 6, 7]},
            endpoint_chart={"red": p[3, [0, 2]].copy(), "blue": p[7, [0, 2]].copy()},
        )
        return p, f, [1, 2, 5, 6]

    def test_exact_rows_affine_jacobian_and_aliases(self):
        p, f, ids = self.fixture()
        before = p.copy()
        obj = ForeheadAnchorDomain(f, p, ids)
        x = p[ids][:, [0, 2]].ravel() * 1000
        self.assertEqual([s[1] for s in obj.specs], ["upper", "radial", "upper", "lower"])
        np.testing.assert_allclose(obj.initial, [1., 3.5, 1., .3], atol=1e-12)
        expected_threshold = np.array([1e-11, 1e-8, 1e-11, 1e-11])
        np.testing.assert_array_equal(obj.threshold, expected_threshold)
        jacobian = obj.jacobian(x).toarray()
        direction = np.linspace(-.2, .2, len(x))
        np.testing.assert_allclose(obj.values(x+direction)-obj.values(x), jacobian@direction, atol=1e-12)
        for column in range(len(x)):
            direction = np.zeros(len(x))
            direction[column] = 1e-5
            np.testing.assert_allclose(
                (obj.values(x+direction)-obj.values(x-direction))/2e-5,
                jacobian[:, column], atol=1e-9,
            )
        p[:] = 99.
        np.testing.assert_array_equal(obj.fixed, before[:, [0, 2]]*1000)
        self.assertFalse(obj.margin.flags.writeable)

    def test_invalid_variable_ids(self):
        p, f, ids = self.fixture()
        for invalid in (ids+[ids[0]], [-1, 1], [len(p)], [1.0], [True], ids+[0], ids+[7]):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                ForeheadAnchorDomain(f, p, invalid)

    def test_invalid_shapes_and_nonfinite(self):
        p, f, ids = self.fixture()
        cases = [p[:, :2], p.ravel(), np.empty((0, 3)), p.copy()]
        cases[-1][1, 0] = np.nan
        for bad in cases:
            with self.assertRaises(ValueError):
                ForeheadAnchorDomain(f, bad, ids)
        obj = ForeheadAnchorDomain(f, p, ids)
        for bad in (np.zeros((len(ids), 2)), np.zeros(3), np.full(2*len(ids), np.inf)):
            with self.assertRaises(ValueError):
                obj.values(bad)

    def test_paths_and_frozen_frame(self):
        p, f, ids = self.fixture()
        mutations = [
            lambda z: z.paths.update(red=[0]),
            lambda z: z.paths.update(red=[0, 1, 1, 3]),
            lambda z: z.paths.update(red=[0, -1, 3]),
            lambda z: z.paths.update(red=[0, 1.0, 3]),
            lambda z: setattr(z, "pivot", np.array([0., 0.])),
            lambda z: setattr(z, "pivot", np.array([0., .0005, 0.])),
            lambda z: setattr(z, "entry", .001),
            lambda z: z.endpoint_chart.update(red=np.array([.004, .001])),
            lambda z: setattr(z, "names", ["red", "red"]),
        ]
        for mutation in mutations:
            other = copy.deepcopy(f)
            mutation(other)
            with self.assertRaises(ValueError):
                ForeheadAnchorDomain(other, p, ids)

    def test_original_author_first_pin_checked_when_frame_exposes_it(self):
        p, f, ids = self.fixture()
        f.atlas = SimpleNamespace(sc=p.copy())
        changed = p.copy()
        changed[0, 2] += .0001
        with self.assertRaisesRegex(ValueError, "original author"):
            ForeheadAnchorDomain(f, changed, ids)
        obj = ForeheadAnchorDomain(f, p, ids)
        self.assertTrue(np.all(obj.values(p[ids][:, [0, 2]].ravel()*1000) > 0))

    def test_seam_classification_uses_frozen_terminal_only(self):
        p, f, ids = self.fixture()
        for offset, expected in [(2e-10, "radial"), (0., "radial"), (-.5e-10, "radial"), (-2e-10, "lower")]:
            q = p.copy()
            q[3, 2] = f.height + offset
            g = copy.deepcopy(f)
            g.endpoint_chart["red"] = q[3, [0, 2]].copy()
            obj = ForeheadAnchorDomain(g, q, ids)
            self.assertEqual(obj.specs[1][1], expected)
        # A penultimate point may cross the seam without reclassifying its
        # terminal role. No interior membership is stored by this module.
        obj = ForeheadAnchorDomain(f, p, ids)
        self.assertEqual(obj.specs[3][1], "lower")

    def test_dense_atlas_counterexample_necessity_not_sufficiency(self):
        names = ("red", "orange", "green", "blue", "purple")
        coords, paths = [], {}
        for role, name in enumerate(names):
            offset = (4-role)*.001
            path = []
            for u, h in [(0., .005), (.0005, .005), (.0015, .004), (.0025, .003), (.004, .002)]:
                path.append(len(coords))
                coords.append([u, 0., h+offset])
            paths[name] = path
        params = np.array(coords)
        atlas = SimpleNamespace(entry=.001, exit_h=0., sc=params.copy(), old=params.copy())
        proposal = {"paths": paths, "protected": [i for path in paths.values() for i in (path[0], path[-1])]}
        fast = ForeheadFrozenGapResidual(atlas, proposal, 129)
        self.assertTrue(fast.evaluate(params, params)["valid"])
        ids = [i for path in paths.values() for i in path[1:-1]]
        obj = ForeheadAnchorDomain(fast, params, ids)
        trial = params.copy()
        path = paths["orange"]
        i = path[2]
        trial[i, 0] = trial[path[1], 0] - .0001
        self.assertTrue(np.all(obj.values(trial[ids][:, [0, 2]].ravel()*1000) > 0))
        self.assertFalse(fast.evaluate(trial, trial)["valid"])


if __name__ == "__main__":
    unittest.main()

"""Focused contracts for affine-safe diagonal proposals."""

import unittest
import numpy as np
from shapely.geometry import LineString
from affine_diagonal_trial import refine


class DiagonalTrialTests(unittest.TestCase):
    def setUp(self):
        self.uv = np.array([[0.0, 0.0], [3.0, 0.0], [4.0, 1.0], [0.0, 1.0]])
        self.faces = np.array([[0, 1, 2], [0, 2, 3]])
        p = np.c_[self.uv, np.zeros(4)]
        self.fields = {"POSITION": p, "NORMAL": np.tile([0.0, 0.0, 1.0], (4, 1))}
        self.fields.update({f"morph{i}.POSITION": np.zeros_like(p) for i in range(7)})

    def run_trial(self, protected=None, locked=None):
        return refine(
            self.fields,
            self.uv,
            self.faces,
            protected if protected is not None else LineString(),
            locked or set(),
        )

    def test_improves_only_diagonal(self):
        faces, report = self.run_trial()
        self.assertEqual(len(report["accepted"]), 1)
        self.assertLess(report["after"], report["before"])
        np.testing.assert_array_equal(np.unique(faces), np.unique(self.faces))
        np.testing.assert_array_equal(self.faces, [[0, 1, 2], [0, 2, 3]])

    def test_non_affine_attribute_rejects(self):
        self.fields["NORMAL"][3, 0] = 0.01
        faces, report = self.run_trial()
        self.assertEqual(report["accepted"], [])
        np.testing.assert_array_equal(faces, self.faces)

    def test_protected_edge_rejects(self):
        _, report = self.run_trial(LineString(self.uv[[0, 2]]))
        self.assertEqual(report["accepted"], [])

    def test_locked_support_rejects(self):
        _, report = self.run_trial(locked={0})
        self.assertEqual(report["accepted"], [])

    def test_morph_gradient_rejects(self):
        self.fields["morph3.POSITION"][3, 2] = 0.01
        _, report = self.run_trial()
        self.assertEqual(report["accepted"], [])

    def test_reversed_winding_is_preserved(self):
        self.faces = self.faces[:, ::-1].copy()
        faces, report = self.run_trial()
        self.assertEqual(len(report["accepted"]), 1)
        for t in faces:
            q = self.uv[t]
            self.assertLess(np.linalg.det([q[1] - q[0], q[2] - q[0]]), 0)


if __name__ == "__main__":
    unittest.main()

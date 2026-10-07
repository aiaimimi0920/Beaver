import json, unittest
from pathlib import Path
from test_style_math import pure_function

C = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]


class RearReturnTests(unittest.TestCase):
    def test_return_preserves_all_original_vertices_and_connected_edges(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("shell.py", "add_posterior_return", {"curve": curve})
        v = [
            (0, 0.25, 0.06),
            (0.1, 0.25, 0.06),
            (0.16, 0.28, 0.1),
            (0.24, 0.30, 0.15),
            (0.285, 0.335, 0.2),
            (0.31, 0.36, 0.28),
            (0.33, 0.34, 0.32),
        ]
        # A simple positive-x half surface with a traced sloped rear boundary.
        faces = [(0, 1, 2), (0, 2, 3), (0, 3, 4), (0, 4, 5), (0, 5, 6)]
        out, fs, ids, n = fn(v, faces, C, 1, 0, 0)
        self.assertEqual(out[: len(v)], v)
        self.assertGreater(n, 0)
        self.assertEqual(fs[: len(faces)], faces)
        for i in range(len(v), len(out)):
            self.assertGreaterEqual(out[i][0], 0)
            self.assertLessEqual(out[i][0], max(x[0] for x in v))
            self.assertTrue(any(i in f for f in fs))
        uses = {}
        for f in fs:
            self.assertEqual(len(set(f)), len(f))
            for a, b in zip(f, f[1:] + f[:1]):
                e = tuple(sorted((a, b)))
                uses[e] = uses.get(e, 0) + 1
        self.assertLessEqual(max(uses.values()), 2)

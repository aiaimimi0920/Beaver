"""High-precision diagnostics, not a mathematical distance/acceptance certificate."""
import itertools
import json
import os
from pathlib import Path
import unittest

import numpy as np
from test_support import mesh
from oracle_decimal import decimal_triangle


class DecimalTests(unittest.TestCase):
    def compare_all_orders(self, triangle, queries):
        oracle = [decimal_triangle(triangle, p) for p in queries]
        oracle_distance = np.array([row[0] for row in oracle])
        oracle_hit = np.array([row[1] for row in oracle])
        ulp = abs(np.spacing(max(1., abs(triangle).max())))
        rows = []
        for order in itertools.permutations(range(3)):
            with mesh(triangle[list(order)], [[0, 1, 2]]) as index:
                distance, _, hits = index.query(queries, True)
            distance_error = float(abs(distance - oracle_distance).max())
            hit_error = float(np.linalg.norm(hits - oracle_hit, axis=1).max())
            # Numerical comparison budget, never added to the unchanged .001 test.
            self.assertLessEqual(distance_error, 32 * ulp)
            self.assertLessEqual(hit_error, 32 * ulp)
            rows.append({"order": order, "distance_error": distance_error,
                         "hit_error": hit_error, "coordinate_ulp": float(ulp),
                         "threshold_decision_differences": int(np.sum(
                             (distance > .001) != (oracle_distance > .001)))})
        return rows

    def queries(self, triangle, rotation):
        return np.array([np.asarray(bary) @ triangle + height * rotation[2]
                         for bary in ([.2, .3, .5], [.5, .5, 0.],
                                      [.5, .5 - 1e-12, 1e-12], [.5, .5 + 1e-12, -1e-12])
                         for height in (0., np.nextafter(.001, 0), .001, np.nextafter(.001, np.inf))])

    def test_rotated_sliver_and_edge_witnesses(self):
        rng = np.random.default_rng(2026110203)
        rows = []
        for aspect in (1., 1e-2, 1e-8, 1e-12, 1e-15, 0.):
            for translation in (0., 7., 1e8):
                rotation, _ = np.linalg.qr(rng.normal(size=(3, 3)))
                triangle = np.array([[0., 0., 0.], [1., 0., 0.], [1., aspect, 0.]]) @ rotation
                triangle += np.array([translation, -translation, 2 * translation])
                queries = self.queries(triangle, rotation)
                if translation == 1e8:
                    # Former translated fixtures remain explicitly unsupported;
                    # never silently normalize them or claim them repaired.
                    with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                        mesh(triangle, [[0, 1, 2]])
                    rows.append({"aspect": aspect, "translation": translation,
                                 "unsupported": True, "query_count": len(queries)})
                    continue
                for row in self.compare_all_orders(triangle, queries):
                    rows.append({"aspect": aspect, "translation": translation,
                                 "unsupported": False, "query_count": len(queries), **row})
        report = {"supported_queries_per_order": 192, "orders": 6,
                  "unsupported_queries": 96, "cases": rows,
                  "threshold_m": .001, "acceptance_tolerance_added": False,
                  "limitation": "Rounded binary64 hits are not real-arithmetic certificates"}
        if os.environ.get("MESH_DISTANCE_RESULTS"):
            output = Path(os.environ["MESH_DISTANCE_RESULTS"])
            output.mkdir(parents=True, exist_ok=True)
            (output / "decimal.json").write_text(json.dumps(report, indent=2) + "\n")

    def test_original_sliver_rotation_stream_retained(self):
        # Preserve the old two-rotation RNG stream too, rather than accidentally
        # replacing it when adding the new translation=7 diagnostic cases.
        rng = np.random.default_rng(2026110203)
        for aspect in (1., 1e-2, 1e-8, 1e-12, 1e-15, 0.):
            for translation in (0., 1e8):
                rotation, _ = np.linalg.qr(rng.normal(size=(3, 3)))
                triangle = np.array([[0., 0., 0.], [1., 0., 0.], [1., aspect, 0.]]) @ rotation
                triangle += np.array([translation, -translation, 2 * translation])
                if translation:
                    with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                        mesh(triangle, [[0, 1, 2]])
                else:
                    self.compare_all_orders(triangle, self.queries(triangle, rotation))


if __name__ == "__main__":
    unittest.main()

"""Corpus-free array/tuple builder boundary tests; no solver or JSON coercion."""
import copy
import unittest
import numpy as np
from forehead_pattern_catalogue import forehead_pattern_catalogue, forehead_pattern_edges
from forehead_pattern_materialize import forehead_pattern_materialize, forehead_pattern_plain_records


class ForeheadPatternNumpyTests(unittest.TestCase):
    def case(self):
        v = np.array([[x, 0., y] for y in range(3) for x in range(3)], float)
        faces = [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]]
        edges = {edge for face in faces for edge in forehead_pattern_edges(face)}
        top = {
            "vertices": v.tolist(), "chart": v.tolist(), "faces": copy.deepcopy(faces),
            "face_owners": list(range(4)), "lineage": {},
            "edge_lineage": [{"edge": list(e), "parent": list(e)} for e in edges],
            "paths": {"role": [1, 4, 7]}, "retiled_new_faces": [], "retiled_old_faces": [],
        }
        p = {
            "vertices": v.tolist(), "faces": faces, "topology": top,
            "source_chart": v.tolist(), "open_delta": np.zeros_like(v).tolist(),
            "lineage": {}, "paths": {"role": [1, 4, 7]}, "protected": [],
            "moving": [], "affected_new": [], "affected_old": [],
        }

        def sample(point):
            x, y = point[0], point[2]
            ix, iy = min(int(x), 1), min(int(y), 1)
            u, z = x - ix, y - iy
            donors = [iy * 3 + ix, iy * 3 + ix + 1, (iy + 1) * 3 + ix + 1, (iy + 1) * 3 + ix]
            return point.tolist(), {"donors": donors, "weights": [(1-u)*(1-z), u*(1-z), u*z, (1-u)*z]}

        cat = forehead_pattern_catalogue(
            p, range(4), v, sample, {"role": [1]*9}, edges, {"role": (1, 7)}
        )
        patterns = []
        for i, face in enumerate(faces):
            mask = sum(1 << k for k, edge in enumerate(forehead_pattern_edges(face)) if 4 in edge)
            patterns.append(next(row for row in cat["patterns"]
                                 if row["face"] == i and row["mask"] == mask
                                 and "transition_rotation" not in row))
        witness = {
            "feasible": True, "points": cat["points"], "lineage": cat["lineage"],
            "patterns": patterns, "selected_base_faces": list(range(4)),
            "new_midpoints": list(cat["mids"].values()), "new_centers": list(cat["centers"].values()),
            "guide_ends": cat["guide_ends"],
            "paths": {"role": [1, cat["mids"][(1, 4)], 4, cat["mids"][(4, 7)], 7]},
        }
        return p, witness, v, edges

    def test_array_birth_materialization_matches_list_and_preserves_aliases(self):
        p, witness, source, edges = self.case()
        fields = {edge: (0., 0, False, True) for edge in edges}
        structured = {"membership": [[[0, 1.0]]] * 9}
        expected, _ = forehead_pattern_materialize(
            p, witness, source, np.zeros_like(source), structured, fields, edges
        )
        q = copy.deepcopy(p)
        q["vertices"] = source.copy()
        q["source_chart"] = q["vertices"]
        q["topology"]["vertices"] = q["vertices"]
        q["topology"]["chart"] = q["vertices"]
        q["topology"]["face_owners"] = np.arange(4)
        q["faces"] = np.array(q["faces"])
        q["open_delta"] = np.zeros_like(source)
        q["paths"] = {"role": np.array([1, 4, 7])}
        before = forehead_pattern_plain_records(q)
        result, audit = forehead_pattern_materialize(
            q, witness, source, np.zeros_like(source), structured, fields, edges
        )
        self.assertEqual(result, expected)
        self.assertEqual(audit["new_points"], 8)
        self.assertEqual(forehead_pattern_plain_records(q), before)
        result["vertices"][0][0] = 999
        self.assertEqual(forehead_pattern_plain_records(q), before)

    def test_structured_ambiguity_still_rejects(self):
        p, witness, source, edges = self.case()
        fields = {edge: (0., 0, False, True) for edge in edges}
        with self.assertRaisesRegex(ValueError, "discrete semantics"):
            forehead_pattern_materialize(
                p, witness, source, np.zeros_like(source),
                {".authored_role": [[[i, 1.0]] for i in range(9)]}, fields, edges
            )

    def test_tuple_rows_and_edge_keys_preserve_values(self):
        row = {(1, 2): (np.float64(.125), np.array([1, 2], dtype=np.int64))}
        converted = forehead_pattern_plain_records(row)
        self.assertEqual(converted, {(1, 2): [.125, [1, 2]]})
        self.assertIsInstance(next(iter(converted)), tuple)


if __name__ == "__main__":
    unittest.main()

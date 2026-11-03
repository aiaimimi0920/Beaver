"""Complete native quotient proofs, including forbidden face-split aliases."""
from copy import deepcopy
import unittest
from construction_weld_contract import validate_weld, terminal_map, projected_face


def source(point_count, faces, extra_edges=()):
    rows, offset, edges = [], 0, []
    for face in faces:
        rows.append({"points": face, "corners": list(range(offset, offset + len(face)))})
        offset += len(face)
        for a, b in zip(face, face[1:] + face[:1]):
            edge = sorted((a, b))
            if edge not in edges:
                edges.append(edge)
    for edge in extra_edges:
        if list(edge) not in edges:
            edges.append(list(edge))
    return {"point_ids": ["p" + str(i) for i in range(point_count)], "faces": rows, "edges": edges}


def observed(before, pairs, keep_faces=None):
    targets = terminal_map(len(before["point_ids"]), pairs)
    origins = sorted(set(targets.values()))
    index = {p: i for i, p in enumerate(origins)}
    rows, seen = [], set()
    for i, face in enumerate(before["faces"]):
        if keep_faces is not None and i not in keep_faces:
            continue
        kept = projected_face(face["points"], face["corners"], targets)
        if len(kept) < 3:
            continue
        rows.append({"face_origin": i, "points": [index[p] for _, p in kept],
                     "corner_origins": [c for c, _ in kept]})
    edges = []
    for a, b in before["edges"]:
        a, b = index[targets[a]], index[targets[b]]
        key = tuple(sorted((a, b)))
        if a != b and key not in seen:
            edges.append(list(key))
            seen.add(key)
    return {"point_origins": origins, "faces": rows, "edges": edges}


class WeldContractTests(unittest.TestCase):
    def test_noop_complete_quotient(self):
        before = source(4, [[0, 1, 2, 3]])
        result = validate_weld(before, [], observed(before, []))
        self.assertEqual(result["point_sources"], [[0], [1], [2], [3]])
        self.assertEqual(result["retired_degenerate_faces"], [])

    def test_external_duplicate_point_and_edge_candidates(self):
        before = source(5, [[0, 1, 2, 3]], [(1, 4)])
        result = validate_weld(before, [[4, 0]], observed(before, [[4, 0]]))
        self.assertEqual(result["point_sources"][0], [0, 4])
        self.assertIn([0, 4], result["edges"])

    def test_adjacent_collapse_uses_outgoing_loop_donor(self):
        before = source(4, [[0, 1, 2, 3]])
        after = observed(before, [[1, 0]])
        self.assertEqual(after["faces"][0]["corner_origins"], [1, 2, 3])
        validate_weld(before, [[1, 0]], after)
        after["faces"][0]["corner_origins"][0] = 0
        with self.assertRaisesRegex(ValueError, "corner donor"):
            validate_weld(before, [[1, 0]], after)

    def test_duplicate_faces_record_actual_donor_and_retirement(self):
        before = source(6, [[0, 1, 2], [3, 4, 5]])
        pairs = [[3, 0], [4, 1], [5, 2]]
        for keep in (0, 1):
            result = validate_weld(before, pairs, observed(before, pairs, [keep]))
            self.assertEqual(result["faces"][0]["source_face_candidates"], [0, 1])
            self.assertEqual(result["retired_duplicate_faces"], [1 - keep])
            self.assertEqual(result["faces"][0]["source_face"], keep)

    def test_original_nonadjacent_split_rejected_before_compression(self):
        targets = terminal_map(4, [[0, 2], [1, 2]])
        with self.assertRaisesRegex(ValueError, "split before"):
            projected_face([0, 1, 2, 3], [0, 1, 2, 3], targets)

    def test_external_nonadjacent_target_collision_rejected(self):
        targets = terminal_map(5, [[0, 4], [2, 4]])
        with self.assertRaisesRegex(ValueError, "nonadjacent"):
            projected_face([0, 1, 2, 3], [0, 1, 2, 3], targets)

    def test_target_map_alias_chain_and_cycles_rejected(self):
        for pairs in ([[True, 2]], [[1.0, 2]], [[0, 4]], [[1, 1]], [[1, 0], [1, 2]],
                      [[0, 1], [1, 2]], [[0, 1], [1, 0]]):
            with self.subTest(pairs=pairs), self.assertRaises(ValueError):
                terminal_map(4, pairs)

    def test_survivors_and_complete_incidence_are_not_counts_only(self):
        before = source(4, [[0, 1, 2, 3]])
        good = observed(before, [])
        bads = []
        row = deepcopy(good); row["point_origins"][0] = 1; bads.append(row)
        row = deepcopy(good); row["faces"][0]["points"].reverse(); bads.append(row)
        row = deepcopy(good); row["edges"][0] = [0, 2]; bads.append(row)
        row = deepcopy(good); row["faces"] = []; bads.append(row)
        row = deepcopy(good); row["faces"] *= 2; bads.append(row)
        row = deepcopy(good); row["faces"][0]["face_origin"] = False; bads.append(row)
        row = deepcopy(good); row["faces"][0]["corner_origins"][0] = False; bads.append(row)
        for bad in bads:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                validate_weld(before, [], bad)

    def test_no_arbitrary_opposite_face_orientation_merge(self):
        before = source(6, [[0, 1, 2], [5, 4, 3]])
        pairs = [[3, 0], [4, 1], [5, 2]]
        with self.assertRaisesRegex(ValueError, "omitted"):
            validate_weld(before, pairs, observed(before, pairs, [0]))


if __name__ == "__main__":
    unittest.main()

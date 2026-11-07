"""Allocation identity composition, with no positions or nearest-point authority."""
from copy import deepcopy
import unittest

from construction_oral_contract import expand_half_boundary, bind_oral_ring, compose_donor_targets
from construction_topology import digest, stable_id
from construction_weld_contract import terminal_map, validate_weld


def fixture():
    boundary = bind_oral_ring(["bottom", "a", "b", "top"], ["bottom", "top"],
                             "skin", ["oral" + str(i) for i in range(8)], list(range(6)))
    source = {"points": [row["skin_point_id"] for row in boundary["pairs"]]
              + boundary["oral_point_ids"] + ["unrelated-a", "unrelated-b"]}
    count = len(source["points"])
    permutation = list(range(3, count + 1)) + [1, 2]
    join = {"source": source, "source_sha256": digest(source), "identity_maps": {"POINT": permutation}}
    before = {"point_ids": [source["points"][i - 1] for i in permutation], "faces": [], "edges": [],
              "join_source_sha256": join["source_sha256"]}
    indices = {point: i for i, point in enumerate(before["point_ids"])}
    pairs = []
    for index, row in enumerate(boundary["pairs"]):
        pair = [indices[row["skin_point_id"]], indices[row["oral_point_id"]]]
        pairs.append(pair if index % 2 else pair[::-1])
    return boundary, join, weld(before, pairs)


def weld(before, pairs):
    origins = sorted(set(terminal_map(len(before["point_ids"]), pairs).values()))
    after = {"point_origins": origins, "faces": [], "edges": []}
    post = {"point_origins": list(reversed(origins)), "faces": [], "edges": []}
    return {"before": before, "native_target_pairs": pairs, "after_weld": after, "after_mesh": post,
            "quotient": validate_weld(before, pairs, after), "mesh_quotient": validate_weld(before, pairs, post)}


class OralContractTests(unittest.TestCase):
    def test_exact_half_mirror_and_seams(self):
        rows = expand_half_boundary(["bottom", "a", "b", "top"], ["bottom", "top"])
        self.assertEqual([(r["source_point_id"], r["side"]) for r in rows],
                         [("bottom", 0), ("a", 1), ("b", 1), ("top", 0), ("b", -1), ("a", -1)])

    def test_mirror_ids_match_existing_native_schema(self):
        boundary, _, _ = fixture()
        for row in boundary["pairs"]:
            side = {0: "seam", 1: "original", -1: "reflected"}[row["side"]]
            self.assertEqual(row["skin_point_id"], stable_id("skin", "mirrored_point", [row["source_point_id"], side]))

    def test_rejects_bad_half_or_seam_declarations(self):
        for half, seams in [(["a", "b"], ["a", "b"]), (["a", "b", "a"], ["a", "a"]),
                            (["a", "b", "c"], ["c", "a"]), (["a", "b", "c"], ["a", "b", "c"]),
                            (["a", 1, "c"], ["a", "c"])]:
            with self.subTest(half=half, seams=seams), self.assertRaises(ValueError):
                expand_half_boundary(half, seams)

    def test_rejects_wrong_ring_allocation(self):
        for ring in ([1, 0, 2, 3], [0, 1, 2], [False, 1, 2, 3], [0, 1, 1, 3]):
            with self.subTest(ring=ring), self.assertRaises(ValueError):
                bind_oral_ring(["a", "b", "c"], ["a", "c"], "skin", ["o0", "o1", "o2", "o3"], ring)

    def test_rejects_aliased_oral_identity(self):
        for oral in (["o0"] * 4, ["o0", "o1"], [stable_id("skin", "mirrored_point", ["a", "seam"]), "o1", "o2", "o3"]):
            with self.subTest(oral=oral), self.assertRaises(ValueError):
                bind_oral_ring(["a", "b", "c"], ["a", "c"], "skin", oral, list(range(4)))

    def test_compose_permuted_join_and_post_mesh_storage(self):
        boundary, join, receipt = fixture()
        result = compose_donor_targets(boundary, join, receipt)
        self.assertFalse(result["current_role_admission"])
        self.assertFalse(result["field_transfer_admission"])
        targets = terminal_map(len(receipt["before"]["point_ids"]), receipt["native_target_pairs"])
        for row in result["rows"]:
            skin = receipt["before"]["point_ids"].index(row["skin_point_id"])
            actual_origin = receipt["after_mesh"]["point_origins"][row["target_index"]]
            self.assertEqual(actual_origin, targets[skin])
        self.assertEqual(len({row["target_index"] for row in result["rows"]}), 6)

    def test_no_input_mutation(self):
        inputs = fixture()
        before = deepcopy(inputs)
        compose_donor_targets(*inputs)
        self.assertEqual(inputs, before)

    def test_missing_pair_fails(self):
        boundary, join, receipt = fixture()
        receipt = weld(receipt["before"], receipt["native_target_pairs"][1:])
        with self.assertRaisesRegex(ValueError, "contributors"):
            compose_donor_targets(boundary, join, receipt)

    def test_extra_contributor_fails(self):
        boundary, join, receipt = fixture()
        extra = receipt["before"]["point_ids"].index("unrelated-a")
        pairs = receipt["native_target_pairs"] + [[extra, receipt["native_target_pairs"][0][1]]]
        with self.assertRaisesRegex(ValueError, "contributors"):
            compose_donor_targets(boundary, join, weld(receipt["before"], pairs))

    def test_unrelated_merge_fails(self):
        boundary, join, receipt = fixture()
        extras = [receipt["before"]["point_ids"].index(p) for p in ("unrelated-a", "unrelated-b")]
        with self.assertRaisesRegex(ValueError, "contributors"):
            compose_donor_targets(boundary, join, weld(receipt["before"], receipt["native_target_pairs"] + [extras]))

    def test_crossed_pairs_and_multiple_skin_owners_fail(self):
        boundary, join, receipt = fixture()
        ids = receipt["before"]["point_ids"]
        indices = [[ids.index(r["skin_point_id"]), ids.index(r["oral_point_id"])] for r in boundary["pairs"]]
        crossed = [[indices[0][0], indices[1][1]], [indices[1][0], indices[0][1]]] + indices[2:]
        multiple_skin = [indices[0], [indices[1][0], indices[0][1]], [indices[1][1], indices[0][1]]] + indices[2:]
        for pairs in (crossed, multiple_skin):
            with self.subTest(pairs=pairs), self.assertRaises(ValueError):
                compose_donor_targets(boundary, join, weld(receipt["before"], pairs))

    def test_changed_join_binding_or_quotients_fail(self):
        for mutation in ("source", "permutation", "before", "quotient", "mesh"):
            boundary, join, receipt = fixture()
            if mutation == "source": join["source"]["points"][0] = "alien"
            if mutation == "permutation": join["identity_maps"]["POINT"][0] = True
            if mutation == "before": receipt["before"]["point_ids"].reverse()
            if mutation == "quotient": receipt["quotient"]["point_sources"][0].append(0)
            if mutation == "mesh": receipt["mesh_quotient"]["sha256"] = "forged"
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                compose_donor_targets(boundary, join, receipt)

    def test_changed_boundary_receipt_fails(self):
        boundary, join, receipt = fixture()
        boundary["pairs"][0]["oral_point_id"] = boundary["pairs"][1]["oral_point_id"]
        with self.assertRaises(ValueError):
            compose_donor_targets(boundary, join, receipt)

    def test_boundary_digest_and_unjoined_interior_allocation_fail(self):
        boundary, join, receipt = fixture()
        boundary["sha256"] = "forged"
        with self.assertRaises(ValueError):
            compose_donor_targets(boundary, join, receipt)
        boundary = bind_oral_ring(boundary["half_point_ids"], boundary["seam_point_ids"], "skin",
                                 boundary["oral_point_ids"] + ["missing-interior"], boundary["ring0_indices"])
        with self.assertRaisesRegex(ValueError, "Missing oral"):
            compose_donor_targets(boundary, join, receipt)

    def test_post_mesh_must_preserve_actual_target_origins(self):
        boundary, join, receipt = fixture()
        removed_source = receipt["native_target_pairs"][0][0]
        receipt["after_mesh"]["point_origins"][0] = removed_source
        with self.assertRaises(ValueError):
            compose_donor_targets(boundary, join, receipt)


if __name__ == "__main__":
    unittest.main()

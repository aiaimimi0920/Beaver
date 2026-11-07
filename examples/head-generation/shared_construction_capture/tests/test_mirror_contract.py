"""Pure mirror-lineage regressions; synthetic rows are not native acceptance."""

from copy import deepcopy
import hashlib
from pathlib import Path
import random
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from construction_mirror_contract import validate_mirror_lineage


def fixture(rows=3, columns=4, seed=19, merge=True):
    namespace = hashlib.sha256(f"opaque test topology {rows} by {columns}".encode()).hexdigest()
    points = [f"point@{row}/{col}" for row in range(rows) for col in range(columns)]
    faces = []
    for row in range(rows-1):
        for col in range(columns-1):
            number = len(faces)
            indices = [row*columns+col, row*columns+col+1, (row+1)*columns+col+1, (row+1)*columns+col]
            faces.append({"face_id": f"face@{number}", "point_ids": [points[i] for i in indices],
                          "corner_ids": [f"corner@{number}/{i}" for i in range(4)]})
    source = {"source_namespace": namespace, "point_ids": points, "faces": faces}
    seam = [points[row*columns] for row in range(rows)] if merge else []
    observed = {"source_namespace": namespace, "vertices": [], "faces": []}
    point_indices, loop_index = {}, 0
    for pid in points:
        for side in ("original", "reflected"):
            if pid in seam and side == "reflected":
                point_indices[pid, side] = point_indices[pid, "original"]
            else:
                index = len(observed["vertices"])
                point_indices[pid, side] = index
                observed["vertices"].append({"index": index, "source_point_id": pid})
    for face in faces:
        for side in ("original", "reflected"):
            order = list(range(len(face["point_ids"])))
            if side == "reflected":
                order.reverse()
            shift = (len(observed["faces"]) + 1) % len(order)
            order = order[shift:] + order[:shift]
            observed["faces"].append({"index": len(observed["faces"]), "source_face_id": face["face_id"],
                "vertex_indices": [point_indices[face["point_ids"][i], side] for i in order],
                "source_corner_ids": [face["corner_ids"][i] for i in order],
                "corner_indices": list(range(loop_index, loop_index+len(order)))})
            loop_index += len(order)
    shuffle_storage(observed, seed)
    return source, observed, seam


def shuffle_storage(observed, seed):
    rng = random.Random(seed)
    point_order = list(range(len(observed["vertices"])))
    face_order = list(range(len(observed["faces"])))
    corner_order = list(range(sum(len(face["corner_indices"]) for face in observed["faces"])))
    for order in (point_order, face_order, corner_order):
        rng.shuffle(order)
    for row in observed["vertices"]:
        row["index"] = point_order[row["index"]]
    for row in observed["faces"]:
        row["index"] = face_order[row["index"]]
        row["vertex_indices"] = [point_order[i] for i in row["vertex_indices"]]
        row["corner_indices"] = [corner_order[i] for i in row["corner_indices"]]
    rng.shuffle(observed["vertices"])
    rng.shuffle(observed["faces"])


def compact_observed_vertices(observed):
    order = {row["index"]: i for i, row in enumerate(observed["vertices"])}
    for row in observed["vertices"]:
        row["index"] = order[row["index"]]
    for face in observed["faces"]:
        face["vertex_indices"] = [order[i] for i in face["vertex_indices"]]


class MirrorContractTests(unittest.TestCase):
    def validate(self, source, observed, seam, **kwargs):
        return validate_mirror_lineage(source, observed, seam_point_ids=seam, **kwargs)

    def test_two_sizes_shuffled_storage_and_cyclic_rotations(self):
        for rows, columns in ((3, 4), (5, 7)):
            source, observed, seam = fixture(rows, columns)
            before = deepcopy((source, observed, seam))
            result = self.validate(source, observed, seam, role_paths={"declared.row": source["faces"][0]["point_ids"][:2]})
            self.assertEqual(before, (source, observed, seam))
            self.assertEqual(len(result["point_map"]), rows*columns)
            self.assertEqual(len(result["face_map"]), (rows-1)*(columns-1))
            self.assertEqual(len(result["corner_map"]), 4*(rows-1)*(columns-1))
            self.assertEqual(len(result["seam_equivalence"]), rows)
            for pid, mapping in result["point_map"].items():
                self.assertEqual(set(mapping), {"original", "reflected"})
                self.assertEqual(mapping["original"] == mapping["reflected"], pid in seam)
            self.assertEqual(result["native_acceptance"], "requires separately retained actual native receipt")

    def test_shuffled_input_rows_do_not_change_semantic_receipt(self):
        source, observed, seam = fixture()
        expected = self.validate(source, observed, seam)
        observed["vertices"].reverse()
        observed["faces"].reverse()
        source["point_ids"].reverse()
        source["faces"].reverse()
        self.assertEqual(expected, self.validate(source, observed, seam))

    def test_no_declared_seam_admits_only_unmerged_side_occurrences(self):
        source, observed, seam = fixture(3, 4, merge=False)
        result = self.validate(source, observed, seam)
        self.assertEqual(result["seam_equivalence"], {})
        self.assertTrue(all(p["original"] != p["reflected"] for p in result["point_map"].values()))

    def test_missing_or_wrong_namespace_rejects(self):
        for namespace in (None, "x"*64, "a"*64):
            source, observed, seam = fixture()
            observed["source_namespace"] = namespace
            with self.subTest(namespace=namespace), self.assertRaisesRegex(ValueError, "namespace"):
                self.validate(source, observed, seam)

    def test_undeclared_native_merge_rejects(self):
        source, observed, seam = fixture()
        with self.assertRaisesRegex(ValueError, "Off-seam merge"):
            self.validate(source, observed, seam[1:])

    def test_nonseam_vertex_merge_rejects_even_with_renumbered_storage(self):
        source, observed, seam = fixture()
        rows = [row for row in observed["vertices"] if row["source_point_id"] == source["point_ids"][1]]
        keep, drop = [row["index"] for row in rows]
        for face in observed["faces"]:
            face["vertex_indices"] = [keep if i == drop else i for i in face["vertex_indices"]]
        observed["vertices"] = [row for row in observed["vertices"] if row["index"] != drop]
        compact_observed_vertices(observed)
        with self.assertRaisesRegex(ValueError, "Off-seam merge"):
            self.validate(source, observed, seam)

    def test_declared_seam_must_be_source_boundary(self):
        source, observed, seam = fixture()
        with self.assertRaisesRegex(ValueError, "source boundary"):
            self.validate(source, observed, seam + ["point@1/1"])

    def test_declared_but_unmerged_seam_rejects(self):
        source, observed, _ = fixture(3, 4, merge=False)
        with self.assertRaisesRegex(ValueError, "Declared seam"):
            self.validate(source, observed, ["point@0/0"])

    def test_wrong_marker_domains_and_corner_point_pairing_reject(self):
        for field, bad in (("source_face_id", "unknown"), ("source_corner_ids", ["unknown"]*4)):
            source, observed, seam = fixture()
            observed["faces"][0][field] = bad
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(source, observed, seam)
        source, observed, seam = fixture()
        cids = observed["faces"][0]["source_corner_ids"]
        cids[0], cids[1] = cids[1], cids[0]
        with self.assertRaisesRegex(ValueError, "lineage"):
            self.validate(source, observed, seam)

    def test_source_winding_corner_and_degenerate_corruption_reject(self):
        for field in ("point_ids", "corner_ids", "both", "degenerate"):
            source, observed, seam = fixture()
            face = source["faces"][0]
            if field == "degenerate": face["point_ids"][1] = face["point_ids"][0]
            elif field == "both":
                face["point_ids"].reverse()
                face["corner_ids"].reverse()
            else: face[field].reverse()
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.validate(source, observed, seam)

    def test_duplicate_same_side_face_occurrence_rejects(self):
        source, observed, seam = fixture()
        rows = [row for row in observed["faces"] if row["source_face_id"] == source["faces"][0]["face_id"]]
        for field in ("vertex_indices", "source_corner_ids"):
            rows[1][field] = rows[0][field][:]
        with self.assertRaisesRegex(ValueError, "Duplicate observed source FACE side"):
            self.validate(source, observed, seam)

    def test_missing_face_and_incomplete_storage_reject(self):
        for domain in ("faces", "vertices"):
            source, observed, seam = fixture()
            observed[domain].pop()
            with self.subTest(domain=domain), self.assertRaises(ValueError):
                self.validate(source, observed, seam)

    def test_duplicate_source_ids_and_isolated_vertices_reject(self):
        for kind in ("point", "face", "corner", "isolated_source", "isolated_observed"):
            source, observed, seam = fixture()
            if kind == "point": source["point_ids"].append(source["point_ids"][0])
            if kind == "face": source["faces"].append(deepcopy(source["faces"][0]))
            if kind == "corner": source["faces"][1]["corner_ids"][0] = source["faces"][0]["corner_ids"][0]
            if kind == "isolated_source": source["point_ids"].append("isolated")
            if kind == "isolated_observed":
                observed["vertices"].append({"index": len(observed["vertices"]), "source_point_id": source["point_ids"][1]})
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                self.validate(source, observed, seam)

    def test_stale_and_nonadjacent_role_stations_reject(self):
        source, observed, seam = fixture()
        for path in (["point@0/0", "deleted"], ["point@0/0", "point@1/1"]):
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.validate(source, observed, seam, role_paths={"explicit": path})

    def test_index_domains_reject_bools_holes_and_duplicate_corners(self):
        for kind in ("bool", "point_hole", "face_hole", "corner_hole", "corner_duplicate"):
            source, observed, seam = fixture()
            if kind == "bool": observed["vertices"][0]["index"] = True
            if kind == "point_hole": observed["vertices"][0]["index"] = 999
            if kind == "face_hole": observed["faces"][0]["index"] = 999
            if kind == "corner_hole": observed["faces"][0]["corner_indices"][0] = 999
            if kind == "corner_duplicate":
                observed["faces"][0]["corner_indices"][0] = observed["faces"][1]["corner_indices"][0]
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                self.validate(source, observed, seam)


    def test_split_point_side_incidence_cannot_be_hidden_by_correct_marker_counts(self):
        source, observed, seam = fixture()
        mapping = self.validate(source, observed, seam)["point_map"]["point@1/1"]
        original, reflected = mapping["original"], mapping["reflected"]
        face = next(face for face in observed["faces"] if original in face["vertex_indices"])
        face["vertex_indices"] = [reflected if vi == original else vi for vi in face["vertex_indices"]]
        with self.assertRaisesRegex(ValueError, "mixed lineage"):
            self.validate(source, observed, seam)

    def test_reversed_duplicate_polygon_with_new_ids_is_not_admitted(self):
        source, observed, seam = fixture()
        duplicate = deepcopy(source["faces"][0])
        duplicate["face_id"] = "different-face-id"
        duplicate["point_ids"].reverse()
        duplicate["corner_ids"] = [f"different-corner-{i}" for i in range(4)]
        source["faces"].append(duplicate)
        with self.assertRaisesRegex(ValueError, "Duplicate source polygon"):
            self.validate(source, observed, seam)

    def test_ambiguous_two_point_source_cycle_rejects(self):
        source, observed, seam = fixture()
        source["faces"][0]["point_ids"] = source["faces"][0]["point_ids"][:2]
        source["faces"][0]["corner_ids"] = source["faces"][0]["corner_ids"][:2]
        with self.assertRaisesRegex(ValueError, "Degenerate"):
            self.validate(source, observed, seam)

    def test_unknown_and_duplicate_explicit_seam_ids_reject(self):
        source, observed, seam = fixture()
        for additional in (seam[0], "missing-point"):
            with self.subTest(additional=additional), self.assertRaisesRegex(ValueError, "constructor seam"):
                self.validate(source, observed, seam + [additional])


if __name__ == "__main__":
    unittest.main()

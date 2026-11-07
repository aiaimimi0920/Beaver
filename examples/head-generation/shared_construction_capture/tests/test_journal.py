"""Live allocation journal regressions without Blender or inferred rail roles."""

from copy import deepcopy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from construction_journal import ConstructionJournal
from construction_topology import role_segments, topology


def grid(rows=3, columns=4, unused=False):
    vertices = [(float(col), float(row), .01*row*col) for row in range(rows) for col in range(columns)]
    if unused:
        vertices.append((-1., -1., -1.))
    journal = ConstructionJournal({"producer": "test_explicit_grid"}, list(range(rows)), list(range(columns)))
    for slot, position in enumerate(vertices):
        journal.allocate(["grid", slot], slot, position, {"producer": "test_grid", "station": slot},
                         seam=(slot < rows*columns and slot % columns == 0))
    for row in range(rows):
        journal.role(["row", row], range(row*columns, (row+1)*columns), kind="dense_chart_row")
    faces = [(row*columns+col, row*columns+col+1, (row+1)*columns+col+1, (row+1)*columns+col)
             for row in range(rows-1) for col in range(columns-1)]
    return journal, vertices, faces


def compact(vertices, faces):
    used = sorted({i for face in faces for i in face})
    remap = {old: new for new, old in enumerate(used)}
    return [vertices[old] for old in used], [tuple(remap[i] for i in face) for face in faces], remap


class JournalTests(unittest.TestCase):
    def test_two_dynamic_layouts_keep_complete_identity_and_tombstones(self):
        for rows, columns in ((3, 4), (5, 7)):
            with self.subTest(rows=rows, columns=columns):
                journal, vertices, faces = grid(rows, columns, unused=True)
                original_ids = [row["point_id"] for row in journal.points]
                journal.before_compact(vertices, faces)
                packet = journal.compact(*compact(vertices, faces))
                self.assertEqual(packet["point_ids"], original_ids[:-1])
                self.assertEqual(packet["dropped_point_ids"], original_ids[-1:])
                self.assertEqual(len(packet["allocation_history"]), rows*columns+1)
                self.assertEqual(len(packet["faces"]), (rows-1)*(columns-1))
                self.assertEqual(len(packet["seam_point_ids"]), rows)
                self.assertFalse(packet["solver_ready"])
                for row in range(rows):
                    role = packet["construction_roles"][f"row/{row}"]
                    self.assertEqual(role["segments"], [original_ids[row*columns:(row+1)*columns]])
                    self.assertTrue(role["complete_declared_path"])

    def test_rejected_boundary_checkpoint_does_not_mutate_any_history(self):
        journal, vertices, faces = grid()
        journal.checkpoint("initial", vertices, faces)
        before = deepcopy(journal.__dict__)
        with self.assertRaisesRegex(ValueError, "patch boundary"):
            journal.checkpoint("invalid", vertices, faces[1:], preserve_boundary=True)
        self.assertEqual(journal.__dict__, before)
        journal.checkpoint("valid", vertices, faces, preserve_boundary=True)
        self.assertEqual(journal.face_registry, before["face_registry"])

    def test_precompact_seal_rejects_later_topology_role_and_allocation(self):
        journal, vertices, faces = grid()
        journal.before_compact(vertices, faces)
        before = deepcopy(journal.__dict__)
        operations = [lambda: journal.checkpoint("late", vertices, faces[:1]),
                      lambda: journal.allocate(["late"], len(vertices), (0., 0., 0.), {"producer": "late"}),
                      lambda: journal.role(["late"], [0, 1], kind="late"),
                      lambda: journal.posterior(vertices, vertices, {})]
        for operation in operations:
            with self.assertRaisesRegex(ValueError, "sealed"):
                operation()
            self.assertEqual(journal.__dict__, before)
        packet = journal.compact(*compact(vertices, faces))
        self.assertEqual(len(packet["faces"]), len(faces))
        with self.assertRaises(ValueError):
            journal.compact(*compact(vertices, faces))

    def test_posterior_requires_unique_valid_original_parent_relation(self):
        mappings = ({0: 12, 1: 12}, {-1: 12}, {12: 12}, {0: 1}, {True: 12}, {0: 13}, {})
        for mapping in mappings:
            journal, vertices, _ = grid()
            before = deepcopy(journal.__dict__)
            with self.subTest(mapping=mapping), self.assertRaises(ValueError):
                journal.posterior(vertices, vertices + [(4., 5., 6.)], mapping)
            self.assertEqual(journal.__dict__, before)
        journal, vertices, _ = grid()
        parent = journal.points[0]["point_id"]
        journal.posterior(vertices, vertices + [(4., 5., 6.)], {0: 12, 1: 1})
        self.assertEqual(journal.points[-1]["provenance"]["parent_point_id"], parent)
        self.assertIn(12, journal.seam)

    def test_chart_aperture_breaks_are_explicit_and_never_bridged(self):
        journal, vertices, faces = grid()
        faces.pop(1)
        journal.before_compact(vertices, faces)
        packet = journal.compact(*compact(vertices, faces))
        role = packet["construction_roles"]["row/0"]
        ids = packet["point_ids"]
        self.assertEqual(role["segments"], [[ids[0], ids[1]], [ids[2], ids[3]]])
        self.assertFalse(role["complete_declared_path"])
        self.assertEqual(role["breaks"][0]["edge"], [1, 2])

    def test_removed_and_isolated_role_stations_remain_explicit_breaks(self):
        paths = {"row": {"slots": [0, 1, 2, 3, 4], "kind": "dense_chart_row", "closed": False}}
        result = role_segments(paths, {0, 1, 3, 4}, {(0, 1)})["row"]
        self.assertEqual(result["segments"], [[0, 1]])
        self.assertFalse(result["complete_declared_path"])
        self.assertEqual({item["slot"] for item in result["breaks"] if "slot" in item}, {2, 3, 4})

    def test_compaction_wrong_values_topology_and_remap_are_rejected(self):
        for kind in ("position", "topology", "missing", "duplicate", "false_bijection"):
            journal, vertices, faces = grid()
            journal.before_compact(vertices, faces)
            new_vertices, new_faces, remap = compact(vertices, faces)
            if kind == "position": new_vertices[0] = (.001, 0., 0.)
            if kind == "topology": new_faces[0] = tuple(reversed(new_faces[0]))
            if kind == "missing": del remap[1]
            if kind == "duplicate": remap[1] = 0
            if kind == "false_bijection": remap[0], remap[1] = remap[1], remap[0]
            before = deepcopy(journal.__dict__)
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                journal.compact(new_vertices, new_faces, remap)
            self.assertEqual(journal.__dict__, before)

    def test_unreported_allocations_duplicate_stages_and_bad_values_reject(self):
        journal, vertices, faces = grid()
        with self.assertRaisesRegex(ValueError, "Unreported"):
            journal.checkpoint("bad", vertices + [(0., 0., 0.)], faces)
        journal.checkpoint("once", vertices, faces)
        with self.assertRaisesRegex(ValueError, "duplicate stage"):
            journal.checkpoint("once", vertices, faces)
        for position in ((0., 1.), (0., True, 2.), (float("nan"), 0., 0.)):
            with self.subTest(position=position), self.assertRaises(ValueError):
                journal.allocate(["invalid"], len(vertices), position, {"producer": "invalid"})

    def test_reversed_duplicate_and_nonmanifold_topology_reject(self):
        for faces in ([(0, 1, 2, 3), (3, 2, 1, 0)], [(0, 1, 2), (0, 1, 3)]):
            with self.subTest(faces=faces), self.assertRaises(ValueError):
                topology(faces, 4)


if __name__ == "__main__":
    unittest.main()

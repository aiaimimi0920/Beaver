"""Exact scalar words and immutable face-corner allocation lifetime."""
import unittest
from construction_journal import ConstructionJournal
from construction_topology import role_segments


def quad():
    journal = ConstructionJournal({"test": "exact identity"}, [0, 1], [0, 1])
    vertices = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]
    for index, row in enumerate(vertices):
        journal.allocate(["point", index], index, row, {"constructor": "quad"})
    return journal, vertices


class ExactIdentityTests(unittest.TestCase):
    def test_signed_zero_and_scalar_type_are_not_exact_compaction(self):
        for changed in (-0.0, 0):
            journal, vertices = quad()
            journal.before_compact(vertices, [[0, 1, 2, 3]])
            altered = [list(row) for row in vertices]
            altered[0][0] = changed
            with self.subTest(changed=repr(changed)), self.assertRaisesRegex(ValueError, "point values"):
                journal.compact(altered, [[0, 1, 2, 3]], dict(enumerate(range(4))))

    def test_final_compacted_faces_require_exact_integer_indices(self):
        for changed in ([False, True, 2, 3], [0.0, 1.0, 2.0, 3.0]):
            journal, vertices = quad()
            journal.before_compact(vertices, [[0, 1, 2, 3]])
            with self.subTest(changed=changed), self.assertRaisesRegex(ValueError, "ordinal"):
                journal.compact(vertices, [changed], dict(enumerate(range(4))))

    def test_broken_closed_path_retains_surviving_wrap_edge(self):
        result = role_segments({"closed": {"slots": [0, 1, 2, 3, 4], "kind": "interface", "closed": True}},
                               {0, 1, 3, 4}, {(0, 1), (1, 3), (3, 4), (0, 4)})["closed"]
        self.assertEqual(result["segments"], [[3, 4, 0, 1]])
        self.assertFalse(result["closed"])
        self.assertFalse(result["complete_declared_path"])

    def test_face_rotation_preserves_born_corner_to_point_binding(self):
        journal, vertices = quad()
        journal.checkpoint("born", vertices, [[0, 1, 2, 3]])
        fid, born = next(iter(journal.face_registry.items()))
        before = dict(zip(born["corner_ids"], born["point_ids"]))
        journal.before_compact(vertices, [[1, 2, 3, 0]])
        result = journal.compact(vertices, [[1, 2, 3, 0]], dict(enumerate(range(4))))
        face = result["faces"][0]
        self.assertEqual(face["face_id"], fid)
        self.assertEqual(dict(zip(face["corner_ids"], face["point_ids"])), before)
        self.assertEqual(result["face_history"][fid], born)


if __name__ == "__main__":
    unittest.main()

"""Synthetic halo ownership regressions, without model fixtures."""
import copy
import unittest
from forehead_affected_domain import forehead_close_affected_domain


class ForeheadAffectedDomainTests(unittest.TestCase):
    def case(self):
        vertices = [
            [1., 0., 0.], [2., 0., 0.], [2., 1., 0.], [1., 1., 0.],
            [3., 0., 0.], [3., 1., 0.],
        ]
        faces = [[0, 1, 2, 3], [1, 4, 5, 2]]
        proposal = {
            "vertices": copy.deepcopy(vertices), "faces": copy.deepcopy(faces),
            "topology": {"vertices": copy.deepcopy(vertices), "face_owners": [0, 1]},
            "affected_old": [0], "affected_new": [0],
            "sentinel_fields": [[0., 0., 0.]] * 6,
        }
        return proposal, vertices, faces

    def test_changed_old_halo_closes_both_domains_and_preserves_input(self):
        proposal, source, faces = self.case()
        proposal["vertices"][1][1] += .01
        before = copy.deepcopy(proposal)
        result, audit = forehead_close_affected_domain(proposal, source, faces)
        self.assertEqual(proposal, before)
        self.assertEqual(result["affected_old"], [0, 1])
        self.assertEqual(result["affected_new"], [0, 1])
        for key in ("vertices", "faces", "topology", "sentinel_fields"):
            self.assertEqual(result[key], before[key])
        result["vertices"][0][0] = 999
        self.assertEqual(proposal, before)

    def test_guard_owner_and_all_children_are_closed(self):
        proposal, source, faces = self.case()
        proposal["affected_new"] = [1]
        proposal["faces"].append(copy.deepcopy(faces[1]))
        proposal["topology"]["face_owners"].append(1)
        result, _ = forehead_close_affected_domain(proposal, source, faces)
        self.assertEqual(result["affected_old"], [0, 1])
        self.assertEqual(result["affected_new"], [0, 1, 2])

    def test_invalid_nonpositive_unsupported_owners_fail(self):
        for mutation in ("owner", "negative", "triangle", "missing", "fractional"):
            proposal, source, faces = self.case()
            if mutation == "owner":
                proposal["topology"]["face_owners"][0] = -1
            elif mutation == "fractional":
                proposal["topology"]["face_owners"][0] = .5
            elif mutation == "negative":
                source[0][0] = -1
                proposal["vertices"][0][0] = -1
                proposal["topology"]["vertices"][0][0] = -1
            elif mutation == "triangle":
                faces[0] = faces[0][:3]
            else:
                faces = None
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                forehead_close_affected_domain(proposal, source, faces)

    def test_moved_birth_and_owner_without_original_motion(self):
        proposal, source, faces = self.case()
        proposal["vertices"].append([2.1, .5, 0.])
        proposal["topology"]["vertices"].append([2., .5, 0.])
        proposal["faces"][1] = [1, 4, 5, 6]
        result, audit = forehead_close_affected_domain(proposal, source, faces)
        self.assertEqual(audit["changed_original_vertices"], [])
        self.assertEqual(result["affected_old"], [0, 1])
        self.assertEqual(result["affected_new"], [0, 1])


if __name__ == "__main__":
    unittest.main()

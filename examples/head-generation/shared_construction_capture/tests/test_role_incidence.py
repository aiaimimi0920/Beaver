"""No per-strip hiding, omitted ordinary supports, or nearest role matching."""
import unittest
from construction_roles import incident_supports, outer_correspondence


def grid(rows, cols):
    names = ["row/" + str(row) for row in range(rows)]
    points = [[f"point@{row}/{col}" for col in range(cols)] for row in range(rows)]
    roles = {name: {"kind": "outer_constructor_row", "complete_declared_path": True, "segments": [path]} for name, path in zip(names, points)}
    faces = [{"face_id": f"face@{row}/{col}", "point_ids": [points[row][col], points[row][col+1], points[row+1][col+1], points[row+1][col]]}
             for row in range(rows-1) for col in range(cols-1)]
    return roles, faces, names


class RoleIncidenceTests(unittest.TestCase):
    def test_two_dimensions_cover_whole_declared_group_and_all_supports(self):
        for rows, cols in ((3, 4), (5, 7)):
            roles, faces, names = grid(rows, cols)
            result = outer_correspondence(roles, faces, names)
            self.assertEqual(len(result["rungs"]), (rows-1)*cols)
            self.assertEqual(result["support"]["support_quads"], faces)

    def test_single_band_and_missing_or_extra_rungs_reject(self):
        roles, faces, names = grid(4, 4)
        with self.assertRaisesRegex(ValueError, "at least two"):
            outer_correspondence(roles, faces, names[:2])
        with self.assertRaisesRegex(ValueError, "Incomplete declared"):
            outer_correspondence(roles, faces, names[:3])
        with self.assertRaisesRegex(ValueError, "adjacency"):
            outer_correspondence(roles, faces[:-1], names)
        with self.assertRaisesRegex(ValueError, "adjacency"):
            outer_correspondence(roles, faces, names[::2] + names[1::2])

    def test_stale_role_and_nonquad_incident_support_reject(self):
        roles, faces, names = grid(3, 4)
        roles[names[1]]["complete_declared_path"] = False
        with self.assertRaisesRegex(ValueError, "deleted/split"):
            outer_correspondence(roles, faces, names)
        faces[0]["point_ids"].pop()
        with self.assertRaisesRegex(ValueError, "nonquad"):
            incident_supports(faces, [faces[0]["point_ids"][0]])


if __name__ == "__main__":
    unittest.main()

import hashlib
import tempfile
import unittest
from pathlib import Path

from replay_v204 import replay_rows, validate_paths, write_new


class ReplayTests(unittest.TestCase):
    def test_nonuniform_weights_and_ordered_replacement(self):
        points = [[0, 0, 0], [4, 0, 0], [0, 0, 8]]
        faces = [[1, 2, 0], [0, 1, 2]]
        recipe = dict(station=0, carrier_triangle_index=0, carrier_triangle=[1, 0, 2],
                      weights=[.25, .70, .05], removed_authored_faces=[0],
                      replacement_faces=[[2, 1, 0]], replacement_donors=[0])
        positions, output, donors = replay_rows(points, faces, [[1, 0, 2]], recipe)
        self.assertEqual(positions, [[1., 0., .4], points[1], points[2]])
        self.assertEqual(output, [[0, 1, 2], [2, 1, 0]])
        self.assertEqual(donors, [1, 0])
        self.assertEqual(points[0], [0, 0, 0])
        with self.assertRaisesRegex(ValueError, "carrier topology"):
            replay_rows(points, faces, [[2, 0, 1]], recipe)

    def test_source_identity_and_external_new_path(self):
        with tempfile.TemporaryDirectory() as temp:
            outside = Path(temp)
            checkout = outside / "checkout"
            checkout.mkdir()
            source = outside / "source.blend"
            target = outside / "target.blend"
            source.write_bytes(b"synthetic")
            digest = hashlib.sha256(b"synthetic").hexdigest()
            self.assertEqual(validate_paths(source, target, checkout, digest),
                             (source.resolve(), target.resolve()))
            with self.assertRaisesRegex(ValueError, "authorized"):
                validate_paths(source, target, checkout, "0" * 64)
            with self.assertRaisesRegex(ValueError, "outside"):
                validate_paths(source, checkout / "in.blend", checkout, digest)
            with self.assertRaisesRegex(ValueError, "new"):
                validate_paths(source, source, checkout, digest)

    def test_exclusive_copy_does_not_replace_late_destination(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = [Path(temp) / s for s in ("verified.blend", "review.blend")]
            source.write_bytes(b"verified")
            write_new(source, target)
            self.assertEqual(target.read_bytes(), b"verified")
            source.write_bytes(b"different")
            with self.assertRaises(FileExistsError):
                write_new(source, target)
            self.assertEqual(target.read_bytes(), b"verified")


if __name__ == "__main__":
    unittest.main()

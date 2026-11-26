import hashlib
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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

    def test_partial_copy_leaves_no_output_and_can_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = [Path(temp) / s for s in ("verified.blend", "review.blend")]
            source.write_bytes(b"verified")

            def interrupted(src, dst):
                dst.write(src.read(4))
                raise OSError("interrupted copy")

            with mock.patch("replay_v204.shutil.copyfileobj", side_effect=interrupted):
                with self.assertRaisesRegex(OSError, "interrupted copy"):
                    write_new(source, target)
            self.assertEqual(set(Path(temp).iterdir()), {source})
            write_new(source, target)
            self.assertEqual(target.read_bytes(), b"verified")

    def test_incomplete_copy_without_exception_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = [Path(temp) / s for s in ("verified.blend", "review.blend")]
            source.write_bytes(b"verified")
            with mock.patch("replay_v204.shutil.copyfileobj"):
                with self.assertRaisesRegex(OSError, "does not match"):
                    write_new(source, target)
            self.assertEqual(set(Path(temp).iterdir()), {source})

    def test_late_collision_preserves_other_writers_file(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = [Path(temp) / s for s in ("verified.blend", "review.blend")]
            source.write_bytes(b"verified")
            link = os.link

            def collide(pending, output):
                output.write_bytes(b"other writer")
                link(pending, output)

            with mock.patch("replay_v204.os.link", side_effect=collide):
                with self.assertRaises(FileExistsError):
                    write_new(source, target)
            self.assertEqual(target.read_bytes(), b"other writer")
            self.assertEqual(set(Path(temp).iterdir()), {source, target})

    def test_link_unavailable_does_not_fall_back_to_copying_output(self):
        with tempfile.TemporaryDirectory() as temp:
            source, target = [Path(temp) / s for s in ("verified.blend", "review.blend")]
            source.write_bytes(b"verified")
            with mock.patch("replay_v204.os.link", side_effect=OSError("not supported")):
                with self.assertRaisesRegex(OSError, "filesystem that supports hard links"):
                    write_new(source, target)
            self.assertEqual(set(Path(temp).iterdir()), {source})


if __name__ == "__main__":
    unittest.main()

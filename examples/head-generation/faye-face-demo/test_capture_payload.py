"""Contract and complete PNG payload regressions, using synthetic test pixels only."""
from pathlib import Path
import struct
import tempfile
import unittest
import zlib
import verify_capture as capture

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def chunk(kind, payload):
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xffffffff)


def png(raw=None, compressed=None, color=2):
    channels = 3 if color == 2 else 4
    raw = b"".join(bytes([method]) + bytes(range(channels)) for method in range(5)) if raw is None else raw
    compressed = zlib.compress(raw) if compressed is None else compressed
    header = chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 5, 8, color, 0, 0, 0))
    return SIGNATURE + header + chunk(b"IDAT", compressed) + chunk(b"IEND", b"")


class CapturePayloadTests(unittest.TestCase):
    def test_manifest_contract_requires_exact_version_and_scope(self):
        valid = {"schema_version": 1, "speech_aligned": False, "continuous_animation": False,
                 "rendering_method": "forward_plus", "rendering_driver": "vulkan"}
        capture.verify_contract(valid)
        for version in [True, 1.0, 0, 2, "1", None]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                capture.verify_contract({**valid, "schema_version": version})
        for method, driver in [("gl_compatibility", "opengl3"), ("mobile", "vulkan"), ("forward_plus", "opengl3")]:
            with self.assertRaises(ValueError):
                capture.verify_contract({**valid, "rendering_method": method, "rendering_driver": driver})
        for field in valid:
            with self.subTest(missing=field), self.assertRaises(ValueError):
                capture.verify_contract({key: value for key, value in valid.items() if key != field})
        for field in ["speech_aligned", "continuous_animation"]:
            for value in [True, 0, None, "false"]:
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    capture.verify_contract({**valid, field: value})

    def test_frames_are_bound_to_manifest_directory_and_index(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            manifest = root / "capture/manifest.json"
            self.assertEqual(capture.frame_path(root, manifest, "res://capture/frame_000.png", 0),
                             root / "capture/frame_000.png")
            for path in ["res://other/frame_000.png", "res://capture/frame_001.png", "res://capture/image.png"]:
                with self.subTest(path=path), self.assertRaises(ValueError):
                    capture.frame_path(root, manifest, path, 0)

    def test_frame_symlink_cannot_redirect_to_other_workspace_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "capture").mkdir()
            (root / "other.png").write_bytes(png())
            (root / "capture/frame_000.png").symlink_to(root / "other.png")
            with self.assertRaises(ValueError):
                capture.frame_path(root, root / "capture/manifest.json", "res://capture/frame_000.png", 0)

    def test_complete_rgb_and_rgba_png_with_every_filter(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            for color in [2, 6]:
                path.write_bytes(png(color=color))
                self.assertEqual(capture.verify_png(path), (1, 5))

    def test_malformed_png_payloads_are_rejected_even_with_valid_headers(self):
        good = png()
        crc_bad = bytearray(good)
        crc_bad[45] ^= 1
        invalid = [good[:24], good[:-1], good + b"trailing", bytes(crc_bad),
                   png(compressed=b"not zlib"), png(raw=b"\x00"), png(raw=b"\x00" * 21),
                   png(raw=b"\x05\x00\x00\x00" * 5),
                   png(compressed=zlib.compress(b"\x00" * 20) + b"trailing"),
                   png(raw=b"\x00" * 100000)]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            for index, data in enumerate(invalid):
                path.write_bytes(data)
                with self.subTest(index=index), self.assertRaises(ValueError):
                    capture.verify_png(path)

    def test_srgb_is_validated_and_other_ancillary_chunks_are_rejected(self):
        good = png()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(good[:33] + chunk(b"sRGB", b"\x00") + good[33:])
            self.assertEqual(capture.verify_png(path), (1, 5))
            for middle in [chunk(b"sRGB", b""), chunk(b"sRGB", b"\x04"),
                           chunk(b"sRGB", b"\x00") * 2, chunk(b"tRNS", b"x")]:
                path.write_bytes(good[:33] + middle + good[33:])
                with self.assertRaises(ValueError):
                    capture.verify_png(path)
            path.write_bytes(good[:-12] + chunk(b"sRGB", b"\x00") + good[-12:])
            with self.assertRaises(ValueError):
                capture.verify_png(path)
            rgba = png(color=6)
            path.write_bytes(rgba[:-12] + chunk(b"tRNS", b"x") + rgba[-12:])
            with self.assertRaises(ValueError):
                capture.verify_png(path)

    def test_png_chunk_order_and_critical_chunks_are_checked(self):
        header = png()[8:33]
        compressed = zlib.compress(b"\x00" * 20)
        end = chunk(b"IEND", b"")
        invalid = [SIGNATURE + header + header + chunk(b"IDAT", compressed) + end,
                   SIGNATURE + header + chunk(b"ABCD", b"") + chunk(b"IDAT", compressed) + end,
                   SIGNATURE + header + chunk(b"IDAT", compressed[:2]) + chunk(b"tEXt", b"a\0b")
                   + chunk(b"IDAT", compressed[2:]) + end,
                   SIGNATURE + header + chunk(b"IDAT", compressed) + chunk(b"IEND", b"x")]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            for index, data in enumerate(invalid):
                path.write_bytes(data)
                with self.subTest(index=index), self.assertRaises(ValueError):
                    capture.verify_png(path)


if __name__ == "__main__":
    unittest.main()

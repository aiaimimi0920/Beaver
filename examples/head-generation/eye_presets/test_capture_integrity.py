"""Capture manifest guards reject contaminated views; no aesthetic inference."""

import hashlib
import base64
import json
from pathlib import Path
import tempfile
import unittest
from verify_capture import expected_views, verify_capture


class CaptureIntegrityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "model.glb").write_bytes(b"fixture model")
        source = hashlib.sha256(b"fixture model").hexdigest()
        png = base64.b64decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aS0sAAAAASUVORK5CYII="
        )
        captures = []
        for i, (yaw, pitch, left, right, mouth, mode) in enumerate(
            sorted(expected_views())
        ):
            (self.root / f"{i}.png").write_bytes(png)
            captures.append(
                dict(
                    requested=dict(
                        left=left,
                        right=right,
                        mouth=mouth,
                        yaw=yaw,
                        pitch=pitch,
                        mode=mode,
                    ),
                    actual_weights=[left, right, mouth],
                    actual_view=dict(
                        mode=mode,
                        viewport_size=[1, 1],
                        face_visible=True,
                        camera_pitch=pitch,
                        body_visible=False,
                        hair_visible=False,
                        projection=1,
                        size=1,
                        yaw=yaw,
                        pitch=pitch,
                    ),
                    image=f"res://{i}.png",
                    sha256=hashlib.sha256(png).hexdigest(),
                )
            )
        self.data = dict(
            source="res://model.glb",
            source_before=source,
            source_after=source,
            captures=captures,
        )
        self.path = self.root / "manifest.json"

    def verify(self):
        self.path.write_text(json.dumps(self.data))
        return verify_capture(self.root, self.path)

    def test_valid_state(self):
        self.assertEqual(self.verify()["captures_verified"], 84)

    def test_rejects_visibility_projection_pose_and_zoom_changes(self):
        view = self.data["captures"][3]["actual_view"]
        for key, value in [
            ("hair_visible", True),
            ("body_visible", True),
            ("projection", 0),
            ("yaw", 5),
            ("pitch", 5),
            ("size", 2),
        ]:
            previous = view[key]
            view[key] = value
            with self.assertRaises(ValueError):
                self.verify()
            view[key] = previous

    def test_rejects_extended_view_mismatches(self):
        view = self.data["captures"][3]["actual_view"]
        for key, value in [
            ("mode", "wrong"),
            ("face_visible", False),
            ("hair_visible", None),
            ("body_visible", 0),
            ("viewport_size", [2, 2]),
            ("camera_pitch", 9),
        ]:
            previous = view[key]
            view[key] = value
            with self.assertRaises(ValueError):
                self.verify()
            view[key] = previous

    def test_rejects_unexpected_source(self):
        self.verify()
        with self.assertRaises(ValueError):
            verify_capture(self.root, self.path, "res://other.glb")

    def test_rejects_missing_view_evidence(self):
        del self.data["captures"][0]["actual_view"]
        with self.assertRaises(ValueError):
            self.verify()

    def test_rejects_weight_mismatch_and_nan(self):
        for value in (1, float("nan")):
            self.data["captures"][0]["actual_weights"][0] = value
            with self.assertRaises(ValueError):
                self.verify()

    def test_rejects_changed_source(self):
        (self.root / "model.glb").write_bytes(b"changed")
        with self.assertRaises(ValueError):
            self.verify()

    def test_rejects_changed_or_duplicate_image(self):
        self.data["captures"][1]["image"] = self.data["captures"][0]["image"]
        with self.assertRaises(ValueError):
            self.verify()
        self.data["captures"][1]["image"] = "res://1.png"
        (self.root / "1.png").write_bytes(b"changed")
        with self.assertRaises(ValueError):
            self.verify()

    def test_rejects_incomplete_capture_set(self):
        self.data["captures"].pop()
        with self.assertRaises(ValueError):
            self.verify()

    def test_rejects_repeated_pose_schedule(self):
        self.data["captures"][1]["requested"] = self.data["captures"][0][
            "requested"
        ].copy()
        with self.assertRaises(ValueError):
            self.verify()

    def test_rejects_resource_escape(self):
        self.data["source"] = "res://../model.glb"
        with self.assertRaises(ValueError):
            self.verify()


if __name__ == "__main__":
    unittest.main()

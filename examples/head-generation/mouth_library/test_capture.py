"""Synthetic evidence checks, never counted as native render evidence."""

import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from PIL import Image
from capture_schedule import schedule
from mouth_motion import validate_weights
from verify_capture import HERE, digest, verify_capture


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / "showcase_extensions").mkdir()
        bindings = {
            "schedule_sha256": (
                "mouth_capture_schedule.json",
                "mouth_capture_schedule.json",
            ),
            "controller_sha256": ("researched_mouth_review.gd", "mouth_review.gd"),
            "probe_sha256": ("researched_mouth_probe.gd", "mouth_probe.gd"),
        }
        self.data = {}
        for key, (installed, source) in bindings.items():
            shutil.copy(
                HERE / "runtime" / source, self.root / "showcase_extensions" / installed
            )
            self.data[key] = digest(HERE / "runtime" / source)
        (self.root / "model.glb").write_bytes(b"synthetic fixture, not native model")
        sha = digest(self.root / "model.glb")
        self.data.update(
            source="res://model.glb",
            source_before=sha,
            source_after=sha,
            expected=92,
            speech_aligned=False,
            captures=[],
        )
        for i, item in enumerate(schedule()):
            path = self.root / f"{i}.png"
            Image.new("RGB", (2, 2)).save(path)
            self.data["captures"].append(
                {
                    "requested": item,
                    "actual_mouth": validate_weights(item["mouth"]),
                    "actual_blink": item["blink"],
                    "actual_view": {
                        "mode": item["mode"],
                        "projection": 1,
                        "size": 1.0,
                        "yaw": item["yaw"],
                        "pitch": item["pitch"],
                        "camera_pitch": item["pitch"],
                        "viewport_size": [2, 2],
                        "face_visible": True,
                        "body_visible": False,
                        "hair_visible": False,
                    },
                    "image": f"res://{i}.png",
                    "sha256": digest(path),
                }
            )

    def verify(self):
        path = self.root / "manifest.json"
        path.write_text(json.dumps(self.data))
        return verify_capture(self.root, path, "res://model.glb")

    def test_valid(self):
        self.assertEqual(self.verify()["captures_verified"], 92)

    def test_actual_weights_rejected(self):
        for value in [True, float("nan"), 0.2]:
            self.data["captures"][0]["actual_mouth"]["MouthRound"] = value
            with self.assertRaises(ValueError):
                self.verify()

    def test_view_rejected(self):
        view = self.data["captures"][0]["actual_view"]
        old = copy.deepcopy(view)
        for key, value in [
            ("camera_pitch", 5),
            ("yaw", 3),
            ("mode", "edges"),
            ("body_visible", True),
            ("face_visible", False),
            ("size", 2),
            ("viewport_size", [1, 1]),
        ]:
            view.clear()
            view.update(old)
            view[key] = value
            with self.assertRaises(ValueError):
                self.verify()

    def test_repeated_and_missing_schedule(self):
        self.data["captures"][1]["requested"] = self.data["captures"][0]["requested"]
        with self.assertRaises(ValueError):
            self.verify()
        self.data["captures"].pop()
        with self.assertRaises(ValueError):
            self.verify()

    def test_changed_code_or_source(self):
        (self.root / "showcase_extensions/researched_mouth_review.gd").write_text(
            "changed"
        )
        with self.assertRaises(ValueError):
            self.verify()

    def test_duplicate_image(self):
        self.data["captures"][1]["image"] = self.data["captures"][0]["image"]
        with self.assertRaises(ValueError):
            self.verify()

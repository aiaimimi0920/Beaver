"""CPU manifest tamper tests, not native image or geometry acceptance."""

import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from PIL import Image

from verify_spatial import BINDINGS, HERE, MOUTH, digest, verify_spatial


class SpatialTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.prefix = "assets/aster/general_face_eyes_combo911/"
        self.selection = dict(eyes="downturned", brows="soft-arched", nose="research-balanced",
                              mouth="research-line", ears="research-side-droop", face="research-compact-rounded")
        model = self.root / self.prefix
        model.mkdir(parents=True)
        (model / "aster_head.glb").write_bytes(b"CPU fixture only")
        (model / "aster_definition.tres").write_text("CPU fixture only")
        (model / "generation_report.json").write_text(json.dumps({
            "face_components": {"selection": self.selection}, "new_glb_sha256": digest(model / "aster_head.glb")}))
        for name in BINDINGS:
            path = self.root / name[6:]
            path.parent.mkdir(parents=True, exist_ok=True)
            original = HERE / path.name
            if original.is_file():
                shutil.copyfile(original, path)
            else:
                path.write_text("CPU binding fixture only")
        paths = {"model": "res://" + self.prefix + "aster_head.glb",
                 "definition": "res://" + self.prefix + "aster_definition.tres",
                 "report": "res://" + self.prefix + "generation_report.json"}
        hashes = {k: digest(self.root / v[6:]) for k, v in paths.items()}
        bindings = {v: digest(self.root / v[6:]) for v in BINDINGS}
        folder = self.root / "artifacts/ear_spatial/spatial-test"
        folder.mkdir(parents=True)
        self.manifest = folder / "manifest.json"
        captures = []
        for i, item in enumerate(json.loads((HERE / "ear_spatial_schedule.json").read_text())):
            image = folder / f"frame_{i:03d}.png"
            Image.new("RGB", (1180, 812), "white").save(image)
            captures.append({"requested": item, "actual_mouth": {k: item["mouth"].get(k, 0.) for k in MOUTH},
                             "actual_blink": [0., 0.], "expected_target": [0., 0., 0.],
                             "expected_position": [0., 0., 1.], "actual_view": {
                                 "target": [0., 0., 0.], "position": [0., 0., 1.],
                                 "mode": item["mode"], "projection": 1, "size": .4 * item["scale"],
                                 "yaw": item["yaw"], "pitch": item["pitch"], "camera_pitch": item["pitch"],
                                 "viewport_size": [1180, 812], "face_visible": True,
                                 "body_visible": False, "hair_visible": False},
                             "image": "res://" + str(image.relative_to(self.root)), "sha256": digest(image)})
        self.data = {"schema_version": 1, "expected": 18, "speech_aligned": False, "continuous_animation": False,
                     "source": paths["model"], "definition": paths["definition"], "report": paths["report"],
                     "source_before": hashes, "source_after": hashes, "bindings_before": bindings,
                     "bindings_after": bindings, "base_view_size": .4, "captures": captures}

    def verify(self, value):
        self.manifest.write_text(json.dumps(value))
        return verify_spatial(self.root, self.manifest, self.prefix, self.selection)

    def test_supplement_has_profile_edges_and_two_distances(self):
        result = self.verify(self.data)
        self.assertEqual(result["captures_verified"], 18)
        rows = [x["requested"] for x in self.data["captures"]]
        self.assertEqual({x["scale"] for x in rows}, {.75, 1., 1.5})
        self.assertEqual(len([x for x in rows if x["mode"] == "edges" and abs(x["yaw"]) == 90]), 6)

    def test_actual_pose_identity_and_provenance_tampering_rejected(self):
        for mutate in (
            lambda d: d.update(source="res://wrong.glb"),
            lambda d: d.update(schema_version=True),
            lambda d: d["captures"][0]["actual_view"].update(target=[1., 0., 0.]),
            lambda d: d["captures"][0]["actual_view"].update(yaw=0),
            lambda d: d["captures"][0]["actual_view"].update(projection=True),
            lambda d: d["captures"][6]["actual_view"].update(size=.4),
            lambda d: d["captures"][0]["actual_mouth"].update(MouthOpen=.2),
            lambda d: d["captures"][0].update(sha256="0" * 64),
            lambda d: d.update(bindings_after={}),
            lambda d: d.update(continuous_animation=True),
        ):
            value = copy.deepcopy(self.data)
            mutate(value)
            with self.assertRaises(ValueError):
                self.verify(value)

    def test_invalid_png_and_dimensions_rejected_even_with_matching_hash(self):
        image = self.root / self.data["captures"][0]["image"][6:]
        image.write_bytes(b"not a PNG")
        self.data["captures"][0]["sha256"] = digest(image)
        with self.assertRaises(ValueError):
            self.verify(self.data)
        Image.new("RGB", (320, 240), "white").save(image)
        self.data["captures"][0]["sha256"] = digest(image)
        with self.assertRaises(ValueError):
            self.verify(self.data)

    def test_missing_and_malformed_capture_records_rejected(self):
        for value in ([], None, {**self.data, "captures": []}):
            with self.assertRaises(ValueError):
                self.verify(value)
        value = copy.deepcopy(self.data)
        value["captures"][0] = None
        with self.assertRaises(ValueError):
            self.verify(value)


if __name__ == "__main__":
    unittest.main()

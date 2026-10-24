import importlib.util
import json
import hashlib
import shutil
from unittest.mock import patch
from pathlib import Path
import tempfile
import unittest

from compose import HERE, BROW_KEYS, resolve, verify_snapshot


class ResearchedBrowTests(unittest.TestCase):
    def setUp(self):
        self.selection = json.loads((HERE / "example-selection.json").read_text())
        self.catalog = json.loads((HERE / "catalog.json").read_text())

    def test_all_presets_owned_snapshotted_and_legacy_unchanged(self):
        old = resolve({**self.selection, "brows": "soft-level"})
        self.assertNotIn("brow_design", old)
        for id in self.catalog["brows"]:
            result = resolve({**self.selection, "brows": id})
            self.assertEqual(verify_snapshot(result), result)
            for field in ("surface_parameters", "silhouette_parameters", "lip_parameters", "component_parameters"):
                self.assertEqual(result[field], old[field])
            self.assertEqual({k: v for k, v in result["eye_parameters"].items() if k not in BROW_KEYS}, {k: v for k, v in old["eye_parameters"].items() if k not in BROW_KEYS})
            if id.startswith("research-"):
                self.assertEqual(len(result["brow_source_sha256"]), 4)

    def test_bounded_controls_cannot_replace_kind_or_other_slot(self):
        choice = {**self.selection, "brows": "research-curl"}
        a, b = resolve(choice), resolve(choice, brow_overrides={"curl_turns": .8})
        self.assertNotEqual(a["composition_sha256"], b["composition_sha256"])
        self.assertEqual(b["brow_design"]["curl_turns"], .8)
        for bad in ({"kind": "symbols"}, {"nose_bridge_scale": 1}, {"width_scale": float("nan")}, {"detail_count": 2.5}):
            with self.assertRaises(ValueError):
                resolve(choice, brow_overrides=bad)
        with self.assertRaises(ValueError):
            resolve({**self.selection, "brows": "soft-level"}, brow_overrides={})

    def test_source_fingerprints_survive_crlf_checkout(self):
        choice = {**self.selection, "brows": "research-linear-fine"}
        expected = resolve(choice)["brow_source_sha256"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            component = root / "face_components"
            shutil.copytree(HERE, component)
            (root / "semantic_eyes").mkdir()
            shutil.copy2(HERE.parent / "semantic_eyes/sampling_contract.py", root / "semantic_eyes/sampling_contract.py")
            library = root / "brow_library"
            shutil.copytree(HERE.parent / "brow_library", library)
            for name in expected:
                source = (library / name).read_text(encoding="utf-8")
                (library / name).write_bytes(source.replace("\n", "\r\n").encode("utf-8"))
            with patch("compose.HERE", component):
                actual = resolve(choice)["brow_source_sha256"]
            self.assertEqual(actual, expected)
            for name, digest in actual.items():
                normalized = (library / name).read_text(encoding="utf-8")
                self.assertEqual(digest, hashlib.sha256(normalized.encode("utf-8")).hexdigest())

    def test_eye_switch_preserves_brow_design(self):
        expected = resolve({**self.selection, "brows": "research-symbols"})["brow_design"]
        for eyes in self.catalog["eyes"]:
            self.assertEqual(resolve({**self.selection, "eyes": eyes, "brows": "research-symbols"})["brow_design"], expected)


if __name__ == "__main__":
    unittest.main()

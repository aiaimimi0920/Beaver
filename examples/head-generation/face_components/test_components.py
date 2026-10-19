import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from compose import HERE, SLOTS, BROW_KEYS, digest, resolve, verify_snapshot
from bundle import build_request


class CompositionTests(unittest.TestCase):
    def setUp(self):
        self.selection = json.loads((HERE / "example-selection.json").read_text())

    def test_six_explicit_slots_required(self):
        for slot in SLOTS:
            choice = dict(self.selection)
            del choice[slot]
            with self.assertRaises(ValueError):
                resolve(choice)
        with self.assertRaises(ValueError):
            resolve({**self.selection, "hair": "any"})

    def test_unknown_id_and_wrong_type_rejected(self):
        for value in (None, True, [], "../somewhere"):
            with self.assertRaises(ValueError):
                resolve({**self.selection, "eyes": value})

    def test_brow_choice_changes_only_brow_controls(self):
        a = resolve(self.selection)
        b = resolve({**self.selection, "brows": "soft-level"})
        changed = {
            k
            for k in a["eye_parameters"]
            if a["eye_parameters"][k] != b["eye_parameters"][k]
        }
        self.assertEqual(changed, {"brow_rise_offset", "brow_arch"})
        for key in ("surface_parameters", "silhouette_parameters", "lip_parameters"):
            self.assertEqual(a[key], b[key])

    def test_eye_choice_does_not_replace_selected_brow(self):
        catalog = json.loads((HERE / "catalog.json").read_text())
        expected = resolve(self.selection)["eye_parameters"]
        for eye in catalog["eyes"]:
            result = resolve({**self.selection, "eyes": eye})
            self.assertEqual(
                {k: result["eye_parameters"][k] for k in BROW_KEYS},
                {k: expected[k] for k in BROW_KEYS},
            )

    def test_face_and_mouth_are_independent(self):
        a = resolve(self.selection)
        b = resolve(
            {**self.selection, "face": "retained", "mouth": "soft-upper-transition"}
        )
        self.assertEqual(a["eye_parameters"], b["eye_parameters"])
        self.assertEqual(b["silhouette_parameters"], {})
        self.assertEqual(b["lip_parameters"], {"upper_lip_lateral_power": 2})

    def test_snapshot_is_detached_and_reproducible(self):
        choice = copy.deepcopy(self.selection)
        snapshot = resolve(choice)
        choice["brows"] = "unknown"
        self.assertEqual(snapshot, resolve(self.selection))
        self.assertEqual(verify_snapshot(snapshot), snapshot)
        snapshot["selection"]["nose"] = "other"
        with self.assertRaisesRegex(ValueError, "changed"):
            verify_snapshot(snapshot)

    def test_retained_nose_ear_scope_is_honest(self):
        catalog = json.loads((HERE / "catalog.json").read_text())
        for slot in ("nose", "ears"):
            self.assertEqual(catalog[slot], {"retained": {}})

    def test_output_is_fresh_and_cannot_overwrite_baseline(self):
        with tempfile.TemporaryDirectory() as folder:
            for prefix in ("assets/aster/general_face_eyes_795/", "../escape/"):
                with self.assertRaises(ValueError):
                    build_request(self.selection, prefix, folder)
            target = "assets/aster/general_face_eyes_combo_test/"
            (Path(folder) / target).mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, "fresh directory"):
                build_request(self.selection, target, folder)


class SDFBindingTests(unittest.TestCase):
    def test_sdf_module_does_not_import_generic_bundle(self):
        import sys
        import types

        sentinel = types.ModuleType("bundle")
        with patch.dict(sys.modules, {"bundle": sentinel}):
            spec = importlib.util.spec_from_file_location(
                "composition_sdf_test", HERE / "sdf_request.py"
            )
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            self.assertTrue(callable(module.build_request))
            self.assertIs(sys.modules["bundle"], sentinel)

    def test_receipt_binds_entire_composition_report(self):
        import hashlib
        from sdf_request import bind_composition_report

        with tempfile.TemporaryDirectory() as folder:
            report = Path(folder) / "generation_report.json"
            receipt = Path(folder) / "receipt.json"
            prefix = "assets/aster/general_face_eyes_combo_test/"
            report.write_text(json.dumps({"face_components": {"mouth": "retained"}}))
            job = {
                "status": "succeeded",
                "result": {
                    "success": True,
                    "outputs": [
                        {
                            "path": prefix + report.name,
                            "sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
                        }
                    ],
                },
            }
            receipt.write_text(json.dumps({"response": {"result": {"result": job}}}))
            self.assertIn(
                "face_components", bind_composition_report(report, prefix, receipt)
            )
            report.write_text(json.dumps({"face_components": {"mouth": "other"}}))
            with self.assertRaisesRegex(ValueError, "differs"):
                bind_composition_report(report, prefix, receipt)

    def test_changed_baseline_is_rejected(self):
        selection = json.loads((HERE / "example-selection.json").read_text())
        from compose import SURFACE

        read_bytes = Path.read_bytes

        def changed(path):
            if path == SURFACE / "face-baseline-795-prompt.zh-CN.md":
                return b"changed baseline"
            return read_bytes(path)

        with patch.object(Path, "read_bytes", changed):
            with self.assertRaisesRegex(ValueError, "baseline changed"):
                resolve(selection)


if __name__ == "__main__":
    unittest.main()

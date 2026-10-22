"""CPU contracts for ear selection and native-request packaging, not visual QA."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from compose import HERE, resolve
from ear_selection import LIBRARY, module, resolve_ear
from ear_evidence import validate_ear_report

BASE = json.loads((HERE.parent / "checkpoints/current/face_style.json").read_text())
SELECTION = json.loads((HERE / "example-selection.json").read_text())


class ResearchedEarTests(unittest.TestCase):
    def test_ear_only_capture_requires_requested_identity(self):
        from verify_run import verify_requested_composition

        snapshot = {
            "selection": SELECTION,
            "ear_design": {"kind": "standard-oval"},
            "composition_sha256": "bound-request",
        }
        spec = {"selection": SELECTION, "fit_policy": "legacy"}
        with self.assertRaises(ValueError):
            verify_requested_composition(snapshot, spec)
        spec["composition_sha256"] = "different-request"
        with self.assertRaises(ValueError):
            verify_requested_composition(snapshot, spec)
        spec["composition_sha256"] = "bound-request"
        verify_requested_composition(snapshot, spec)

    def fixture(self):
        snapshot = resolve({**SELECTION, "ears": "research-short-point"})
        style, record = module().apply_design(BASE, snapshot["ear_design"])
        report = {"ear_design": record, "facial_surface": {"ear_design": copy.deepcopy(record)}}
        sources = {n: (LIBRARY / n).read_text() for n in ("ear_design.py", "ear_profile.py")}
        return snapshot, report, style, sources

    def test_legacy_and_independent_controls(self):
        self.assertNotIn("ear_design", resolve(SELECTION))
        with self.assertRaises(ValueError):
            resolve(SELECTION, ear_overrides={})
        for name in module().SEEDS:
            selection = {**SELECTION, "ears": "research-" + name}
            first = resolve(selection)
            second = resolve(selection, ear_overrides={"posterior_scale": 1.1})
            self.assertNotEqual(first["composition_sha256"], second["composition_sha256"])
            for key in ("eye_parameters", "surface_parameters", "lip_parameters", "silhouette_parameters"):
                self.assertEqual(first[key], second[key])

    def test_plan_controls_and_invalid_inputs(self):
        from plan import build_plan
        selection = {**SELECTION, "ears": "research-long-leaf"}
        controls = {"tip_roundness": .2}
        self.assertEqual(build_plan(selection, records=[], ear_overrides=controls)["snapshot"],
                         resolve(selection, ear_overrides=controls))
        for controls in ({"kind": "other"}, {"tip_height": True}, {"tip_height": float("nan")},
                         {"root_height_offset": .2}, {"nose_width": 1}):
            with self.assertRaises(ValueError):
                resolve(selection, ear_overrides=controls)
        with self.assertRaises(ValueError):
            resolve_ear({"design": "long-leaf", "ear_span_scale": 1})

    def test_report_and_source_tampering(self):
        original = self.fixture()
        self.assertEqual(validate_ear_report(*original)["status"], "native_ear_calibration_bound")
        for part in ("style", "source", "report"):
            snapshot, report, style, sources = copy.deepcopy(original)
            if part == "style":
                style["calibration"]["ear_root_min_height"] += .01
            elif part == "source":
                sources["ear_profile.py"] += "\n# changed"
            else:
                report["ear_design"]["parameters"]["tip_height"] = .02
            with self.assertRaises(ValueError):
                validate_ear_report(snapshot, report, style, sources)

    def test_each_real_request_retains_ear_sources(self):
        from bundle import build_plan
        sys.path.insert(0, str(HERE.parent))
        from source_layout.package import literal_writes
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                required = ["project.godot", "authoring/head_generation_guide.md"]
                required += ["assets/aster/head_recovery_49/" + n for n in
                             ("aster_head_editable.blend", "aster_head.glb", "aster_definition.tres", "hair_base.png", "hair_ilm.png")]
                for relative in required:
                    path = root / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text("CPU-only fixture")
                for name in module().SEEDS:
                    selection = {**SELECTION, "ears": "research-" + name}
                    prefix = "assets/aster/general_face_eyes_combo_ear_cpu/"
                    request = build_plan(selection, prefix, root)["generation"]
                    writes = literal_writes(request["code"])
                    style = json.loads(writes[prefix + "face_style.json"][0])
                    preflight = json.loads(writes[prefix + "facial_surface_preflight.json"][0])
                    record = preflight["report"]["ear_design"]
                    report = {"ear_design": record, "facial_surface": preflight["report"]}
                    validate_ear_report(resolve(selection), report, style, preflight["ear_library_sources"])
                    self.assertLessEqual(len(request["outputs"]), 64)
                    self.assertLessEqual(len(request["code"].encode()), 1024 * 1024)
        finally:
            sys.path.remove(str(HERE.parent))


if __name__ == "__main__":
    unittest.main()

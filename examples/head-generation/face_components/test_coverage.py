import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from compose import digest, SLOTS
from coverage import verify_batch
from verify_run import verify_run, sha, NativeJobFailure


class CoverageTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.prefix = "assets/aster/general_face_eyes_combotest/"
        self.folder = self.root / self.prefix
        self.folder.mkdir(parents=True)
        selection = {k: "test" for k in SLOTS}
        self.spec = {
            "workspace": str(self.root),
            "prefix": self.prefix,
            "selection": selection,
            "fit_policy": "legacy",
            "generation_receipt": str(self.root / "gen.json"),
            "sdf_receipt": str(self.root / "sdf.json"),
            "eye_manifest": "unused",
            "mouth_manifest": "unused",
        }
        snapshot = {
            "selection": selection,
            "fit_policy": "legacy",
            "eye_parameters": {},
            "shading_policy": "smooth_proxy",
        }
        snapshot["composition_sha256"] = digest(snapshot)
        self.report = {
            "face_components": snapshot,
            "semantic_eyes": {"parameters": {}},
            "new_glb_sha256": "",
        }
        for name in (
            "aster_head.glb",
            "aster_head_runtime_morph.blend",
            "aster_definition.tres",
        ):
            self.write(name, "original")
        self.write("semantic_parameters.json", {})
        self.write("generation_report.json", self.report)
        generated = {self.prefix + p.name: sha(p) for p in self.folder.iterdir()}
        self.generated = generated
        self.job("gen.json", "gen", generated)
        binding = {
            "receipt_sha256": sha(self.root / "gen.json"),
            "job_id": "gen",
            "script_sha256": "script",
            "immutable_outputs": {
                self.prefix + n: generated[self.prefix + n]
                for n in (
                    "aster_head.glb",
                    "aster_head_runtime_morph.blend",
                    "semantic_parameters.json",
                )
            },
        }
        self.write("sdf/generation_binding.json", binding)
        self.write("sdf/input_hashes.json", generated)
        self.write("sdf/face_sdf360.png", "atlas")
        self.write("sdf/bake_report.json", {})
        self.report["semantic_eyes"]["sdf"] = {
            "policy": "smooth_proxy",
            "generation_binding": binding,
            "atlas_sha256": sha(self.folder / "sdf/face_sdf360.png"),
        }
        self.write("generation_report.json", self.report)
        self.write("aster_definition.tres", "baked")
        self.refresh_sdf()

    def write(self, name, value):
        p = self.folder / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(value if isinstance(value, str) else json.dumps(value))

    def job(self, name, identifier, outputs, status="succeeded"):
        (self.root / name).write_text(
            json.dumps(
                {
                    "response": {
                        "result": {
                            "result": {
                                "status": status,
                                "jobId": identifier,
                                "process": {
                                    "workspace": str(self.root),
                                    "scriptSha256": "script",
                                },
                                "result": {
                                    "success": status == "succeeded",
                                    "exitCode": 0 if status == "succeeded" else 1,
                                    "outputs": [
                                        {"path": k, "sha256": v}
                                        for k, v in outputs.items()
                                    ],
                                },
                            }
                        }
                    }
                }
            )
        )

    def refresh_sdf(self):
        names = ["generation_report.json", "aster_definition.tres"] + [
            str(p.relative_to(self.folder)) for p in (self.folder / "sdf").iterdir()
        ]
        self.job(
            "sdf.json", "sdf", {self.prefix + n: sha(self.folder / n) for n in names}
        )

    def verify(self):
        with patch(
            "verify_run.load_capture_verifier",
            return_value=lambda *args: {
                "captures_verified": 84 if args[-1] == "eye" else 18
            },
        ):
            return verify_run(self.spec)

    def test_chain(self):
        self.assertEqual(self.verify()["status"], "technically_verified")

    def test_changed_input_report(self):
        self.write(
            "sdf/input_hashes.json",
            {**self.generated, self.prefix + "generation_report.json": "wrong"},
        )
        self.refresh_sdf()
        with self.assertRaisesRegex(ValueError, "SDF input"):
            self.verify()

    def test_wrong_policy(self):
        self.report["semantic_eyes"]["sdf"]["policy"] = "wrong"
        self.write("generation_report.json", self.report)
        self.refresh_sdf()
        with self.assertRaisesRegex(ValueError, "policy"):
            self.verify()

    def test_changed_glb(self):
        self.write("aster_head.glb", "changed")
        with self.assertRaisesRegex(ValueError, "Changed"):
            self.verify()

    def test_wrong_selection(self):
        self.spec["selection"] = {**self.spec["selection"], "eyes": "other"}
        with self.assertRaisesRegex(ValueError, "selection"):
            self.verify()

    def test_native_failure(self):
        self.job("gen.json", "gen", {}, "failed")
        r = verify_batch([self.spec])
        self.assertEqual(r["runs"][0]["status"], "generation_failed")

    def test_missing(self):
        (self.root / "gen.json").unlink()
        self.assertEqual(verify_batch([self.spec])["runs"][0]["status"], "incomplete")

    def test_batch_isolation(self):
        r = verify_batch([{}, self.spec], lambda s: {"status": "technically_verified"})
        self.assertEqual(
            [x["status"] for x in r["runs"]],
            ["integrity_failed", "technically_verified"],
        )
        self.assertEqual(r["technically_verified_runs"], 1)


if __name__ == "__main__":
    unittest.main()

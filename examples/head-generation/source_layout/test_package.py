"""Synthetic receipt contract tests; not native model execution evidence."""

import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.package import modular_plan, packaging_request, literal_writes
from source_layout.evidence import verify_packaging

PREFIX = "assets/aster/general_face_eyes_combotest/"
PIPE = f"""OUT={PREFIX!r}
x=[]
for role in ["Face"]:
    x.append("assembly")
for role in ["Face"]:
    x.append("runtime")
x.append("export")
"""
PARAM = "def upper_shape(t,c):\n    return t\ndef upper_shape_derivative(t,c):\n    return 1\nimport copy\n"


def plan():
    sources = {
        "pipeline.py": PIPE,
        "semantic_parameters.py": PARAM,
        "input-manifest.json": "{}",
    }
    code = PIPE + "\n"
    for name, source in sources.items():
        code += f"Path(beaver_output({PREFIX + name!r})).write_text({source!r})\n"
    code += "report={}\n"
    paths = [
        *sources,
        "generation_report.json",
        "aster_head.glb",
        "aster_head_runtime_morph.blend",
    ]
    return modular_plan(
        {
            "code": code,
            "outputs": [{"path": PREFIX + p, "expectedSha256": None} for p in paths],
        },
        PREFIX,
    )


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.plan = plan()
        self.report = {"source_layout": self.plan["source_packaging"]["layout"]}
        literals = literal_writes(self.plan["generation"]["code"])
        for row in self.plan["generation"]["outputs"]:
            target = self.root / row["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(
                literals[row["path"]][0] if row["path"] in literals else "fixture"
            )
        (self.root / PREFIX / "generation_report.json").write_text(
            json.dumps(self.report)
        )
        self.gen = self.root / "generation-receipt.json"
        self.write_receipt(
            self.gen,
            "gen",
            [x["path"] for x in self.plan["generation"]["outputs"]],
            self.plan["source_packaging"]["generation_script_sha256"],
        )

    def write_receipt(self, path, job_id, names, script_hash="test-only"):
        job = {
            "jobId": job_id,
            "status": "succeeded",
            "process": {"workspace": str(self.root), "scriptSha256": script_hash},
            "result": {
                "success": True,
                "exitCode": 0,
                "outputs": [
                    {"path": name, "sha256": digest(self.root / name)} for name in names
                ],
            },
        }
        path.write_text(json.dumps({"response": {"result": {"result": job}}}))

    def request(self):
        return packaging_request(self.plan["source_packaging"], self.gen, self.root)

    def test_complete_chain_and_tampered_helper(self):
        request = self.request()
        self.assertEqual(len(request["outputs"]), 6)
        # Execute only the source-file packaging contract, no model engine.
        exec(
            request["code"],
            {
                "beaver_input": lambda p: str(self.root / p),
                "beaver_output": lambda p: str(self.root / p),
            },
        )
        packed = self.root / "pack-receipt.json"
        self.write_receipt(packed, "pack", [x["path"] for x in request["outputs"]])
        self.assertEqual(
            len(verify_packaging(self.report, self.root, PREFIX, self.gen, packed)), 6
        )
        sdf_report = copy.deepcopy(self.report)
        sdf_report["semantic_eyes"] = {"sdf": {"completed": True}}
        self.assertEqual(
            len(verify_packaging(sdf_report, self.root, PREFIX, self.gen, packed)), 6
        )
        for filename in (
            "pipeline.py",
            "semantic_parameters.py",
            "input-manifest.json",
        ):
            target = self.root / PREFIX / filename
            original = target.read_bytes()
            target.write_text("tampered")
            with self.assertRaises(ValueError):
                verify_packaging(sdf_report, self.root, PREFIX, self.gen, packed)
            target.write_bytes(original)
        original_receipt = self.gen.read_bytes()
        self.gen.write_bytes(original_receipt + b"\n")
        with self.assertRaises(ValueError):
            verify_packaging(sdf_report, self.root, PREFIX, self.gen, packed)
        self.gen.write_bytes(original_receipt)
        (self.root / PREFIX / "pipeline_runtime.py").write_text("changed")
        with self.assertRaises(ValueError):
            verify_packaging(self.report, self.root, PREFIX, self.gen, packed)

    def test_missing_receipt_output(self):
        data = json.loads(self.gen.read_text())
        data["response"]["result"]["result"]["result"]["outputs"].pop(0)
        self.gen.write_text(json.dumps(data))
        with self.assertRaises(ValueError):
            self.request()

    def test_changed_generation(self):
        (self.root / PREFIX / "pipeline.py").write_text("changed")
        with self.assertRaises(ValueError):
            self.request()

    def test_changed_plan_entry(self):
        self.plan["source_packaging"]["layout"]["records"][0]["entry_sha256"] = "bad"
        with self.assertRaises(ValueError):
            self.request()

    def test_unsafe_prefix(self):
        for bad in ("/tmp/", "assets/../escape/", PREFIX + "../"):
            self.plan["source_packaging"]["prefix"] = bad
            with self.assertRaises(ValueError):
                self.request()

    def test_symlink_rejected(self):
        target = self.root / PREFIX / "pipeline.py"
        raw = target.read_text()
        outside = self.root / "elsewhere.py"
        outside.write_text(raw)
        target.unlink()
        target.symlink_to(outside)
        with self.assertRaises(ValueError):
            self.request()

    def test_missing_packaging_rejected(self):
        with self.assertRaises(ValueError):
            verify_packaging(self.report, self.root, PREFIX, self.gen)

    def test_wrong_out_prefix(self):
        raw = {
            "code": PIPE
            + f'Path(beaver_output({PREFIX + "pipeline.py"!r})).write_text({PIPE!r})\n',
            "outputs": [],
        }
        with self.assertRaises(ValueError):
            modular_plan(raw, PREFIX.replace("test", "other"))

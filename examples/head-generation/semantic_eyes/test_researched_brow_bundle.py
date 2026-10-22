import ast
from pathlib import Path
import tempfile
import unittest

from bundle import build_request, REQUIRED


class ResearchedBundleTests(unittest.TestCase):
    def setUp(self):
        self.design = {"schema_version": 1, "kind": "symbols", "detail_count": 3}

    def test_protected_baselines_and_existing_outputs_refused(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for prefix in ("assets/aster/general_face_eyes_795/", "assets/aster/general_face_eyes_796/"):
                with self.assertRaisesRegex(ValueError, "fresh"):
                    build_request({}, prefix, root, brow_design=self.design)
            prefix = "assets/aster/general_face_eyes_existing/"
            (root / prefix).mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, "fresh"):
                build_request({}, prefix, root, brow_design=self.design)

    def test_runtime_is_packaged_without_extra_output_slots(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for relative in REQUIRED:
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"packaging-only fixture")
            plain = build_request({}, "assets/aster/general_face_eyes_plain/", root)
            new = build_request({}, "assets/aster/general_face_eyes_new/", root, brow_design=self.design)
            self.assertEqual(len(new["outputs"]), len(plain["outputs"]))
            self.assertLessEqual(len(new["outputs"]), 64)
            self.assertLess(len(new["code"].encode()), 1048576)
            self.assertIn('types.ModuleType("researched_brow_runtime")', new["code"])
            self.assertIn("RETAINED_BROW_DESIGN_SOURCES", new["code"])
            self.assertNotIn("researched_brow_runtime", plain["code"].split('RETAINED_UPPER_SHAPE_SOURCES')[0].split('Path(beaver_output')[0])
            ast.parse(new["code"])


if __name__ == "__main__":
    unittest.main()

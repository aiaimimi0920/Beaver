"""Real six-slot source packaging on two resolved shapes; no native execution."""

import ast
from pathlib import Path
import sys
import tempfile
import unittest

PARENT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PARENT / "face_components"))
from test_feature_flow_recipe import SELECTION, FLOW, fixture_root, emitted_sources
from bundle import build_plan
from shared_capture_adapter import POLICY, RUNTIME_MODULES, bootstrap, retained_sources
from source_layout.transport_sources import validate_native_frame


class PackagingTests(unittest.TestCase):
    def test_two_compositions_use_real_post_morph_boundary_and_existing_seed(self):
        source_hashes = []
        for face in ("research-compact-rounded", "research-straight-tapered"):
            with self.subTest(face=face), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                fixture_root(root)
                plan = build_plan({**SELECTION, "face": face},
                                  "assets/aster/general_face_eyes_combo_reviewbootstrap/", root,
                                  sampling_overrides=FLOW, shared_source_capture=POLICY, source_seed=True)
                self.assertLessEqual(len(plan["generation"]["outputs"]), 64)
                self.assertLessEqual(validate_native_frame(plan["generation"]), 1024 * 1024)
                sources = emitted_sources(plan)
                pipeline = sources["pipeline_runtime.py"]
                begin = pipeline.index("_shared_source_capture = begin_source_capture(")
                triangulate = pipeline.index("bmesh.ops.triangulate(")
                mouth = pipeline.index('report["mouth_motion"] = build_mouth_shapes(')
                finish = pipeline.index("finish_source_capture(members[0], _shared_source_capture)")
                save = pipeline.index('"aster_head_runtime_morph.blend"')
                self.assertLess(begin, triangulate)
                self.assertLess(mouth, finish)
                self.assertLess(finish, save)
                self.assertIn("authored_skin_ownership", pipeline)
                self.assertIn("_capture_sys.modules[_capture_name]", plan["generation"]["code"])
                self.assertNotIn("shared_facial_rails", pipeline)
                call = next(n for n in ast.walk(ast.parse(pipeline)) if isinstance(n, ast.Call)
                            and isinstance(n.func, ast.Name) and n.func.id == "begin_source_capture")
                source_hashes.append(ast.literal_eval(call.args[2])["source_sha256"])
                for name, source in sources.items():
                    compile(source, name, "exec")
        self.assertEqual(len(source_hashes), 2)
        if len(source_hashes) == 2:
            self.assertNotEqual(source_hashes[0], source_hashes[1])

    def test_retained_runtime_sources_load_without_executing_packager(self):
        source = "raise RuntimeError('must not execute retained packager')\n"
        source += "RETAINED_SHARED_CAPTURE_SOURCES = " + repr(retained_sources())
        scope = {}
        exec(bootstrap(source), scope)
        self.assertTrue(all(name in sys.modules for name in RUNTIME_MODULES))
        self.assertTrue(callable(sys.modules["shared_capture_native"].finish_source_capture))

    def test_default_unchanged_and_unsupported_policy_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root(root)
            args = (SELECTION, "assets/aster/general_face_eyes_combo_defaultcapture/", root)
            implicit = build_plan(*args)
            explicit = build_plan(*args, shared_source_capture=None)
            self.assertEqual(implicit, explicit)
            default_sources = emitted_sources(implicit)
            self.assertNotIn("_shared_source_capture =", default_sources["pipeline_runtime.py"])
            for policy, flow in (("unknown", FLOW), (POLICY, {})):
                with self.assertRaises(ValueError):
                    build_plan(*args, shared_source_capture=policy, sampling_overrides=flow, source_seed=True)


if __name__ == "__main__":
    unittest.main()

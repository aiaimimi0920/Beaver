"""Actual constructor adapter/seed packaging; no native or visual admission."""
import ast
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "face_components"))
from test_feature_flow_recipe import SELECTION, FLOW, fixture_root, emitted_sources
from bundle import build_plan
from source_layout.layout import reconstructed
from source_layout.transport_sources import validate_native_frame
from construction_adapter import POLICY, RUNTIME_MODULES, bootstrap, retained_sources
from construction_topology import digest


class PackagingTests(unittest.TestCase):
    def test_two_real_compositions_bind_constructor_and_actual_capture(self):
        for face in ("research-compact-rounded", "research-straight-tapered"):
            with self.subTest(face=face), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                fixture_root(root)
                plan = build_plan({**SELECTION, "face": face}, "assets/aster/general_face_eyes_combo_ccreview/", root,
                    sampling_overrides=FLOW, construction_identity_capture=POLICY,
                    shared_source_capture="POST_MORPH_SOURCE_CAPTURE_V1", source_seed=True)
                self.assertLessEqual(validate_native_frame(plan["generation"]), 1024 * 1024)
                self.assertLessEqual(len(plan["generation"]["outputs"]), 64)
                sources = emitted_sources(plan)
                record = next(r for r in plan["source_packaging"]["layout"]["records"]
                              if r.get("entry_name") == "shell.py")
                shell = reconstructed(sources, record)
                self.assertIn("_construction.chart_point(j, i, verts", shell)
                self.assertIn("_construction.outer_stage(outer_flow_report, verts, shell_faces)", shell)
                self.assertIn("_construction.compact(verts, shell_faces, flow_remap)", shell)
                self.assertNotIn("bind_forehead_paths", shell[:shell.index("def feature_flow_surface_point")])
                pipeline = sources["pipeline_runtime.py"]
                self.assertLess(pipeline.index("join_with_oral_capture(members)"), pipeline.index("invalidate_before_surgery("))
                self.assertLess(pipeline.index("capture_weld(members[0],"), pipeline.index("invalidate_before_surgery("))
                self.assertIn("finish_source_capture(members[0]", pipeline)
                self.assertIn("verify_oral_transfer(", pipeline)
                self.assertIn("construction=begin_oral(_construction, rings[0])", shell)
                for name, source in sources.items():
                    compile(source, name, "exec")

    def test_default_dispatch_and_unknown_policy_are_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root(root)
            args = (SELECTION, "assets/aster/general_face_eyes_combo_ccdefault/", root)
            implicit = build_plan(*args)
            explicit = build_plan(*args, construction_identity_capture=None)
            self.assertEqual(implicit, explicit)
            sources = emitted_sources(implicit)
            self.assertNotIn("construction_native", sources["pipeline_runtime.py"])
            for policy, flow in (("unknown", FLOW), (POLICY, {})):
                with self.assertRaises(ValueError):
                    build_plan(*args, construction_identity_capture=policy, sampling_overrides=flow, source_seed=True)

    def test_retained_runtime_loads_without_executing_packager(self):
        sources = retained_sources()
        inventory = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in sources.items()}
        binding = {"capture_inventory": inventory, "recipe": {"capture_inventory_sha256": digest(inventory)}}
        scope = {"retained": "raise RuntimeError('packager must not run')\nRETAINED_CONSTRUCTION_SOURCES = " + repr(sources)
                 + "\nRETAINED_CONSTRUCTION_BINDING = " + repr(binding)}
        exec(bootstrap("retained"), scope)
        self.assertTrue(all(name in sys.modules for name in RUNTIME_MODULES))
        self.assertTrue(callable(sys.modules["construction_native"].require_current_roles))
        with self.assertRaisesRegex(ValueError, "not admitted"):
            sys.modules["construction_native"].require_current_roles({"solver_ready": True})


if __name__ == "__main__":
    unittest.main()

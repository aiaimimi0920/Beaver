"""RNA stand-in lifecycle regressions, explicitly not actual native acceptance."""
import importlib.util
from pathlib import Path
import sys
from types import SimpleNamespace as N
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parents[2]
_original_path = sys.path[:]
try:
    sys.path.insert(0, str(HERE / "shared_source_capture"))
    sys.path.insert(0, str(HERE / "shared_source_capture/tests"))
    spec = importlib.util.spec_from_file_location("constructor_rna_fixture", HERE / "shared_source_capture/tests/test_native_capture.py")
    fixture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixture)
finally:
    sys.path[:] = _original_path
from construction_markers import MARKERS, attach, clear_verified, fields, read
from construction_native import HALVES, apply_modifiers, bind_half, invalidate_before_surgery, require_current_roles, join_objects
from construction_topology import digest
from test_mirror_contract import fixture as mirror_fixture


def native(rows=3, cols=4):
    mesh, _, _ = fixture.layout(rows, cols, .002)
    obj = fixture.native(mesh)
    obj.name = "Half"
    obj.as_pointer = lambda: id(obj)
    obj.data.as_pointer = lambda: id(obj.data)
    for collection in (obj.data.vertices, obj.data.polygons, obj.data.loops):
        for index, row in enumerate(collection):
            row.index = index
    obj.modifiers = [N(type="MIRROR", name="Mirror", use_axis=[True, False, False],
                       use_bisect_axis=[False] * 3, use_mirror_merge=True)]
    return obj


def values(obj):
    return {"POINT": list(range(1, len(obj.data.vertices)+1)),
            "FACE": list(range(1, len(obj.data.polygons)+1)),
            "CORNER": list(range(1, len(obj.data.loops)+1))}


class NativeLifecycleTests(unittest.TestCase):
    def tearDown(self):
        HALVES.clear()

    def test_marker_only_fields_and_cleanup_two_sizes(self):
        for rows, cols in ((3, 4), (5, 7)):
            obj = native(rows, cols)
            before = digest(fields(obj))
            attach(obj, values(obj))
            self.assertEqual(read(obj), values(obj))
            self.assertEqual(digest(fields(obj)), before)
            clear_verified(obj)
            self.assertEqual(digest(fields(obj)), before)
            self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))

    def test_collision_preserves_existing_attribute(self):
        obj = native()
        existing = obj.data.attributes.new(MARKERS["POINT"], "INT", "POINT")
        with self.assertRaisesRegex(ValueError, "collision"):
            attach(obj, values(obj))
        self.assertIs(obj.data.attributes.get(MARKERS["POINT"]), existing)

    def test_wrong_schema_rejects(self):
        obj = native()
        attach(obj, values(obj))
        obj.data.attributes[MARKERS["POINT"]].data_type = "FLOAT"
        with self.assertRaisesRegex(ValueError, "schema"):
            read(obj)
        clear_verified(obj)

    def test_failed_modifier_cleans_all_markers(self):
        obj = native()
        source, _, seam = mirror_fixture()
        packet = {**source, "seam_point_ids": seam, "construction_roles": {}}
        bind_half(obj, packet)
        def fail(**kwargs):
            raise RuntimeError("injected native modifier failure")
        fake_bpy = N(ops=N(object=N(modifier_apply=fail)))
        with patch.dict(sys.modules, {"bpy": fake_bpy}), self.assertRaisesRegex(RuntimeError, "injected"):
            apply_modifiers(obj)
        self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))
        self.assertNotIn("native_mirror", packet)

    def test_object_rename_replacement_and_stale_state_reject(self):
        obj = native()
        source, _, seam = mirror_fixture()
        packet = {**source, "seam_point_ids": seam, "construction_roles": {}}
        bind_half(obj, packet)
        fake_bpy = N(ops=N(object=N(modifier_apply=lambda **kwargs: None)))
        with patch.dict(sys.modules, {"bpy": fake_bpy}):
            obj.name = "Renamed"
            with self.assertRaisesRegex(ValueError, "renamed"):
                apply_modifiers(obj)
            obj.name = "Half"
            with self.assertRaisesRegex(ValueError, "borrowed"):
                apply_modifiers(native())
            invalidate_before_surgery([packet], "weld")
            with self.assertRaisesRegex(ValueError, "reused"):
                apply_modifiers(obj)

    def test_unreadable_fields_do_not_prevent_marker_removal(self):
        obj = native()
        attach(obj, values(obj))
        obj.data.attributes["NormalRegion"].data_type = "UNKNOWN"
        with self.assertRaisesRegex(ValueError, "Unsupported"):
            clear_verified(obj)
        self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))

    def test_join_failure_attempts_cleanup_on_every_live_object(self):
        first, second = native(), native()
        second.name = "Other"
        def fail():
            first.data.attributes["NormalRegion"].data_type = "UNKNOWN"
            raise RuntimeError("injected join failure")
        with patch.dict(sys.modules, {"bpy": N(ops=N(object=N(join=fail)))}):
            with self.assertRaisesRegex(RuntimeError, "every live object"):
                join_objects([first, second])
        for obj in (first, second):
            self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))

    def test_duplicate_constructor_instance_ids_reject_before_native_join(self):
        first, second = native(), native()
        second.name = "Other"
        source, _, seam = mirror_fixture()
        for obj in (first, second):
            packet = {**source, "seam_point_ids": seam, "construction_roles": {}, "native_mirror": {
                "face_map": {row["face_id"]: {"original": i} for i, row in enumerate(source["faces"])},
                "corner_map": {cid: {"original": {"index": i*4+j}} for i, row in enumerate(source["faces"]) for j, cid in enumerate(row["corner_ids"])}}}
            bind_half(obj, packet)
            HALVES[obj.as_pointer()].update(phase="mirrored", mirrored_point_ids=list(source["point_ids"]))
        calls = []
        with patch.dict(sys.modules, {"bpy": N(ops=N(object=N(join=lambda: calls.append(True))))}):
            with self.assertRaisesRegex(ValueError, "Duplicate constructor instance"):
                join_objects([first, second])
        self.assertEqual(calls, [])
        for obj in (first, second):
            self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))

    def test_uv_and_color_selector_metadata_is_in_marker_field_guard(self):
        obj = native()
        before = digest(fields(obj))
        obj.data.uv_layers[0].active_render = True
        self.assertNotEqual(digest(fields(obj)), before)
        before = digest(fields(obj))
        obj.data.color_attributes = N(active_color_name="Color", default_color_name="Other")
        self.assertNotEqual(digest(fields(obj)), before)

    def test_join_rejects_unverified_topology_or_mesh_replacement(self):
        for change in ("topology", "datablock"):
            HALVES.clear()
            obj = native()
            source, _, seam = mirror_fixture()
            packet = {**source, "seam_point_ids": seam, "construction_roles": {}}
            bind_half(obj, packet)
            HALVES[obj.as_pointer()].update(phase="mirrored", mirrored_point_ids=list(source["point_ids"]))
            if change == "topology":
                obj.data.loops[0].vertex_index = 1
            else:
                obj.data = native().data
            calls = []
            with patch.dict(sys.modules, {"bpy": N(ops=N(object=N(join=lambda: calls.append(True))))}):
                with self.subTest(change=change), self.assertRaisesRegex(ValueError, "topology changed|datablock replaced"):
                    join_objects([obj])
            self.assertEqual(calls, [])
            self.assertFalse(any(obj.data.attributes.get(name) for name in MARKERS.values()))

    def test_surgery_invalidates_even_counterfeit_ready_state(self):
        packet = {"solver_ready": True, "native_transport": {"stage": "join_verified"}}
        invalidate_before_surgery([packet], "weld")
        self.assertFalse(packet["solver_ready"])
        self.assertFalse(packet["native_transport"]["current_role_admission"])
        with self.assertRaisesRegex(ValueError, "not admitted"):
            require_current_roles(packet)


if __name__ == "__main__":
    unittest.main()

"""Injected failures exercise the actual emitted capture interval cleanup."""

import unittest
from fixtures import layout, triangulate
from test_native_capture import native
from shared_capture_adapter import adapt
from shared_capture_contract import MARKERS


class LifetimeTests(unittest.TestCase):
    def test_failures_before_triangulation_and_during_mouth_builder_cleanup(self):
        source, ownership, _ = layout(3, 4, .002)
        runtime, ids, shapes, _ = triangulate(source)
        for fail_before in (True, False):
            with self.subTest(before_triangulation=fail_before):
                obj, converted = native(source), native(runtime, ids, shapes)
                for i, face in enumerate(obj.data.polygons):
                    face.index = i
                script = '''for role in ["Face"]:
    if role == "Face":
        data = members[0].data
        _source_skin_quads=[(p.index,tuple(p.vertices)) for p in data.polygons if len(p.vertices)==4 and data.attributes['NormalRegion'].data[p.index].value in (1,2)]
        maybe_fail_before()
        members[0].data = converted.data
        data = members[0].data
        data.normals_split_custom_set(corner_normals)
        report["mouth_motion"] = build_mouth_shapes(members[0], semantic_part_ordinals, C)
'''
                pipeline = adapt({"pipeline.py": script})["pipeline.py"]
                def before():
                    if fail_before:
                        raise RuntimeError("injected before triangulation")
                def mouth(*args):
                    raise RuntimeError("injected mouth builder")
                scope = {"members": [obj], "converted": converted, "maybe_fail_before": before,
                         "build_mouth_shapes": mouth, "semantic_part_ordinals": {}, "C": {},
                         "corner_normals": [[0., 0., 1.]] * len(runtime["corners"]),
                         "report": {"ear_shell_membership": {"authored_skin_ownership": ownership}}}
                with self.assertRaisesRegex(RuntimeError, "injected"):
                    exec(compile(pipeline, "<emitted capture lifetime>", "exec"), scope)
                self.assertTrue(all(obj.data.attributes.get(name) is None for name in MARKERS.values()))
                self.assertNotIn("shared_source_capture", scope["report"])


if __name__ == "__main__":
    unittest.main()

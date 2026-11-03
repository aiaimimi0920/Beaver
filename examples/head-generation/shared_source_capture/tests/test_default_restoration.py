"""Narrow native BMesh default elision: restore real fields, never waive transport."""

from copy import deepcopy
import unittest
from fixtures import layout, triangulate
from test_native_capture import native
from shared_capture_contract import same
from shared_capture_native import read_mesh, restore_triangulated_editor_attributes, begin_source_capture, finish_source_capture
from shared_capture_normals import set_captured_normals

BUILTINS = {".select_vert": ("POINT", "vertices"), ".select_edge": ("EDGE", "edges"),
            ".select_poly": ("FACE", "faces"), "sharp_face": ("FACE", "faces")}


def fixture(rows=3, cols=4):
    source, _, _ = layout(rows, cols, .003)
    for name, (domain, collection) in BUILTINS.items():
        source["attributes"][name] = {"domain": domain, "type": "BOOLEAN",
                                      "values": [False] * len(source[collection])}
    runtime, ids, _, _ = triangulate(source)
    for name in BUILTINS:
        del runtime["attributes"][name]
    return source, runtime, ids


def face_selection_fixture():
    source, _, _ = fixture()
    for i, face in enumerate(source["faces"]):
        face["select"] = i % 2 == 0
    source["attributes"][".select_poly"]["values"] = [face["select"] for face in source["faces"]]
    runtime, ids, shapes, normals = triangulate(source)
    for name in BUILTINS:
        del runtime["attributes"][name]
    for face in runtime["faces"]:
        face["select"] = False
    obj = native(runtime, ids, shapes)
    bind_selection_storage(obj)
    return source, runtime, ids, obj, normals


def bind_selection_storage(obj):
    create = obj.data.attributes.new
    collections = {".select_vert": obj.data.vertices, ".select_edge": obj.data.edges,
                   ".select_poly": obj.data.polygons}
    class SelectionValue:
        def __init__(self, element):
            self.element = element
        @property
        def value(self):
            return self.element.select
        @value.setter
        def value(self, value):
            self.element.select = value
    def bind(attr):
        if attr.name in collections:
            attr.data = [SelectionValue(element) for element in collections[attr.name]]
    for attr in obj.data.attributes:
        bind(attr)
    def new_attribute(*args, **kwargs):
        attr = create(*args, **kwargs)
        bind(attr)
        return attr
    obj.data.attributes.new = new_attribute


class DefaultRestorationTests(unittest.TestCase):
    def test_nondefault_point_or_edge_storage_omission_is_not_admitted(self):
        for name, collection in ((".select_vert", "vertices"), (".select_edge", "edges")):
            source, runtime, ids = fixture()
            source["attributes"][name]["values"][0] = True
            source[collection][0]["select"] = True
            with self.assertRaisesRegex(ValueError, "unexpectedly omitted"):
                restore_triangulated_editor_attributes(native(runtime, ids), source, ids)

    def test_bmesh_selection_propagation_restores_all_original_domains(self):
        source, _, _ = fixture()
        for name, (domain, collection) in BUILTINS.items():
            if name == "sharp_face":
                continue
            values = [i % 3 == 0 for i in range(len(source[collection]))]
            source["attributes"][name]["values"] = values
            for row, value in zip(source[collection], values):
                row["select"] = value
        runtime, ids, shapes, normals = triangulate(source)
        for name, collection in ((".select_vert", "vertices"), (".select_edge", "edges")):
            runtime["attributes"][name]["values"] = [True] * len(runtime[collection])
            for row in runtime[collection]:
                row["select"] = True
        del runtime["attributes"][".select_poly"]
        for face in runtime["faces"]:
            face["select"] = False
        obj = native(runtime, ids, shapes)
        bind_selection_storage(obj)
        _, owner, recipe = layout(3, 4, .003)
        capture = begin_source_capture(native(source), owner, recipe)
        set_captured_normals(obj, capture, normals)
        packet = finish_source_capture(obj, capture)
        receipt = packet["native_normal_generation"]["transport_attribute_restoration"]
        maps = receipt["editor_selection_source_ids"]
        for domain, collection in (("POINT", "vertices"), ("EDGE", "edges"), ("FACE", "faces")):
            expected = [source[collection][i]["select"] if i is not None else True for i in maps[domain]]
            self.assertEqual([row["select"] for row in packet["runtime"][collection]], expected)
        self.assertIn(None, maps["EDGE"])
        self.assertTrue(same(packet["source"]["mesh"], source))
        self.assertTrue(same(receipt["before"], runtime))

    def test_unbound_generated_selection_storage_is_not_silently_removed(self):
        source, runtime, ids = fixture()
        del source["attributes"][".select_vert"]
        runtime["attributes"][".select_vert"] = {"domain": "POINT", "type": "BOOLEAN",
                                                  "values": [False] * len(runtime["vertices"])}
        with self.assertRaisesRegex(ValueError, "Unbound generated"):
            restore_triangulated_editor_attributes(native(runtime, ids), source, ids)

    def test_nondefault_face_selection_restored_by_verified_source_ids(self):
        source, runtime, ids, obj, normals = face_selection_fixture()
        _, owner, recipe = layout(3, 4, .003)
        capture = begin_source_capture(native(source), owner, recipe)
        set_captured_normals(obj, capture, normals)
        packet = finish_source_capture(obj, capture)
        expected = [source["faces"][i]["select"] for i in ids["FACE"]]
        self.assertEqual([face["select"] for face in packet["runtime"]["faces"]], expected)
        self.assertTrue(same(packet["source"]["mesh"], source))
        receipt = packet["native_normal_generation"]["transport_attribute_restoration"]
        self.assertEqual(receipt["editor_selection_source_ids"]["FACE"], ids["FACE"])
        self.assertTrue(same(receipt["before"], runtime))

    def test_bad_face_identity_or_source_field_disagreement_fails_before_mutation(self):
        for case in ("identity", "source"):
            source, runtime, ids, obj, _ = face_selection_fixture()
            if case == "identity":
                ids["FACE"][0] = len(source["faces"])
            else:
                source["attributes"][".select_poly"]["values"][0] = False
            before = read_mesh(obj)
            with self.assertRaises(ValueError):
                restore_triangulated_editor_attributes(obj, source, ids)
            self.assertTrue(same(before, read_mesh(obj)))

    def test_full_capture_retains_native_restoration_receipt(self):
        source, runtime, ids = fixture()
        _, owner, recipe = layout(3, 4, .003)
        capture = begin_source_capture(native(source), owner, recipe)
        _, _, shapes, normals = triangulate(source)
        obj = native(runtime, ids, shapes)
        set_captured_normals(obj, capture, normals)
        packet = finish_source_capture(obj, capture)
        receipt = packet["native_normal_generation"]["transport_attribute_restoration"]
        self.assertTrue(same(receipt["before"], runtime))
        self.assertTrue(same(packet["source"]["mesh"], source))
        self.assertEqual(set(receipt["restored_attributes"]), set(BUILTINS))

    def test_two_layouts_restore_real_fields_with_immutable_source(self):
        for rows, cols in ((3, 4), (5, 7)):
            source, runtime, ids = fixture(rows, cols)
            original = deepcopy(source)
            obj = native(runtime, ids)
            after, receipt = restore_triangulated_editor_attributes(obj, source, ids)
            self.assertTrue(same(source, original))
            self.assertTrue(same(after, read_mesh(obj)))
            self.assertEqual(set(receipt["restored_attributes"]), set(BUILTINS))
            self.assertTrue(same(receipt["before"], runtime))
            for name in BUILTINS:
                self.assertIsNotNone(obj.data.attributes.get(name))

    def test_inconsistent_source_or_nondefault_sharp_omission_fails_before_mutation(self):
        for name in BUILTINS:
            source, runtime, ids = fixture()
            source["attributes"][name]["values"][0] = True
            obj = native(runtime, ids)
            before = read_mesh(obj)
            with self.assertRaisesRegex(ValueError, "Nondefault|disagreement"):
                restore_triangulated_editor_attributes(obj, source, ids)
            self.assertTrue(same(before, read_mesh(obj)))

    def test_missing_mistyped_or_unknown_field_fails(self):
        for mutation in ("domain", "type", "unknown"):
            source, runtime, ids = fixture()
            if mutation == "domain":
                source["attributes"][".select_vert"]["domain"] = "FACE"
            elif mutation == "type":
                source["attributes"][".select_vert"]["type"] = "INT"
            else:
                del runtime["attributes"]["SourceAuthorSharp"]
            with self.assertRaisesRegex(ValueError, "omission|disagreement"):
                restore_triangulated_editor_attributes(native(runtime, ids), source, ids)

    def test_public_rna_nondefault_and_other_mutations_fail(self):
        for change in ("public", "attribute", "extra"):
            source, runtime, ids = fixture()
            if change == "public":
                runtime["vertices"][0]["select"] = True
            elif change == "attribute":
                runtime["attributes"]["NormalRegion"]["values"][0] = 8
            else:
                runtime["attributes"]["unrelated"] = deepcopy(source["attributes"][".select_vert"])
            obj = native(runtime, ids)
            before = read_mesh(obj)
            with self.assertRaises(ValueError):
                restore_triangulated_editor_attributes(obj, source, ids)
            self.assertTrue(same(before, read_mesh(obj)))

    def test_no_omission_is_read_only(self):
        source, _, ids = fixture()
        runtime, ids, _, _ = triangulate(source)
        obj = native(runtime, ids)
        before = read_mesh(obj)
        after, receipt = restore_triangulated_editor_attributes(obj, source, ids)
        self.assertIsNone(receipt)
        self.assertTrue(same(before, after))

    def test_native_creation_side_effect_is_not_accepted(self):
        source, runtime, ids = fixture()
        obj = native(runtime, ids)
        create = obj.data.attributes.new
        def corrupt(*args, **kwargs):
            result = create(*args, **kwargs)
            obj.data.vertices[0].hide = True
            return result
        obj.data.attributes.new = corrupt
        with self.assertRaisesRegex(ValueError, "restoration changed other"):
            restore_triangulated_editor_attributes(obj, source, ids)


if __name__ == "__main__":
    unittest.main()

"""CPU capture contract regressions, not native/pose/visual acceptance."""

from copy import deepcopy
import json
import unittest
from fixtures import captured, layout, triangulate
from shared_capture_contract import complete_capture, freeze_source, source_payload


class CaptureTests(unittest.TestCase):
    def test_two_layouts_capture_real_arrays_and_reindex(self):
        for rows, cols, bend in ((3, 4, .002), (5, 7, .006)):
            with self.subTest(layout=(rows, cols)):
                frozen, args = captured(rows, cols, bend)
                packet = complete_capture(frozen, *args)
                self.assertEqual(len(packet["fixed_triangles"]), 2 * (rows - 1) * (cols - 1))
                self.assertEqual(packet["saved_keys_by_source_point"]["Basis"],
                                 [p["co"] for p in source_payload(frozen)["mesh"]["vertices"]])
                runtime, ids, shapes, _ = args
                for key, values in packet["saved_keys_by_source_point"].items():
                    self.assertEqual(values, [shapes[key][ids["POINT"].index(i)] for i in range(rows * cols)])
                self.assertEqual(packet["source"]["mesh"]["attributes"]["MouthOpenDelta"],
                                 source_payload(frozen)["mesh"]["attributes"]["MouthOpenDelta"])
                self.assertFalse(packet["native_ready"])
                self.assertEqual(packet["raw_author_binding"], "pending independent reference roles")
                self.assertNotIn("protected", packet)  # CreaseGroup 2 is not an XYZ pin.

    def test_frozen_source_cannot_be_rebased(self):
        frozen, args = captured()
        original = source_payload(frozen)
        original["mesh"]["vertices"][0]["co"][0] += .1
        frozen["source_json"] = json.dumps(original)
        with self.assertRaisesRegex(ValueError, "immutable"):
            complete_capture(frozen, *args)

    def test_missing_morph_and_changed_basis_fail(self):
        for mutation in (lambda shapes: shapes.pop("blink.L"),
                         lambda shapes: shapes["Basis"][0].__setitem__(0, .3)):
            frozen, args = captured()
            mutation(args[2])
            with self.assertRaises(ValueError):
                complete_capture(frozen, *args)

    def test_category_corner_uv_and_point_changes_fail(self):
        changes = [
            lambda m: m["attributes"]["NormalRegion"]["values"].__setitem__(0, 9),
            lambda m: m["uv_layers"]["UV2"][0]["uv"].__setitem__(0, 9),
            lambda m: m["vertices"][0]["co"].__setitem__(0, 9),
            lambda m: m["vertices"][0]["groups"].clear(),
            lambda m: m["attributes"]["MouthOpenDelta"]["values"][0].__setitem__(0, 9),
        ]
        for change in changes:
            frozen, args = captured()
            change(args[0])
            with self.assertRaises(ValueError):
                complete_capture(frozen, *args)

    def test_topology_identity_and_corner_owner_changes_fail(self):
        changes = [lambda a: a[1]["POINT"].__setitem__(0, a[1]["POINT"][1]),
                   lambda a: a[1]["FACE"].__setitem__(0, 999),
                   lambda a: a[1]["CORNER"].__setitem__(0, a[1]["CORNER"][-1]),
                   lambda a: a[0]["faces"][0]["vertices"].reverse(),
                   lambda a: a[0]["faces"].pop()]
        for change in changes:
            frozen, args = captured()
            change(args)
            with self.assertRaises(ValueError):
                complete_capture(frozen, *args)

    def test_unsupported_or_ambiguous_source_roles_fail(self):
        for change in (lambda o: o.update(validated=False),
                       lambda o: o["other_members"].append({"object_name": "Ear", "regions": [7]}),
                       lambda o: o.update(skin_regions=[8])):
            source, owners, recipe = layout(3, 4, .002)
            change(owners)
            with self.assertRaises(ValueError):
                freeze_source(source, owners, recipe)

    def test_normal_vectors_reject_nonnumeric_boolean_and_nonfinite(self):
        for value in ([True, 0., 1.], ["a", "b", "c"], [0., float("inf"), 1.]):
            frozen, args = captured()
            args[3][0] = value
            with self.assertRaises(ValueError):
                complete_capture(frozen, *args)
            source, owners, recipe = layout(3, 4, .002)
            source["native_corner_normals"][0] = value
            with self.assertRaises(ValueError):
                freeze_source(source, owners, recipe)

    def test_signed_zero_source_field_change_is_not_exact(self):
        frozen, args = captured()
        args[0]["attributes"]["MouthOpenDelta"]["values"][0][0] = -0.0
        with self.assertRaisesRegex(ValueError, "Changed source attribute"):
            complete_capture(frozen, *args)

    def test_nonfinite_actual_saved_values_fail(self):
        frozen, args = captured()
        args[2]["MouthPucker"][0][0] = float("nan")
        with self.assertRaises(ValueError):
            complete_capture(frozen, *args)


if __name__ == "__main__":
    unittest.main()

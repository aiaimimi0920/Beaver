import copy
import json
import unittest
from compose import HERE
from inventory import family_inventory


class FamilyInventoryTests(unittest.TestCase):
    def setUp(self):
        self.catalog = json.loads((HERE / "catalog.json").read_text())
        self.registry = json.loads((HERE / "families.json").read_text())

    def test_variants_are_not_families(self):
        result = family_inventory()
        self.assertEqual(result["mouth"]["selectable_options"], 6)
        self.assertEqual(result["mouth"]["implemented_families"], 2)
        self.assertEqual(result["mouth"]["pending_native_families"], 0)
        self.assertEqual(result["brows"]["selectable_options"], 4)
        self.assertEqual(result["face"]["implemented_families"], 2)

    def test_duplicate_or_missing_preset_rejected(self):
        r = copy.deepcopy(self.registry)
        r["slots"]["brows"]["angular-peaked-wedge"]["presets"].append("soft-level")
        with self.assertRaises(ValueError):
            family_inventory(self.catalog, r)
        r = copy.deepcopy(self.registry)
        r["slots"]["ears"]["rounded-human"]["presets"].pop()
        with self.assertRaises(ValueError):
            family_inventory(self.catalog, r)

    def test_planned_family_not_executable(self):
        r = copy.deepcopy(self.registry)
        r["slots"]["ears"]["lateral-elf"]["status"] = "planned"
        with self.assertRaises(ValueError):
            family_inventory(self.catalog, r)

    def test_unknown_status_or_no_sources_rejected(self):
        for key, value in [("status", "passed"), ("sources", [])]:
            r = copy.deepcopy(self.registry)
            r["slots"]["mouth"]["minimal-seam"][key] = value
            with self.assertRaises(ValueError):
                family_inventory(self.catalog, r)

    def test_nose_variant_does_not_inflate_family_count(self):
        result = family_inventory()["nose"]
        self.assertEqual(result["selectable_options"], 4)
        self.assertEqual(result["implemented_families"], 2)
        self.assertEqual(result["planned_families"], 1)

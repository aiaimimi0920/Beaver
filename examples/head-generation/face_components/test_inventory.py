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
        self.assertEqual(result["mouth"]["selectable_options"], 3)
        self.assertEqual(result["mouth"]["implemented_families"], 1)
        self.assertEqual(result["brows"]["selectable_options"], 4)
        self.assertEqual(result["face"]["implemented_families"], 1)

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
        r["slots"]["ears"]["lateral-elf"]["presets"] = ["compact-human"]
        with self.assertRaises(ValueError):
            family_inventory(self.catalog, r)

    def test_unknown_status_or_no_sources_rejected(self):
        for key, value in [("status", "passed"), ("sources", [])]:
            r = copy.deepcopy(self.registry)
            r["slots"]["mouth"]["minimal-seam"][key] = value
            with self.assertRaises(ValueError):
                family_inventory(self.catalog, r)

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
        self.assertEqual(result["brows"]["selectable_options"], 15)
        self.assertEqual(result["brows"]["implemented_families"], 12)
        self.assertEqual(result["brows"]["pending_native_families"], 0)
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

    def test_researched_brow_evidence_is_an_exact_preset_partition(self):
        root = HERE / "../brow_library"
        presets = json.loads((root / "presets.json").read_text())
        evidence = json.loads((root / "native-evidence.json").read_text())
        covered = [
            sample["selection"]["brows"].removeprefix("research-")
            for sample in evidence["samples"]
        ]
        pending = evidence["pending_presets"]
        self.assertEqual(len(covered), len(set(covered)))
        self.assertEqual(len(pending), len(set(pending)))
        self.assertFalse(set(covered) & set(pending))
        self.assertEqual(set(covered) | set(pending), set(presets))
        for sample in evidence["samples"]:
            self.assertEqual(sample["status"], "technically_verified")
            self.assertEqual(sample["captures"]["eye"]["captures_verified"], 84)
            self.assertEqual(sample["captures"]["mouth"]["captures_verified"], 18)

    def test_researched_family_samples_bind_to_their_own_presets(self):
        evidence = json.loads(
            (HERE / "../brow_library/native-evidence.json").read_text()
        )
        samples = {s["candidate"]: s for s in evidence["samples"]}
        for name, family in self.registry["slots"]["brows"].items():
            if not name.startswith("research-"):
                continue
            self.assertEqual(
                family["status"], "implemented_with_historical_samples"
            )
            referenced = [samples[key] for key in family["native_samples"]]
            self.assertEqual(
                {s["selection"]["brows"] for s in referenced},
                set(family["presets"]),
            )
            self.assertTrue(
                all(s["status"] == "technically_verified" for s in referenced)
            )

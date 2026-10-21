"""Historical mouth evidence must cover only its explicitly verified candidates."""

import json
from pathlib import Path
import unittest

HERE = Path(__file__).resolve().parent


class MouthRegistryTests(unittest.TestCase):
    def test_samples_are_unique_complete_and_not_pending(self):
        evidence = json.loads(
            (HERE / "../mouth_library/native-evidence.json").read_text()
        )
        catalog = json.loads((HERE / "../mouth_library/presets.json").read_text())
        samples = evidence["samples"]
        self.assertEqual(len({s["candidate"] for s in samples}), len(samples))
        self.assertEqual(len({s["selection"]["mouth"] for s in samples}), len(samples))
        for sample in samples + evidence["cross_combinations"]:
            self.assertEqual(sample["status"], "technically_verified")
            self.assertIn(
                sample["selection"]["mouth"].removeprefix("research-"), catalog
            )
            self.assertNotIn(
                int(sample["candidate"].removeprefix("combo")),
                evidence["pending_cases"],
            )
            for key, count in [("eye", 84), ("mouth", 18), ("mouth_motion", 92)]:
                self.assertEqual(sample["captures"][key]["captures_verified"], count)
                self.assertEqual(
                    sample["captures"][key]["source_sha256"], sample["glb_sha256"]
                )
            self.assertEqual(len(sample["composition_sha256"]), 64)

    def test_families_bind_verified_presets(self):
        evidence = json.loads(
            (HERE / "../mouth_library/native-evidence.json").read_text()
        )
        families = json.loads((HERE / "families.json").read_text())["slots"]["mouth"]
        samples = {row["candidate"]: row for row in evidence["samples"]}
        for name, family in families.items():
            if not name.startswith("research-"):
                continue
            self.assertEqual(family["status"], "implemented_with_historical_samples")
            self.assertEqual(
                {
                    samples[key]["selection"]["mouth"]
                    for key in family["native_samples"]
                },
                set(family["presets"]),
            )

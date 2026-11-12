"""Exact retained implementation binding, including between-read changes."""
import hashlib
import json
from unittest.mock import patch
import unittest
import construction_adapter as adapter
from construction_topology import digest


def binding(sources):
    inventory = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in sources.items()}
    return {"capture_inventory": inventory, "recipe": {"capture_inventory_sha256": digest(inventory)}}


class SourceBindingTests(unittest.TestCase):
    def test_changed_helper_between_adapt_and_retention_is_rejected_atomically(self):
        sources = adapter.retained_sources()
        record = binding(sources)
        sources["construction_journal.py"] += "\n# altered after initial binding\n"
        retained = {"semantic_bundle.py": "original"}
        with patch.object(adapter, "retained_sources", return_value=sources):
            with self.assertRaisesRegex(ValueError, "changed during"):
                adapter.retain_sources(retained, adapter.POLICY, {"construction_binding.json": json.dumps(record)})
        self.assertEqual(retained, {"semantic_bundle.py": "original"})

    def test_bootstrap_rejects_hash_mismatch_and_duplicate_bindings(self):
        sources = adapter.retained_sources()
        record = binding(sources)
        for kind in ("changed_helper", "duplicate_binding", "wrong_digest"):
            current = dict(sources)
            row = json.loads(json.dumps(record))
            if kind == "changed_helper":
                current["construction_native.py"] += "\nraise RuntimeError('must not run')\n"
            if kind == "wrong_digest":
                row["recipe"]["capture_inventory_sha256"] = "0" * 64
            text = "RETAINED_CONSTRUCTION_SOURCES = " + repr(current) + "\nRETAINED_CONSTRUCTION_BINDING = " + repr(row)
            if kind == "duplicate_binding":
                text += "\nRETAINED_CONSTRUCTION_BINDING = " + repr(row)
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                exec(adapter.bootstrap("retained"), {"retained": text})


if __name__ == "__main__":
    unittest.main()

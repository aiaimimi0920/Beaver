"""Seeded bootstrap retains all source bodies and complete hash verification."""
import ast
import hashlib
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import construction_adapter as adapter
from construction_topology import digest


def bound(sources):
    inventory = {name: hashlib.sha256(source.encode()).hexdigest() for name, source in sources.items()}
    return {'capture_inventory': inventory, 'recipe': {'capture_inventory_sha256': digest(inventory)}}


class RuntimeRetentionTests(unittest.TestCase):
    def setUp(self):
        self.sources = adapter.retained_sources()
        self.record = bound(self.sources)
        retained = {'semantic_bundle.py': ''}
        adapter.retain_sources(retained, adapter.POLICY, {'construction_binding.json': json.dumps(self.record)})
        self.text = retained['semantic_bundle.py']

    def test_seeded_bootstrap_preserves_all_sources_and_binding(self):
        nodes = {n.targets[0].id: ast.literal_eval(n.value) for n in ast.parse(self.text).body}
        self.assertEqual(nodes['RETAINED_CONSTRUCTION_SOURCES'], self.sources)
        self.assertEqual(nodes['RETAINED_CONSTRUCTION_BINDING'], self.record)
        self.assertEqual(nodes['RETAINED_CONSTRUCTION_BOOTSTRAP'], adapter.bootstrap('_construction_bootstrap_retained'))
        exec(adapter.seeded_bootstrap('retained'), {'retained': self.text})
        self.assertLess(len(adapter.seeded_bootstrap('retained')), len(adapter.bootstrap('retained')))

    def test_seeded_replay_rejects_missing_duplicate_and_tampered_bootstrap(self):
        rows = ast.parse(self.text).body
        good = {n.targets[0].id: ast.literal_eval(n.value) for n in rows}
        for kind in ('missing', 'duplicate', 'tampered', 'changed_runtime', 'changed_helper'):
            current = dict(good)
            if kind == 'missing':
                current.pop('RETAINED_CONSTRUCTION_BOOTSTRAP')
            elif kind == 'tampered':
                current['RETAINED_CONSTRUCTION_BOOTSTRAP'] += '\nraise RuntimeError("must not run")'
            elif kind in ('changed_runtime', 'changed_helper'):
                sources = dict(current['RETAINED_CONSTRUCTION_SOURCES'])
                name = 'ear_join.py' if kind == 'changed_runtime' else 'ear_adapter.py'
                sources[name] += '\nraise RuntimeError("must not run")'
                current['RETAINED_CONSTRUCTION_SOURCES'] = sources
            text = '\n'.join(key + ' = ' + repr(value) for key, value in current.items())
            if kind == 'duplicate':
                text += '\nRETAINED_CONSTRUCTION_BOOTSTRAP = ' + repr(good['RETAINED_CONSTRUCTION_BOOTSTRAP'])
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                exec(adapter.seeded_bootstrap('retained'), {'retained': text})

    def test_helper_change_rejected_before_retention(self):
        changed = dict(self.sources)
        changed['ear_adapter.py'] += '\n# changed after adaptation\n'
        retained = {'semantic_bundle.py': 'original'}
        with patch.object(adapter, 'retained_sources', return_value=changed):
            with self.assertRaisesRegex(ValueError, 'changed during'):
                adapter.retain_sources(retained, adapter.POLICY, {'construction_binding.json': json.dumps(self.record)})
        self.assertEqual(retained, {'semantic_bundle.py': 'original'})


if __name__ == '__main__':
    unittest.main()

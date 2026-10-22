"""Late review regressions: requested identity, planner parity, and portable sources."""

import copy
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

from compose import HERE, resolve
from plan import build_plan
from verify_run import verify_requested_composition
from inventory import family_inventory
import face_selection
import ear_selection
from face_evidence import validate_face_report
from ear_evidence import validate_ear_report


class ReviewRegressions(unittest.TestCase):
    def setUp(self):
        self.selection = json.loads((HERE / 'example-selection.json').read_text())
        self.brow = {**self.selection, 'brows': 'research-linear-fine'}
        self.controls = {'width_scale': 0.8}

    def test_brow_requested_identity_is_required_and_exact(self):
        snapshot = resolve(self.brow, brow_overrides=self.controls)
        spec = {'selection': self.brow, 'fit_policy': 'legacy'}
        with self.assertRaisesRegex(ValueError, 'requires requested composition'):
            verify_requested_composition(snapshot, spec)
        spec['composition_sha256'] = resolve(self.brow)['composition_sha256']
        with self.assertRaisesRegex(ValueError, 'identity mismatch'):
            verify_requested_composition(snapshot, spec)
        spec['composition_sha256'] = snapshot['composition_sha256']
        verify_requested_composition(snapshot, spec)

    def test_legacy_identity_remains_optional_but_wrong_hash_rejected(self):
        snapshot = resolve(self.selection)
        spec = {'selection': self.selection, 'fit_policy': 'legacy'}
        verify_requested_composition(snapshot, spec)
        with self.assertRaises(ValueError):
            verify_requested_composition(snapshot, {**spec, 'composition_sha256': 'bad'})

    def test_planner_forwards_controls_and_matches_evidence_exactly(self):
        snapshot = resolve(self.brow, brow_overrides=self.controls)
        record = {**snapshot, 'candidate': 'cpu-fixture'}
        plan = build_plan(self.brow, records=[record], brow_overrides=self.controls)
        self.assertEqual(plan['snapshot'], snapshot)
        self.assertEqual(plan['recorded_evidence'][0]['match'], 'exact_snapshot')
        default = build_plan(self.brow, records=[record])
        self.assertEqual(default['snapshot'], resolve(self.brow))
        self.assertEqual(default['recorded_evidence'][0]['match'], 'historical_selection_only')
        with self.assertRaises(ValueError):
            build_plan(self.brow, records=[], brow_overrides={'width_scale': -1})
        # Append the new parameter so existing positional override callers keep meaning.
        self.assertEqual(build_plan(self.brow, 'legacy', [], None, None, None, None),
                         build_plan(self.brow, records=[]))

    def test_planner_cli_controls_match_api_and_reject_invalid_bounds(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            selection, controls = root / 'selection.json', root / 'controls.json'
            selection.write_text(json.dumps(self.brow))
            controls.write_text(json.dumps(self.controls))
            command = [sys.executable, str(HERE / 'plan.py'), '--selection', str(selection),
                       '--brow-controls', str(controls)]
            result = subprocess.run(command, check=True, capture_output=True, text=True)
            self.assertEqual(json.loads(result.stdout)['snapshot'],
                             resolve(self.brow, brow_overrides=self.controls))
            controls.write_text(json.dumps({'width_scale': -1}))
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)

    def test_portable_provenance_and_frozen_validation_reject_real_tampering(self):
        base = json.loads((HERE.parent / 'checkpoints/current/face_style.json').read_text())
        cases = [('face', 'face', 'research-compact-rounded', face_selection,
                  validate_face_report),
                 ('ear', 'ears', 'research-standard-oval', ear_selection,
                  validate_ear_report)]
        for kind, slot, preset, selection_module, validate in cases:
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as directory:
                library = Path(directory) / (kind + '_library')
                shutil.copytree(selection_module.LIBRARY, library)
                expected = selection_module.module().provenance()
                for name in expected:
                    source = (library / name).read_text(encoding='utf-8')
                    (library / name).write_bytes(source.replace('\n', '\r\n').encode('utf-8'))
                with patch.object(selection_module, 'LIBRARY', library):
                    module = selection_module.module()
                    self.assertEqual(module.provenance(), expected)
                    snapshot = resolve({**self.selection, slot: preset})
                    style, record = module.apply_design(base, snapshot[kind + '_design'])
                    report = {kind + '_design': record,
                              'facial_surface': {kind + '_design': copy.deepcopy(record)}}
                    frozen = {name: (library / name).read_text(encoding='utf-8')
                              for name in expected}
                    for name, source in frozen.items():
                        self.assertEqual(hashlib.sha256(source.encode('utf-8')).hexdigest(),
                                         expected[name])
                    self.assertIsNotNone(validate(snapshot, report, style, frozen))
                    for name in expected:
                        changed = {**frozen, name: frozen[name] + '\n# real change\n'}
                        with self.assertRaisesRegex(ValueError, 'Frozen'):
                            validate(snapshot, report, style, changed)
                    changed_path = library / next(iter(expected))
                    changed_path.write_bytes(changed_path.read_bytes() + b'\r\n# real change\r\n')
                    self.assertNotEqual(selection_module.module().provenance(), expected)

    def test_historical_family_status_never_accepts_fresh_output(self):
        inventory = family_inventory()
        for slot in ('face', 'ears'):
            self.assertEqual(inventory[slot]['pending_native_families'], 0)
            self.assertEqual(inventory[slot]['implemented_families'], 3)
        plan = build_plan({**self.selection, 'face': 'research-compact-rounded',
                           'ears': 'research-standard-oval'}, records=[])
        self.assertEqual(plan['current_output_status'], 'not_generated')
        self.assertTrue(all(step['status'] == 'pending' for step in plan['steps']))


if __name__ == '__main__':
    unittest.main()

"""Ordered authoring modules retain shared global state and exact source identity."""
import copy
from pathlib import Path
import sys
import tempfile
import types
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.layout import digest, reconstructed
from source_layout.module_layout import MODULE_STAGES, module_layout, execute_ordered_module
from source_layout.evidence import HELPERS, validate_layout_set

SOURCE = '''"""Original face documentation."""
trace = ['surface']
scale = 2
def surface(value):
    return value * scale
def mesh(value):
    return density_rows(surface(value))
trace.append('parts')
def density_rows(value):
    return value + offset
def orbital_endpoint_weight(value):
    return mesh(value)
offset = 3
trace.append(orbital_endpoint_weight(4))
def ear_shell_boundary_edges(value):
    return orbital_endpoint_weight(value)
'''


class OrderedModuleTests(unittest.TestCase):
    def test_reconstruction_and_late_global_binding(self):
        files, record = module_layout(SOURCE, 'face.py')
        self.assertEqual(reconstructed(files, record), SOURCE)
        original = {'__name__': 'face'}
        exec(SOURCE, original)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, text in files.items():
                (root / name).write_text(text)
            actual = {'__file__': str(root / 'face.py'), '__name__': 'face'}
            exec(files['face.py'], actual)
            self.assertEqual(actual['trace'], original['trace'])
            self.assertEqual(actual['__doc__'], original['__doc__'])
            self.assertIs(actual['mesh'].__globals__, actual)
            actual['scale'] = original['scale'] = 5
            self.assertEqual(actual['mesh'](4), original['mesh'](4))
            self.assertFalse(any(name.startswith('_source_layout_') for name in actual))

    def test_explicit_moduletype_replay_contract(self):
        files, record = module_layout(SOURCE, 'face.py')
        frame = types.ModuleType('face')
        with self.assertRaisesRegex(ValueError, 'filesystem-aware'):
            exec(files['face.py'], frame.__dict__)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, text in files.items():
                (root / name).write_text(text, encoding='utf-8')
            execute_ordered_module(root / 'face.py', record, frame.__dict__)
            self.assertEqual(frame.mesh(4), 11)
            self.assertIs(frame.mesh.__globals__, frame.__dict__)
            (root / record['stages'][0]['path']).write_text('changed = True')
            with self.assertRaises(ValueError):
                execute_ordered_module(root / 'face.py', record, {})

    def test_replay_executes_verified_snapshot_not_changed_files(self):
        from source_layout import layout as layout_module
        files, record = module_layout(SOURCE, 'face.py')
        original_reconstruct = layout_module.reconstructed
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, text in files.items():
                (root / name).write_text(text, encoding='utf-8')
            def verify_then_change(snapshot, evidence):
                result = original_reconstruct(snapshot, evidence)
                (root / record['stages'][0]['path']).write_text('raise RuntimeError("changed")')
                return result
            frame = types.ModuleType('face')
            with mock.patch.object(layout_module, 'reconstructed', side_effect=verify_then_change):
                execute_ordered_module(root / 'face.py', record, frame.__dict__)
            self.assertEqual(frame.mesh(4), 11)

    def test_tampered_entry_and_stage_rejected(self):
        files, record = module_layout(SOURCE, 'face.py')
        for name in files:
            changed = dict(files)
            changed[name] += '\nchanged = True\n'
            with self.subTest(name=name), self.assertRaises(ValueError):
                reconstructed(changed, record)

    def test_unknown_order_and_reserved_state_rejected(self):
        for source in (
            SOURCE.replace('def mesh(', 'def unknown('),
            SOURCE.replace('def mesh(', 'def density_rows_tmp(').replace('def density_rows(', 'def mesh(').replace('def density_rows_tmp(', 'def density_rows('),
            '_source_layout_path = 1\n' + SOURCE,
            'from __future__ import annotations\n' + SOURCE,
        ):
            with self.assertRaises(ValueError):
                module_layout(source, 'face.py')

    def test_oversized_responsibility_rejected(self):
        with self.assertRaisesRegex(ValueError, 'exceeds'):
            module_layout('x = 1\n' * 501 + SOURCE, 'face.py')

    def layout_fixture(self):
        records = [{'path': 'pipeline.py'}, {'path': 'semantic_parameters.py'}]
        hashes = {'x/' + name: 'baseline' for name in HELPERS}
        for entry, stages in MODULE_STAGES.items():
            source = 'value = 1\n' + '\n'.join('def ' + function + '():\n    return value\n' for _, function in stages if function)
            files, record = module_layout(source, entry)
            records.append({'path': entry, **record})
            hashes.update({'x/' + name: digest(text) for name, text in files.items() if name != entry})
        return {'records': records, 'helper_sha256': hashes}

    def test_complete_layout_family_and_hashes(self):
        layout = self.layout_fixture()
        validate_layout_set(layout, 'x/')
        for mutate in ('missing', 'hash', 'path'):
            changed = copy.deepcopy(layout)
            if mutate == 'missing':
                changed['records'].pop()
            elif mutate == 'hash':
                changed['records'][-1]['stages'][0]['sha256'] = 'wrong'
            else:
                changed['records'][-1]['stages'][0]['path'] = '../escape.py'
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                validate_layout_set(changed, 'x/')


if __name__ == '__main__':
    unittest.main()

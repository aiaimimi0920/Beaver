"""Hash-bound source seed transport is opt-in and rejects changed source bytes."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.source_seed import seed_retained_bundle, readable_source_literal


class SourceSeedTests(unittest.TestCase):
    def fixture(self):
        retained = 'RETAINED_UPPER_SHAPE_SOURCES = {}\nRETAINED_BROW_SOURCES = {}\n'
        code = ('"""菲的源码传输测试。"""\nfrom __future__ import annotations\n'
                'from pathlib import Path\nimport json\nreport = {}\n'
                + 'Path(beaver_output("asset/semantic_bundle.py")).write_text(' + repr(retained) + ')\n')
        return retained, code

    def test_source_bytes_and_future_semantics_preserved(self):
        retained, code = self.fixture()
        generation, seed, binding = seed_retained_bundle(code, 'asset/')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'asset').mkdir()
            scope = {'beaver_output': lambda p: str(root / p), 'beaver_input': lambda p: str(root / p)}
            exec(seed['code'], scope)
            exec(generation, scope)
            self.assertEqual((root / 'asset/semantic_bundle.py').read_text(), retained)
            self.assertEqual(scope['report']['authored_source_seed'], binding)
            (root / binding['path']).write_text(retained + '# changed\n')
            with self.assertRaisesRegex(ValueError, 'seed changed'):
                exec(generation, scope)

    def test_split_seed_preserves_ast_and_checks_runtime_hash(self):
        retained, code = self.fixture()
        runtime = 'label = ' + repr('curve '*6000) + '\nresult = len(label)\n'
        code += f'exec(compile({runtime!r}, "runtime", "exec"), globals())\n'
        from source_layout.transport_sources import validate_native_frame
        calls = []
        def force_first_split(request):
            calls.append(request)
            if len(calls) == 1:
                raise ValueError('Synthetic oversized combined seed')
            validate_native_frame(request)
        with patch('source_layout.source_seed.validate_native_frame', force_first_split):
            generation, seed, binding = seed_retained_bundle(code, 'asset/')
        self.assertEqual(binding['schema_version'], 2)
        self.assertEqual(len(seed['outputs']), 2)
        validate_native_frame(seed)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'asset').mkdir()
            scope = {'beaver_output': lambda p:str(root/p), 'beaver_input':lambda p:str(root/p)}
            exec(seed['code'], scope)
            self.assertEqual((root/binding['path']).read_text(), retained)
            self.assertEqual((root/binding['runtime_path']).read_text(), runtime)
            exec(generation, scope)
            self.assertEqual(scope['result'], len('curve '*6000))
            (root/binding['runtime_path']).write_text(runtime+'# changed\n')
            with self.assertRaisesRegex(ValueError, 'Runtime source seed changed'):
                exec(generation, scope)

    def test_readable_fallback_seed_writes_exact_modules(self):
        retained, code = self.fixture()
        runtime = 'label = ' + repr('curve ' * 6000) + '\nresult = len(label)\n'
        code += f'exec(compile({runtime!r}, "runtime", "exec"), globals())\n'
        from source_layout.transport_sources import validate_native_frame
        calls = []
        def force_readable(request):
            calls.append(request)
            if len(calls) < 3:
                raise ValueError('Synthetic oversized quoted seed')
            validate_native_frame(request)
        with patch('source_layout.source_seed.validate_native_frame', force_readable):
            generation, seed, binding = seed_retained_bundle(code, 'asset/')
        self.assertIn("r" + "'" * 3, seed['code'])
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'asset').mkdir()
            scope = {'beaver_output': lambda p: str(root/p), 'beaver_input': lambda p: str(root/p)}
            exec(seed['code'], scope)
            self.assertEqual((root/binding['path']).read_bytes(), retained.encode('utf-8'))
            self.assertEqual((root/binding['runtime_path']).read_bytes(), runtime.encode('utf-8'))
            exec(generation, scope)
            self.assertEqual(scope['result'], len('curve '*6000))

    def test_two_native_jobs_keep_exact_sources_and_loader(self):
        retained, code = self.fixture()
        runtime = 'label = ' + repr('curve '*6000) + '\nresult = len(label)\n'
        code += f'exec(compile({runtime!r}, "runtime", "exec"), globals())\n'
        from source_layout.transport_sources import validate_native_frame
        calls = []
        def force_two_jobs(request):
            calls.append(request)
            if len(calls) <= 3:
                raise ValueError('Synthetic oversized combined source jobs')
            validate_native_frame(request)
        with patch('source_layout.source_seed.validate_native_frame', force_two_jobs):
            generation, jobs, binding = seed_retained_bundle(code, 'asset/')
        self.assertEqual(binding['schema_version'], 3)
        self.assertEqual(len(jobs), 2)
        self.assertTrue(all(len(job['outputs']) == 1 for job in jobs))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root/'asset').mkdir()
            scope = {'beaver_output': lambda p:str(root/p), 'beaver_input': lambda p:str(root/p)}
            exec(jobs[0]['code'], scope)
            with self.assertRaises(FileNotFoundError): exec(generation, scope)
            exec(jobs[1]['code'], scope)
            self.assertEqual((root/binding['path']).read_bytes(), retained.encode())
            self.assertEqual((root/binding['runtime_path']).read_bytes(), runtime.encode())
            exec(generation, scope)
            self.assertEqual(scope['result'], len('curve '*6000))

    def test_readable_literal_preserves_every_character(self):
        values = ['', "'", "''", "'"*3, "'"*12, chr(92), chr(92)+"'"*3,
                  chr(92)+'\n', '中🙂\n', 'a\rb', 'a\r\nb', '\x00', '"""x"""']
        for value in values:
            with self.subTest(value=repr(value)):
                literal = readable_source_literal(value)
                actual = eval(compile(literal, '<literal test>', 'eval'), {'__builtins__': {}})
                self.assertEqual(actual.encode('utf-8'), value.encode('utf-8'))
        self.assertEqual(readable_source_literal('a\r\nb'), repr('a\r\nb'))

    def test_missing_retained_source_rejected(self):
        with self.assertRaises(ValueError):
            seed_retained_bundle('answer = 1\n', 'asset/')

    def test_large_runtime_source_is_readable_hash_bound_and_exact(self):
        retained,code=self.fixture()
        runtime='label = '+repr('鼻侧线流 '*9000)+'\nresult = len(label)\n'
        code += f'exec(compile({runtime!r}, "runtime", "exec"), globals())\n'
        generation,seed,binding=seed_retained_bundle(code,'asset/')
        self.assertLess(len(generation),len(code)//2)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);(root/'asset').mkdir()
            scope={'beaver_output':lambda p:str(root/p),'beaver_input':lambda p:str(root/p)}
            exec(seed['code'],scope)
            stored=(root/binding['path']).read_text()
            self.assertTrue(stored.startswith(retained))
            self.assertIn('NATIVE_RUNTIME_SOURCE = ',stored)
            exec(generation,scope)
            self.assertEqual(scope['result'],len('鼻侧线流 '*9000))
            self.assertEqual((root/'asset/semantic_bundle.py').read_text(),retained)
            (root/binding['path']).write_text(stored.replace('result = len','result = 1+len'))
            with self.assertRaisesRegex(ValueError,'seed changed'):exec(generation,scope)


if __name__ == '__main__':
    unittest.main()

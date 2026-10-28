"""Synthetic receipt fixtures test rejection paths, not native execution."""

import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.seed_evidence import finalize_seed_plan, verify_seed, verify_retained_source


class SeedEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.path = 'assets/aster/general_face_eyes_combo_test/source_seed_semantic_bundle.py'
        self.file = self.root / self.path
        self.file.parent.mkdir(parents=True)
        self.file.write_bytes(b'answer = 42\n')
        self.binding = {
            'path': self.path,
            'sha256': hashlib.sha256(self.file.read_bytes()).hexdigest(),
            'native_seed_script_sha256': hashlib.sha256(b'seed fixture').hexdigest(),
        }
        self.job = {
            'jobId': 'unit-test-seed', 'status': 'succeeded',
            'process': {'workspace': str(self.root),
                        'scriptSha256': self.binding['native_seed_script_sha256']},
            'result': {'success': True, 'exitCode': 0,
                       'outputs': [{'path': self.path, 'sha256': self.binding['sha256']}]},
        }
        self.receipt = self.root / 'synthetic-seed-receipt.json'
        self.write_receipt(self.job)

    def write_receipt(self, job):
        self.receipt.write_text(json.dumps({'response': {'result': {'result': job}}}),
                                encoding='utf-8')

    def test_finalize_binds_original_receipt_and_new_generation_identity(self):
        code = f'report["authored_source_seed"] = {self.binding!r}\n'
        plan = {'source_seed_binding': self.binding,
                'generation': {'code': code, 'outputs': [], 'timeoutSeconds': 120},
                'source_packaging': {}}
        before = copy.deepcopy(plan)
        result = finalize_seed_plan(plan, self.receipt, self.root)
        self.assertEqual(plan, before)
        binding = result['source_seed_binding']
        self.assertEqual(binding['job_id'], 'unit-test-seed')
        self.assertEqual(binding['receipt_sha256'], hashlib.sha256(self.receipt.read_bytes()).hexdigest())
        self.assertEqual(result['source_packaging']['source_seed_binding'], binding)
        expected = hashlib.sha256(result['generation']['code'].encode()).hexdigest()
        self.assertEqual(result['source_packaging']['generation_script_sha256'], expected)
        self.assertNotEqual(expected, binding['native_seed_script_sha256'])
        self.assertEqual(verify_seed(binding, self.receipt, self.root), binding)
        with self.assertRaisesRegex(ValueError, 'already finalized'):
            finalize_seed_plan(result, self.receipt, self.root)

    def test_split_runtime_seed_exact_and_tamper_rejection(self):
        runtime = self.file.with_name('source_seed_runtime.py')
        runtime.write_bytes(b'runtime_answer = 17\n')
        self.binding.update(schema_version=2, runtime_path=str(runtime.relative_to(self.root)),
                            runtime_sha256=hashlib.sha256(runtime.read_bytes()).hexdigest())
        self.job['result']['outputs'].append({'path': self.binding['runtime_path'],
                                            'sha256': self.binding['runtime_sha256']})
        self.write_receipt(self.job)
        bound = verify_seed(self.binding, self.receipt, self.root)
        retained = self.file.with_name('semantic_bundle.py')
        retained.write_bytes(self.file.read_bytes())
        relative = str(retained.relative_to(self.root))
        verify_retained_source(bound, self.root, relative)
        code = f'report["authored_source_seed"] = {self.binding!r}\n'
        plan = {'source_seed_binding': self.binding,
                'generation': {'code': code, 'outputs': [], 'timeoutSeconds': 120},
                'source_packaging': {}}
        result = finalize_seed_plan(plan, self.receipt, self.root)
        self.assertEqual(result['source_seed_binding'], bound)
        runtime.write_bytes(b'runtime_answer = 18\n')
        with self.assertRaises(ValueError): verify_seed(bound, self.receipt, self.root)
        with self.assertRaises(ValueError): verify_retained_source(bound, self.root, relative)
        runtime.write_bytes(b'runtime_answer = 17\n')
        for rows in (self.job['result']['outputs'][:1],
                     self.job['result']['outputs'] * 2,
                     self.job['result']['outputs'] + [{'path':'extra.py','sha256':'0'*64}]):
            bad = copy.deepcopy(self.job); bad['result']['outputs'] = rows
            self.write_receipt(bad)
            with self.assertRaises(ValueError): verify_seed(bound, self.receipt, self.root)
        self.write_receipt(self.job)
        bad = dict(bound, runtime_path=bound['path'])
        with self.assertRaises(ValueError): verify_seed(bad, self.receipt, self.root)
        bad = dict(bound, path=bound['runtime_path'], runtime_path=bound['path'])
        with self.assertRaises(ValueError): verify_seed(bad, self.receipt, self.root)
        for key in ('job_id', 'receipt_sha256', 'native_seed_script_sha256'):
            bad = dict(bound); bad[key] = 'wrong'
            with self.assertRaises(ValueError): verify_seed(bad, self.receipt, self.root)

    def test_wrong_native_identity_or_outputs_are_rejected(self):
        cases = []
        script = copy.deepcopy(self.job)
        script['process']['scriptSha256'] = '0' * 64
        cases.append(script)
        workspace = copy.deepcopy(self.job)
        workspace['process']['workspace'] = str(self.root / 'other')
        cases.append(workspace)
        output = copy.deepcopy(self.job)
        output['result']['outputs'][0]['sha256'] = '0' * 64
        cases.append(output)
        failed = copy.deepcopy(self.job)
        failed['status'] = 'failed'
        cases.append(failed)
        for index, job in enumerate(cases):
            with self.subTest(case=index), self.assertRaises(ValueError):
                self.write_receipt(job)
                verify_seed(self.binding, self.receipt, self.root)

    def test_changed_artifact_and_bound_receipt_are_rejected(self):
        bound = verify_seed(self.binding, self.receipt, self.root)
        self.file.write_bytes(b'answer = 43\n')
        with self.assertRaisesRegex(ValueError, 'content changed'):
            verify_seed(self.binding, self.receipt, self.root)
        self.file.write_bytes(b'answer = 42\n')
        self.receipt.write_bytes(self.receipt.read_bytes() + b'\n')
        with self.assertRaisesRegex(ValueError, 'not the bound native job'):
            verify_seed(bound, self.receipt, self.root)

    def test_retained_prefix_and_optional_runtime_pool_are_exact(self):
        retained=self.file.with_name('semantic_bundle.py')
        retained.write_bytes(b'answer = 42\n')
        relative=str(retained.relative_to(self.root))
        verify_retained_source(self.binding,self.root,relative)
        suffix=b'\n\nNATIVE_RUNTIME_SOURCE = "runtime = 12\\n"\n'
        self.file.write_bytes(retained.read_bytes()+suffix)
        binding=dict(self.binding,sha256=hashlib.sha256(self.file.read_bytes()).hexdigest())
        verify_retained_source(binding,self.root,relative)
        retained.write_bytes(b'answer = 4\n')
        with self.assertRaises(ValueError):verify_retained_source(binding,self.root,relative)
        retained.write_bytes(b'answer = 42\n')
        for suffix in (b'\n\nNATIVE_RUNTIME_SOURCE = 3\n',
                       b'\n\nNATIVE_RUNTIME_SOURCE = "x=1"\nextra=1\n',
                       b'\n\nOTHER = "x=1"\n'):
            self.file.write_bytes(retained.read_bytes()+suffix)
            binding['sha256']=hashlib.sha256(self.file.read_bytes()).hexdigest()
            with self.assertRaises(ValueError):verify_retained_source(binding,self.root,relative)


if __name__ == '__main__':
    unittest.main()

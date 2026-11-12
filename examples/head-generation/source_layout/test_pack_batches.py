"""Native packaging transport batching, without executing a model engine."""
import sys
from pathlib import Path
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.pack_batches import source_packaging_batches, original_packaging_jobs
from source_layout.test_package import PackagingTests, PREFIX
from source_layout.evidence import verify_packaging
from source_layout.transport_sources import validate_native_frame


class BatchTransportTests(unittest.TestCase):
    def test_output_limit_keeps_all_sources_once(self):
        payload = {f'file{i}.py': f'x={i}\n' for i in range(71)}
        jobs = source_packaging_batches('from pathlib import Path\n', payload)
        self.assertEqual([len(j['outputs']) for j in jobs], [64, 7])
        self.assertEqual([r['path'] for j in jobs for r in j['outputs']], list(payload))
        for job in jobs:
            validate_native_frame(job)
            compile(job['code'], '<test>', 'exec')

    def test_escaped_frame_is_split_and_single_oversize_rejected(self):
        jobs = source_packaging_batches('', {'a': '\\'*150000, 'b': '\\'*150000})
        self.assertEqual(len(jobs), 2)
        for job in jobs: validate_native_frame(job)
        with self.assertRaises(ValueError):
            source_packaging_batches('', {'a': '\\'*1100000})

    def test_empty_payload_rejected(self):
        with self.assertRaises(ValueError): source_packaging_batches('', {})


class OriginalBatchReceiptTests(PackagingTests):
    def test_disjoint_original_receipts_then_overlap_and_missing(self):
        request = self.request()
        exec(request['code'], {'beaver_input': lambda p: str(self.root/p),
                              'beaver_output': lambda p: str(self.root/p)})
        names = [r['path'] for r in request['outputs']]
        a, b = self.root/'a.json', self.root/'b.json'
        self.write_receipt(a, 'pack-a', names[:3])
        self.write_receipt(b, 'pack-b', names[3:])
        actual = verify_packaging(self.report, self.root, PREFIX, self.gen, [a, b])
        self.assertEqual(set(actual), set(names))
        with self.assertRaises(ValueError):
            verify_packaging(self.report, self.root, PREFIX, self.gen, [a])
        with self.assertRaises(ValueError):
            original_packaging_jobs([a, a], self.root, 'gen')
        self.write_receipt(b, 'pack-b', names[2:])
        with self.assertRaises(ValueError):
            original_packaging_jobs([a, b], self.root, 'gen')
        self.write_receipt(b, 'gen', names[3:])
        with self.assertRaises(ValueError):
            original_packaging_jobs([a, b], self.root, 'gen')

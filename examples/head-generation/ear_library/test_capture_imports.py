"""Only bound screenshot sidecars may use Skip File; runtime assets stay imported."""

import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from PIL import Image

from capture_imports import requests, SKIP


class CaptureImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.folder = self.root / 'artifacts/mouth_motion/probe-test'
        self.folder.mkdir(parents=True)
        self.image = self.folder / 'frame_000.png'
        Image.new("RGB", (8, 8), "white").save(self.image)
        self.row = {'image': 'res://' + str(self.image.relative_to(self.root)),
                    'sha256': hashlib.sha256(self.image.read_bytes()).hexdigest()}
        self.manifest = self.folder / 'manifest.json'

    def prepare(self, rows):
        self.manifest.write_text(json.dumps({'captures': rows}))
        return requests(self.root, [self.manifest])

    def test_only_public_sidecar_is_output_and_operation_is_idempotent(self):
        jobs = self.prepare([self.row])
        self.assertEqual(jobs[0]['outputs'], [{'path': str(self.image.relative_to(self.root)) + '.import', 'expectedSha256': None}])
        self.assertIn(self.row['sha256'], jobs[0]['code'])
        Path(str(self.image) + '.import').write_text(SKIP)
        self.assertEqual(self.prepare([self.row]), [])

    def test_wrong_digest_duplicate_and_runtime_asset_rejected(self):
        for rows in [[{**self.row, 'sha256': '0' * 64}], [self.row, self.row],
                     [{**self.row, 'image': 'res://assets/aster/frame_000.png'}]]:
            with self.assertRaises(ValueError):
                self.prepare(rows)

    def test_generated_job_preserves_source_and_rejects_stale_input(self):
        source_before = self.image.read_bytes()
        sidecar = Path(str(self.image) + '.import')
        sidecar.write_text('original import configuration')
        job = self.prepare([self.row])[0]
        self.assertEqual(job['outputs'][0]['expectedSha256'], hashlib.sha256(sidecar.read_bytes()).hexdigest())
        staging = self.root / 'staged.import'
        context = {'beaver_input': lambda path: self.root / path,
                   'beaver_output': lambda path: staging}
        exec(job['code'], context)
        self.assertEqual(staging.read_text(), SKIP)
        self.assertEqual(self.image.read_bytes(), source_before)
        self.assertEqual(sidecar.read_text(), 'original import configuration')
        staging.unlink()
        self.image.write_bytes(source_before + b'changed')
        with self.assertRaises(AssertionError):
            exec(job['code'], context)
        self.assertFalse(staging.exists())

    def test_signature_without_image_payload_rejected(self):
        self.image.write_bytes(b'\x89PNG\r\n\x1a\n')
        self.row['sha256'] = hashlib.sha256(self.image.read_bytes()).hexdigest()
        with self.assertRaises(ValueError):
            self.prepare([self.row])

    def test_native_output_limit_is_preserved(self):
        rows = []
        for i in range(65):
            p = self.folder / f'frame_{i:03d}.png'
            p.write_bytes(self.image.read_bytes())
            rows.append({**self.row, 'image': 'res://' + str(p.relative_to(self.root))})
        jobs = self.prepare(rows)
        self.assertEqual([len(x['outputs']) for x in jobs], [64, 1])


if __name__ == '__main__':
    unittest.main()

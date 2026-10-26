"""Hash-bound source seed transport is opt-in and rejects changed source bytes."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.source_seed import seed_retained_bundle


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

    def test_missing_retained_source_rejected(self):
        with self.assertRaises(ValueError):
            seed_retained_bundle('answer = 1\n', 'asset/')


if __name__ == '__main__':
    unittest.main()

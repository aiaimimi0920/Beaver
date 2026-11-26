"""Platform newline simulation for byte-bound native source artifacts."""

import hashlib
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.source_seed import seed_retained_bundle
from source_layout.text_outputs import canonical_text_outputs


def windows_write_text(path, data, encoding=None, errors=None, newline=None):
    """Model Windows defaults without claiming a native Windows execution."""
    with path.open('w', encoding=encoding or 'cp1252', errors=errors,
                   newline='\r\n' if newline is None else newline) as stream:
        return stream.write(data)


class CanonicalTextTests(unittest.TestCase):
    def test_seed_and_retained_bytes_match_on_windows_text_defaults(self):
        retained = ('# 菲的源文件\nRETAINED_UPPER_SHAPE_SOURCES = {}\n'
                    'RETAINED_BROW_SOURCES = {}\n')
        code = ('from pathlib import Path\nimport json\nreport={}\n'
                f'Path(beaver_output("asset/semantic_bundle.py")).write_text({retained!r})\n')
        code = canonical_text_outputs(code, 'asset/')
        generation, seed, binding = seed_retained_bundle(code, 'asset/')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'asset').mkdir()
            scope = {'beaver_output': lambda p: str(root / p),
                     'beaver_input': lambda p: str(root / p)}
            with patch.object(Path, 'write_text', windows_write_text):
                exec(seed['code'], scope)
                exec(generation, scope)
            for name in (binding['path'], 'asset/semantic_bundle.py'):
                actual = (root / name).read_bytes()
                self.assertEqual(actual, retained.encode('utf-8'))
                self.assertEqual(hashlib.sha256(actual).hexdigest(), binding['sha256'])

    def test_trailing_comma_comments_and_idempotence(self):
        code = ('from pathlib import Path\n'
                'Path(beaver_output("asset/a.py")).write_text("x\\n", # comment\n)\n')
        canonical = canonical_text_outputs(code, 'asset/')
        self.assertEqual(canonical_text_outputs(canonical, 'asset/'), canonical)
        compile(canonical, '<test>', 'exec')
        self.assertIn('# comment', canonical)

    def test_unrelated_outputs_are_not_rewritten(self):
        code = 'Path(beaver_output("other/file.py")).write_text("x")\n'
        self.assertEqual(canonical_text_outputs(code, 'asset/'), code)

    def test_conflicting_options_fail_closed(self):
        for option in ("encoding='latin1'", "newline='\\r\\n'"):
            with self.subTest(option=option), self.assertRaises(ValueError):
                canonical_text_outputs(
                    f'Path(beaver_output("asset/a.py")).write_text("x",{option})', 'asset/')


if __name__ == '__main__':
    unittest.main()

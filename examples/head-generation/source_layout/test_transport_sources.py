"""Source sharing must preserve behavior and provenance, not bypass native gates."""

import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from source_layout.transport_sources import share_source_literals


class TransportSourcesTests(unittest.TestCase):
    def test_exact_source_execution_and_written_bytes_with_unicode_offsets(self):
        source = '# 原始源码\n' + '# retained source\n' * 400 + 'answer = 42\n'
        code = ('from __future__ import annotations\nfrom pathlib import Path\n'
                + 'label = "菲"; exec(compile(' + repr(source) + ', "source.py", "exec"))\n'
                + 'Path(output).write_text(' + repr(source) + ', encoding="utf-8")\n')
        actual = share_source_literals(code)
        self.assertLess(len(actual.encode()), len(code.encode()))
        self.assertEqual(actual, share_source_literals(code))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / 'source.py'
            scope = {'output': str(output)}
            exec(compile(actual, '<test>', 'exec'), scope)
            self.assertEqual(output.read_bytes(), source.encode('utf-8'))
            self.assertEqual(scope['answer'], 42)
            self.assertEqual(scope['label'], '菲')

    def test_docstring_future_import_and_name_collision_preserved(self):
        source = '# payload\n' * 500 + 'answer = 3\n'
        code = ('"""Preserved module documentation."""\n'
                'from __future__ import annotations\n'
                '_beaver_shared_source_0 = "existing"\n'
                + 'compiled = compile(' + repr(source) + ', "source.py", "exec")\n'
                + 'destination.write_text(' + repr(source) + ')\n')
        actual = share_source_literals(code)
        self.assertEqual(ast.get_docstring(ast.parse(actual)), 'Preserved module documentation.')
        self.assertIn('__beaver_shared_source_0 = ', actual)
        compile(actual, '<test>', 'exec')

    def test_non_source_constants_and_single_occurrences_unchanged(self):
        value = 'x' * 5000
        for code in ('first = ' + repr(value) + '\nsecond = ' + repr(value),
                     'compiled = compile(' + repr(value) + ', "x", "exec")'):
            self.assertEqual(share_source_literals(code), code)


if __name__ == '__main__':
    unittest.main()

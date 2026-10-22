"""Source sharing must preserve behavior and provenance, not bypass native gates."""

import ast
import json
from pathlib import Path
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from source_layout.transport_sources import share_source_literals, validate_native_frame


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

    def test_same_line_docstring_and_future_import_execute_safely(self):
        source = '# retained source\n' * 300 + 'answer = 7\n'
        for header in ['"doc"; ', 'from __future__ import annotations; ',
                       '"doc"; from __future__ import annotations; ']:
            code = header + 'exec(compile(' + repr(source) + ', "x", "exec"))\n'
            code += 'destination.write_text(' + repr(source) + ')\n'
            with tempfile.TemporaryDirectory() as directory:
                destination = Path(directory) / 'source.py'
                scope = {'destination': destination}
                exec(share_source_literals(code), scope)
                self.assertEqual(scope['answer'], 7)
                self.assertEqual(destination.read_text(), source)
                if header.startswith('"doc"'):
                    self.assertEqual(scope['__doc__'], 'doc')

    def test_nfkc_identifier_collision_keeps_source_and_existing_binding(self):
        source = '# retained source\n' * 300 + 'answer = 11\n'
        code = '_ｂeaver_shared_source_0 = "existing"\n'
        code += 'exec(compile(' + repr(source) + ', "x", "exec"))\n'
        code += 'destination.write_text(' + repr(source) + ')\n'
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / 'source.py'
            scope = {'destination': destination}
            exec(share_source_literals(code), scope)
            self.assertEqual(scope['_beaver_shared_source_0'], 'existing')
            self.assertEqual(scope['answer'], 11)
            self.assertEqual(destination.read_text(), source)

    def test_decorator_evaluated_after_shared_sources_exist(self):
        source = '# retained source\n' * 300 + 'answer = 13\n'
        code = '@decorator(compile(' + repr(source) + ', "x", "exec"))\n'
        code += 'def target():\n    return 1\n'
        code += 'destination.write_text(' + repr(source) + ')\n'
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / 'source.py'
            scope = {'destination': destination, 'decorator': lambda code: lambda fn: fn}
            exec(share_source_literals(code), scope)
            self.assertEqual(scope['target'](), 1)
            self.assertEqual(destination.read_text(), source)

    def test_serialized_escape_expansion_rejected_below_decoded_limit(self):
        code = "x = '\\n'\n" * 100000
        self.assertLess(len(code.encode()), 1024 * 1024)
        with self.assertRaisesRegex(ValueError, 'serialized request frame'):
            validate_native_frame({'code': code, 'outputs': [], 'timeoutSeconds': 240})

    def test_longest_escaped_protocol_id_is_within_frame_budget(self):
        request = {'code': '', 'outputs': [], 'timeoutSeconds': 240}
        request['code'] = 'a' * (1024 * 1024 - validate_native_frame(request))
        identity = '\0' * 160
        frame = {'id': '\0' * 200, 'method': 'external.tool', 'input': {
            'taskId': identity, 'runId': identity, 'revision': 2**64 - 1,
            'requestId': identity,
            'arguments': {'tool': 'blender.start', 'arguments': request},
        }}
        wire = json.dumps(frame, ensure_ascii=False, separators=(',', ':')) + '\n'
        self.assertEqual(len(wire.encode()), 1024 * 1024)
        self.assertEqual(validate_native_frame(request), len(wire.encode()))

    def test_complete_frame_boundary_includes_identity_envelope_and_newline(self):
        request = {'code': '', 'outputs': [], 'timeoutSeconds': 240}
        overhead = validate_native_frame(request)
        request['code'] = 'a' * (1024 * 1024 - overhead)
        self.assertEqual(validate_native_frame(request), 1024 * 1024)
        request['code'] += 'a'
        with self.assertRaisesRegex(ValueError, 'serialized request frame'):
            validate_native_frame(request)

    def test_non_source_constants_and_single_occurrences_unchanged(self):
        value = 'x' * 5000
        for code in ('first = ' + repr(value) + '\nsecond = ' + repr(value),
                     'compiled = compile(' + repr(value) + ', "x", "exec")'):
            self.assertEqual(share_source_literals(code), code)


if __name__ == '__main__':
    unittest.main()

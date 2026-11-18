import unittest
from source_layout.python_lines import conservative_effective_lines


class PythonLineTests(unittest.TestCase):
    def test_comments_and_true_docstrings(self):
        source = '"""module doc"""\n# comment\ndef f():\n    """function\n    doc"""\n    return 1 # code\n'
        self.assertEqual(conservative_effective_lines(source), 2)

    def test_assigned_strings_and_fake_comments_are_code(self):
        source = 'text = """data\n# not a comment\n\nend"""\n'
        self.assertEqual(conservative_effective_lines(source), 4)

    def test_nonleading_string_is_not_a_docstring(self):
        self.assertEqual(conservative_effective_lines('x = 1\n"not a docstring"\n'), 2)

    def test_invalid_source_is_rejected(self):
        with self.assertRaises(SyntaxError):
            conservative_effective_lines('def f(:\n')


if __name__ == '__main__':
    unittest.main()

import ast
from pathlib import Path
import unittest
from source_outline_trace import instrument_outline


class OutlineTraceTests(unittest.TestCase):
    def test_trace_is_observational_and_source_guarded(self):
        source = (
            Path(__file__).parent.parent / "checkpoints/current/shell.py"
        ).read_text()
        result = instrument_outline(source)
        ast.parse(result)
        for key in (
            "before_fairing",
            "after_jaw_fairing",
            "after_final_fairing",
            "construction",
            "underside",
        ):
            self.assertIn(key, result)
        self.assertEqual(
            result.count("x, zz = jaw_return_position("),
            source.count("x, zz = jaw_return_position("),
        )
        self.assertEqual(
            result.count("= fair_jaw_boundary("), source.count("= fair_jaw_boundary(")
        )
        with self.assertRaises(ValueError):
            instrument_outline(result)

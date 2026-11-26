import unittest
from pathlib import Path
from source_width_guard import adapt_width_envelope
class WidthGuardAdapterTests(unittest.TestCase):
    def test_original_gate_and_anchor_update_retained(self):
        path=Path(__file__).parent.parent/'checkpoints/current/ear_width.py'
        source=path.read_text(); adapted=adapt_width_envelope(source)
        compile(adapted,'<guarded ear width>','exec')
        self.assertIn("assert not invalid,'Frontal ear-under envelope inverted a face'",adapted)
        self.assertIn("for report,mapping in zip(reports,anchors):",adapted)
        self.assertIn("'lateral_width_guard':lateral_guard",adapted)
        self.assertEqual(adapted.count('limited, lateral_guard ='),1)
    def test_unknown_source_rejected(self):
        with self.assertRaises(ValueError):adapt_width_envelope('pass')
if __name__=='__main__':unittest.main()

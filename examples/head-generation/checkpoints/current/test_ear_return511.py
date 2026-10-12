import unittest
from ear_return import return_weight

class ReturnWidth(unittest.TestCase):
    def test_bounded_compact_width(self):
        self.assertEqual(return_weight(.12),0)
        self.assertEqual(return_weight(.38),0)
        self.assertAlmostEqual(return_weight(.26),1)
        for i in range(1001):
            value=return_weight(i/1000)
            self.assertGreaterEqual(value,0)
            self.assertLessEqual(value,1)
            if i/1000<=.12 or i/1000>=.38:
                self.assertEqual(value,0)

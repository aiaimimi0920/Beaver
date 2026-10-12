import unittest
from ear_return import return_weight

class ReturnWidth(unittest.TestCase):
    def test_bounded_compact_width(self):
        self.assertEqual(return_weight(.11),0)
        self.assertEqual(return_weight(.37),0)
        self.assertAlmostEqual(return_weight(.235),1)
        for i in range(1001):
            value=return_weight(i/1000)
            self.assertGreaterEqual(value,0)
            self.assertLessEqual(value,1)
            if i/1000<=.11 or i/1000>=.37:
                self.assertEqual(value,0)

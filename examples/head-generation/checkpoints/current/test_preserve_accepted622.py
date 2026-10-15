import unittest
from preserve_accepted import is_eye_edit_zone
class Preservation(unittest.TestCase):
 def test_ear_frozen(self):self.assertFalse(is_eye_edit_zone((.107,-.01,1.64)))
 def test_nose_frozen(self):self.assertFalse(is_eye_edit_zone((0,-.114,1.59)))
 def test_eye_allowed(self):self.assertTrue(is_eye_edit_zone((.055,-.08,1.63)))
 def test_lower_face_frozen(self):self.assertFalse(is_eye_edit_zone((.055,-.08,1.58)))

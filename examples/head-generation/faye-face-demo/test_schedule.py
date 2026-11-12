"""Keep capture labels tied to the inherited viewer, not assumed rotation signs."""
import json
from pathlib import Path
import re
import unittest

HERE = Path(__file__).resolve().parent


class CaptureScheduleTests(unittest.TestCase):
    def test_right_views_match_both_retained_viewer_button_definitions(self):
        schedule = json.loads((HERE / 'faye_capture_schedule.json').read_text())
        self.assertEqual(len(schedule), 9)
        for source in [HERE.parent / 'face_library/runtime/head_review.gd',
                       HERE.parents[1] / 'head-review/showcase_extensions/head_review.gd']:
            rows = re.findall(r'\["(正面|右45°|右侧)",\s*([-\d.]+),\s*([-\d.]+)\]', source.read_text())
            angles = {label: (float(yaw), float(pitch)) for label, yaw, pitch in rows}
            self.assertEqual(set(angles), {'正面', '右45°', '右侧'})
            for i, item in enumerate(schedule):
                label = ['正面', '右45°', '右侧'][i % 3]
                self.assertEqual((item['yaw'], item['pitch']), angles[label])
                self.assertEqual(item['view'], ['front', 'right45', 'right90'][i % 3])
                self.assertEqual(item['mode'], ['render', 'white', 'edges'][i // 3])
                self.assertEqual(item['mouth'], {})
                self.assertEqual(item['scale'], .85)


if __name__ == '__main__':
    unittest.main()

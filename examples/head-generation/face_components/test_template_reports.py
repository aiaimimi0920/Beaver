import unittest
from verify_run import verify_template_reports


class TemplateReportTests(unittest.TestCase):
    def test_legacy_and_two_slot_binding(self):
        verify_template_reports({}, {}, {})
        f = {"slot": "face"}
        m = {"slot": "mouth", "seam_bow_height": 0.004, "shoulder_radius_ratio": 0.7}
        snapshot = {"family_templates": [f, m]}
        surface = {
            "family_template": {"template": f, "final_width_stations": [[0, 0.1]]},
            "mouth_template": {"template": m},
        }
        c = {
            "width_stations": [[0, 0.1]],
            "mouth_seam_bow_height": 0.004,
            "mouth_seam_shoulder_radius_ratio": 0.7,
        }
        verify_template_reports(snapshot, surface, c)
        for change in ({"mouth_seam_bow_height": 0}, {"width_stations": []}):
            with self.assertRaises(ValueError):
                verify_template_reports(snapshot, surface, {**c, **change})
        with self.assertRaises(ValueError):
            verify_template_reports({}, surface, c)
        with self.assertRaises(ValueError):
            verify_template_reports({"family_templates": [m, m]}, surface, c)
        with self.assertRaises(ValueError):
            verify_template_reports(
                snapshot, {"family_template": surface["family_template"]}, c
            )

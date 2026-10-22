"""Capture wrappers retain exact schedule, controller and implementation checks."""

from pathlib import Path
import tempfile
import unittest

from capture_bindings import HERE, digest, verify_bindings


class CaptureBindingTests(unittest.TestCase):
    def test_ear_loop_matches_existing_capture_implementation(self):
        source = (HERE.parent / "ear_library/ear_motion_probe.gd").read_text()
        additions = (
            'const EAR_CAPTURE_VIEW = preload("res://showcase_extensions/ear_capture_view.gd")\n\n',
            '\tvar probe_before := FileAccess.get_sha256(get_script().resource_path)\n',
            '\tvar view_before := FileAccess.get_sha256("res://showcase_extensions/ear_capture_view.gd")\n',
            '\tEAR_CAPTURE_VIEW.prepare(self)\n',
            '\t\t"probe_before_sha256": probe_before,\n',
            '\t\t"capture_view_before_sha256": view_before,\n',
            '\t\t"capture_view_after_sha256": FileAccess.get_sha256("res://showcase_extensions/ear_capture_view.gd"),\n',
        )
        for addition in additions:
            self.assertEqual(source.count(addition), 1)
            source = source.replace(addition, "", 1)
        self.assertEqual(source, (HERE / "runtime/mouth_probe.gd").read_text())

    def test_profiles_and_changed_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            deployed = root / "showcase_extensions"
            deployed.mkdir()
            files = {
                "mouth_capture_schedule.json": HERE / "runtime/mouth_capture_schedule.json",
                "researched_mouth_review.gd": HERE / "runtime/mouth_review.gd",
                "researched_mouth_probe.gd": HERE / "runtime/mouth_probe.gd",
                "ear_motion_probe.gd": HERE.parent / "ear_library/ear_motion_probe.gd",
                "ear_capture_view.gd": HERE.parent / "ear_library/ear_capture_view.gd",
            }
            for name, source in files.items():
                (deployed / name).write_bytes(source.read_bytes())
            data = {
                "schedule_sha256": digest(files["mouth_capture_schedule.json"]),
                "controller_sha256": digest(files["researched_mouth_review.gd"]),
                "probe_sha256": digest(files["researched_mouth_probe.gd"]),
            }
            verify_bindings(root, data)
            with self.assertRaises(ValueError):
                verify_bindings(root, data, "ear")
            data["probe_sha256"] = digest(files["ear_motion_probe.gd"])
            data["probe_before_sha256"] = data["probe_sha256"]
            data["capture_view_before_sha256"] = digest(files["ear_capture_view.gd"])
            data["capture_view_after_sha256"] = data["capture_view_before_sha256"]
            verify_bindings(root, data, "ear")
            with self.assertRaises(ValueError):
                verify_bindings(root, data)
            for name, source in files.items():
                if name == "researched_mouth_probe.gd":
                    continue  # The complete ear entry no longer inherits it.
                with self.subTest(changed=name):
                    (deployed / name).write_bytes(source.read_bytes() + b"\n# changed\n")
                    with self.assertRaises(ValueError):
                        verify_bindings(root, data, "ear")
                    (deployed / name).write_bytes(source.read_bytes())
            for key in ("probe_before_sha256", "capture_view_before_sha256", "capture_view_after_sha256"):
                original = data[key]
                data[key] = "changed-at-capture-then-restored"
                with self.assertRaises(ValueError):
                    verify_bindings(root, data, "ear")
                data[key] = original
            with self.assertRaises(ValueError):
                verify_bindings(root, data, "untrusted-manifest-path")


if __name__ == "__main__":
    unittest.main()

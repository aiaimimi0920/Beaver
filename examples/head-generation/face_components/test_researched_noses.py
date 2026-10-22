"""Composition ownership and evidence-tampering regressions for researched noses."""

import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

from compose import HERE, resolve
from nose_evidence import validate_nose_report
from nose_selection import LIBRARY
from inventory import family_inventory

BASE = {
    "eyes": "almond",
    "brows": "research-linear-fine",
    "mouth": "retained",
    "nose": "retained",
    "ears": "retained",
    "face": "retained",
}


def source_module():
    old = sys.path[:]
    sys.path.insert(0, str(LIBRARY))
    try:
        spec = importlib.util.spec_from_file_location(
            "test_nose_source", LIBRARY / "nose_source.py"
        )
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
        return m
    finally:
        sys.path[:] = old


def fixture(kind="balanced"):
    s = resolve(
        {
            **BASE,
            "nose": "research-" + ("faceted" if kind == "faceted-candidate" else kind),
        }
    )
    m = source_module()
    style = json.loads(
        (HERE.parent / "checkpoints/current/face_style.json").read_text()
    )
    style, a = m.apply_design(style, s["nose_design"])
    post = {"selected_vertices": 0, "max_delta_m": 0.0, "outside_nose_unchanged": True}
    report = {
        "nose_design": {
            "parameters": s["nose_design"],
            "source_sha256": s["nose_source_sha256"],
            "attachments": [],
        },
        "facial_surface": {"nose_design": a},
        "nose_refinement": post,
        "nose_arc_fairing": copy.deepcopy(post),
    }
    return s, report, style, m.retained_sources()


class ResearchedNoseTests(unittest.TestCase):
    def test_analytic_contract_rejects_consistent_invalid_reports(self):
        s, report, style, frozen = fixture()
        for mutation in ("mode", "stations", "envelope"):
            r, c = copy.deepcopy(report), copy.deepcopy(style)
            if mutation == "mode":
                c["calibration"]["nose_template_mode"] = "legacy"
            elif mutation == "stations":
                c["calibration"]["nose_stations"] = []
                r["facial_surface"]["nose_design"]["stations"] = []
            else:
                r["facial_surface"]["nose_design"][
                    "authored_relief_upper_bound_m"
                ] = 1e-12
            with self.assertRaises(ValueError):
                validate_nose_report(s, r, c, frozen)

    def test_requested_continuous_composition_identity(self):
        from verify_run import verify_requested_composition

        requested = resolve({**BASE, "nose": "research-balanced"})
        other = resolve(
            {**BASE, "nose": "research-balanced"}, nose_overrides={"projection": 0.1}
        )
        spec = {
            "selection": requested["selection"],
            "fit_policy": "legacy",
            "composition_sha256": requested["composition_sha256"],
        }
        verify_requested_composition(requested, spec)
        with self.assertRaisesRegex(ValueError, "identity mismatch"):
            verify_requested_composition(other, spec)
        spec.pop("composition_sha256")
        with self.assertRaisesRegex(ValueError, "requires requested"):
            verify_requested_composition(requested, spec)

    def test_catalog_and_ownership(self):
        baseline = resolve(BASE)
        catalog = json.loads((HERE / "catalog.json").read_text())
        ids = [n for n in catalog["nose"] if n.startswith("research-")]
        self.assertEqual(len(ids), 5)
        for id in ids:
            s = resolve({**BASE, "nose": id})
            self.assertIn("nose_design", s)
            for key in (
                "eye_parameters",
                "brow_design",
                "brow_source_sha256",
                "lip_parameters",
                "silhouette_parameters",
                "surface_parameters",
            ):
                self.assertEqual(s[key], baseline[key])
        for id in ("retained", "defined-bridge", "long-bridge", "short-rounded"):
            self.assertNotIn("nose_design", resolve({**BASE, "nose": id}))

    def test_override_restrictions(self):
        s = {**BASE, "nose": "research-balanced"}
        self.assertEqual(
            resolve(s, nose_overrides={"projection": 0.09})["nose_design"][
                "projection"
            ],
            0.09,
        )
        for v in (
            {"kind": "pad-muzzle"},
            {"eye_width": 1},
            {"version": 2},
            {"projection": True},
            {"projection": float("nan")},
            {"pad_width": 0.03},
        ):
            with self.assertRaises(ValueError):
                resolve(s, nose_overrides=v)
        with self.assertRaises(ValueError):
            resolve(BASE, nose_overrides={})

    def test_no_mixed_slot_contract(self):
        c = json.loads((HERE / "catalog.json").read_text())
        real = Path.read_text
        for key, value in [
            ("nose_bridge_scale", 1),
            ("template_id", "short-rounded-nose-v1"),
        ]:
            modified = copy.deepcopy(c)
            modified["nose"]["research-balanced"][key] = value

            def read(path, *a, **kw):
                return (
                    json.dumps(modified)
                    if path == HERE / "catalog.json"
                    else real(path, *a, **kw)
                )

            with patch.object(Path, "read_text", read), self.assertRaises(ValueError):
                resolve({**BASE, "nose": "research-balanced"})

    def test_report_binding(self):
        args = fixture()
        self.assertEqual(validate_nose_report(*args)["attachments"], 0)
        for index, mutation in [
            (0, lambda v: v["nose_design"].update(projection=0.1)),
            (1, lambda v: v["nose_design"].update(parameters={})),
            (2, lambda v: v["calibration"].update(nose_stations=[])),
            (3, lambda v: v.update({"nose_field.py": "changed"})),
        ]:
            changed = copy.deepcopy(args)
            mutation(changed[index])
            with self.assertRaises(ValueError):
                validate_nose_report(*changed)

    def test_unbound_and_legacy_postshape_rejected(self):
        s, r, style, frozen = fixture()
        with self.assertRaises(ValueError):
            validate_nose_report(resolve(BASE), r, style, {})
        self.assertIsNone(validate_nose_report(resolve(BASE), {}, {}, {}))
        r["nose_refinement"]["max_delta_m"] = 0.001
        with self.assertRaises(ValueError):
            validate_nose_report(s, r, style, frozen)

    def test_missing_special_metrics_rejected(self):
        with self.assertRaises(ValueError):
            validate_nose_report(*fixture("pad-muzzle"))
        with self.assertRaises(ValueError):
            validate_nose_report(*fixture("faceted-candidate"))

    def test_final_attachment_samples_and_facets_bound(self):
        for kind in ("pad-muzzle", "faceted-candidate"):
            args = fixture(kind)
            r = args[1]
            r["nose_design"]["attachments"] = [
                {
                    "name": "test attachment",
                    "kind": kind,
                    "vertices": 6,
                    "triangles": 8,
                    "closed_two_manifold": True,
                    "signed_volume_m3": 0.000001,
                    "flat_faces": 8,
                }
            ]
            r["nose_final_attachment_checks"] = [
                {
                    "name": "test attachment",
                    "samples": 3,
                    "target_region": 1,
                    "min_back_cap_embedding_m": 0.001,
                    "max_back_cap_embedding_m": 0.002,
                }
            ]
            self.assertEqual(validate_nose_report(*args)["attachments"], 1)
            changed = copy.deepcopy(args)
            changed[1]["nose_final_attachment_checks"][0][
                "min_back_cap_embedding_m"
            ] = -0.001
            with self.assertRaises(ValueError):
                validate_nose_report(*changed)
            if kind == "faceted-candidate":
                changed = copy.deepcopy(args)
                changed[1]["nose_design"]["attachments"][0]["flat_faces"] = 0
                with self.assertRaises(ValueError):
                    validate_nose_report(*changed)

    def test_plan_keeps_overrides_and_fresh_status(self):
        from plan import build_plan

        plan = build_plan(
            {**BASE, "nose": "research-balanced"}, nose_overrides={"projection": 0.09}
        )
        self.assertEqual(plan["snapshot"]["nose_design"]["projection"], 0.09)
        self.assertEqual(plan["current_output_status"], "not_generated")
        self.assertTrue(all(step["status"] == "pending" for step in plan["steps"]))

    def test_three_common_presets_are_one_core(self):
        f = family_inventory()["nose"]["families"]
        self.assertEqual(
            set(f["research-fused-core"]["presets"]),
            {"research-compact", "research-balanced", "research-elongated"},
        )
        self.assertEqual(
            f["research-faceted-candidate"]["presets"], ["research-faceted"]
        )


if __name__ == "__main__":
    unittest.main()

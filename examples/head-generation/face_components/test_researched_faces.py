"""Researched face choices and receipt-bound calibration regression tests."""

import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from compose import HERE, resolve
from face_selection import resolve_face, module, LIBRARY
from face_evidence import validate_face_report

BASE = json.loads((HERE.parent / "checkpoints/current/face_style.json").read_text())
SELECTION = json.loads((HERE / "example-selection.json").read_text())


class ResearchedFaceTests(unittest.TestCase):
    def fixture(self):
        snap = resolve({**SELECTION,"face":"research-compact-rounded"})
        style, record = module().apply_design(BASE,snap['face_design'])
        report = {'face_design':record,'facial_surface':{'face_design':copy.deepcopy(record)}}
        frozen={'face_design.py':(LIBRARY/'face_design.py').read_text()}
        return snap,report,style,frozen

    def test_legacy_selection_has_no_design(self):
        snap=resolve(SELECTION)
        self.assertNotIn('face_design',snap)
        with self.assertRaises(ValueError):resolve(SELECTION,face_overrides={})

    def test_each_preset_and_control_is_bound(self):
        for name in ('compact-rounded','soft-tapered','straight-tapered','blunt-chin'):
            selection={**SELECTION,'face':'research-'+name}
            a=resolve(selection);b=resolve(selection,face_overrides={'chin_projection':.004})
            self.assertEqual(a['face_design']['kind'],name)
            self.assertNotEqual(a['composition_sha256'],b['composition_sha256'])
            for key in ('eye_parameters','surface_parameters','lip_parameters','component_parameters'):
                self.assertEqual(a[key],b[key])

    def test_plan_preserves_explicit_controls(self):
        from plan import build_plan
        selection={**SELECTION, 'face':'research-soft-tapered'}
        controls={'chin_projection':.004}
        result=build_plan(selection,records=[],face_overrides=controls)
        self.assertEqual(result['snapshot']['composition_sha256'],resolve(selection,face_overrides=controls)['composition_sha256'])

    def test_bad_face_overrides_rejected(self):
        selection={**SELECTION,'face':'research-compact-rounded'}
        for controls in ({'version':2},{'kind':'other'},{'eye_width':1},{'chin_projection':True},{'chin_projection':.03}):
            with self.assertRaises(ValueError):resolve(selection,face_overrides=controls)
        with self.assertRaises(ValueError):resolve_face({'design':'compact-rounded','chin_width':.01})

    def test_exact_report_calibration_passes(self):
        result=validate_face_report(*self.fixture())
        self.assertEqual(result['status'],'native_face_calibration_bound')

    def test_mismatched_report_and_sources_rejected(self):
        for target in ('style','record','analytic','source','hash','parameters'):
            snap,report,style,frozen=self.fixture()
            if target=='style':style['calibration']['width_stations'][0][1]+=.001
            if target=='record':report['face_design']['continuous_half_width_change_bound_m']=0
            if target=='analytic':report['facial_surface']['face_design']['parameters']['chin_projection']=.005
            if target=='source':frozen['face_design.py']+='\n'
            if target=='hash':snap['face_source_sha256']['face_design.py']='0'*64
            if target=='parameters':snap['face_design']['chin_projection']=.005
            with self.subTest(target=target),self.assertRaises(ValueError):validate_face_report(snap,report,style,frozen)

    def test_self_reported_shifted_depth_baseline_rejected(self):
        snap,report,style,frozen=self.fixture()
        for entry in report['face_design']['retained_front_profile']:
            entry[1] += .1
        report['facial_surface']['face_design']=copy.deepcopy(report['face_design'])
        with self.assertRaisesRegex(ValueError, 'Retained face depth calibration'):
            validate_face_report(snap,report,style,frozen)

    def test_unbound_face_design_rejected(self):
        snap,report,style,frozen=self.fixture()
        snap.pop('face_design')
        with self.assertRaises(ValueError):validate_face_report(snap,report,style,frozen)

    def test_underside_lift_does_not_rewrite_front_shell_lift(self):
        path = str(HERE.parent / "facial_surface")
        sys.path.insert(0, path)
        try:
            from source_face_design import adapt_sources
            with tempfile.TemporaryDirectory() as directory:
                target = Path(directory)
                for name in ("shell.py", "ear_boundary.py"):
                    source = HERE.parent / "checkpoints/current" / name
                    (target / name).write_text(source.read_text())
                adapt_sources(target, BASE["calibration"]["width_stations"])
                source = (target / "shell.py").read_text()
                self.assertIn('researched_underside_lift(C, lift) * q * q - drop', source)
                self.assertIn('def researched_underside_lift(config, lift):', source)
                self.assertIn('C["chin_return_lift"] * q * q * exp(', source)
                self.assertIn('guard_outline', source)
        finally:
            sys.path.remove(path)

    def test_real_packaged_calibration_and_source_are_bound(self):
        from bundle import build_plan
        sys.path.insert(0,str(HERE.parent))
        from source_layout.package import literal_writes
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            required=['project.godot','authoring/head_generation_guide.md']+['assets/aster/head_recovery_49/'+name for name in ('aster_head_editable.blend','aster_head.glb','aster_definition.tres','hair_base.png','hair_ilm.png')]
            for relative in required:
                path=root/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_text('CPU-only fixture')
            selection={**SELECTION,'face':'research-straight-tapered','mouth':'research-line','nose':'research-balanced'}
            prefix='assets/aster/general_face_eyes_combo_face_cpu/'
            plan=build_plan(selection,prefix,root)
        request=plan['generation'];writes=literal_writes(request['code'])
        style=json.loads(writes[prefix+'face_style.json'][0])
        preflight=json.loads(writes[prefix+'facial_surface_preflight.json'][0])
        record=preflight['report']['face_design']
        report={'face_design':record,'facial_surface':preflight['report']}
        validate_face_report(resolve(selection),report,style,preflight['face_library_sources'])
        self.assertLessEqual(len(request['outputs']),64)
        self.assertLessEqual(len(request['code'].encode()),1024*1024)
        self.assertIn('outline_backtracking_factor',request['code'])


if __name__=='__main__':unittest.main()

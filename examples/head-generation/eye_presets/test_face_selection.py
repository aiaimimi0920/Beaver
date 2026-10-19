"""Fixed face/eye presets cannot inherit evidence after parameter changes."""
from copy import deepcopy
from pathlib import Path
import unittest
from unittest.mock import patch
from test_presets import packager
from face_evidence import validate_fixed_combination

NAMES = ('face-jaw-wide-817', 'face-jaw-narrow-818', 'face-wide-819', 'face-chin-narrow-820')

class FaceSelectionTests(unittest.TestCase):
    def test_exact_snapshot_and_tamper_detection(self):
        for name in NAMES:
            spec=packager.load_preset(name)
            validate_fixed_combination(spec)
            self.assertFalse(spec['evidence']['user_aesthetic_accepted'])
            for field,key in (('eye_parameters','eye_width'),('surface_parameters','axial_smoothing'),('silhouette_parameters',next(iter(spec['silhouette_parameters'])))):
                changed=deepcopy(spec);changed[field][key]+=.001
                with self.assertRaises(ValueError):validate_fixed_combination(changed)
            for key in ('width_gate_passed','visual_self_review_passed'):
                changed=deepcopy(spec);changed['evidence'][key]=False
                with self.assertRaises(ValueError):validate_fixed_combination(changed)

    def test_missing_silhouette_cannot_bypass_loader(self):
        spec=packager.load_preset(NAMES[0])
        del spec['silhouette_parameters']
        with patch.object(packager.json,'loads',return_value=spec):
            with self.assertRaises(ValueError):packager.load_preset(NAMES[0])

    def test_expanded_eye_defaults_are_frozen(self):
        from test_refined_selection import validate
        for name in NAMES:
            spec=packager.load_preset(name)
            # Retained snapshots predate this identity-default control.
            # Require exactly the identity additions; keep stored parameters frozen.
            normalized=validate(spec['eye_parameters'])
            self.assertEqual(set(normalized)-set(spec['eye_parameters']),{'lateral_guide_regularization','brow_angularity','brow_peak_position','brow_band_fullness'})
            self.assertEqual(normalized.pop('lateral_guide_regularization'),1.0)
            self.assertEqual(normalized.pop('brow_angularity'),0.0)
            self.assertEqual(normalized.pop('brow_peak_position'),0.5)
            self.assertEqual(normalized.pop('brow_band_fullness'),0.0)
            self.assertEqual(normalized,spec['eye_parameters'])
            self.assertEqual(set(spec['silhouette_parameters']),{'face_width','jaw_width','chin_width'})
            self.assertIn('orbit_rim_flattening',spec['surface_parameters'])

    def test_packaged_calibration_matches_native(self):
        import ast, hashlib, json, tempfile
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for name in ['project.godot','authoring/head_generation_guide.md'] + [
                'assets/aster/head_recovery_49/'+f for f in
                ('aster_head_editable.blend','aster_head.glb','aster_definition.tres','hair_base.png','hair_ilm.png')]:
                p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text('CPU fixture')
            for name in NAMES:
                request=packager.build_request(name,'assets/aster/general_face_eyes_cpu_replay/',root)
                tree=ast.parse(request['code'])
                style=next(ast.literal_eval(n.value) for n in tree.body
                           if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name)
                           and t.id=='_beaver_bundle_style' for t in n.targets))
                digest=hashlib.sha256(json.dumps(style,sort_keys=True,separators=(',',':')).encode()).hexdigest()
                self.assertEqual(digest,packager.load_preset(name)['evidence']['native_face_style_canonical_sha256'])

    def test_full_configuration_forwarding(self):
        for name in NAMES:
            spec=packager.load_preset(name)
            with patch.object(packager.importlib.util,'module_from_spec') as factory:
                module=factory.return_value
                with patch.object(packager.importlib.util,'spec_from_file_location'):
                    packager.build_request(name,'assets/aster/face_replay/',Path('/tmp/face-replay'))
                args=module.build_request.call_args
                self.assertEqual(args.args[0],spec['surface_parameters'])
                self.assertEqual(args.args[1],spec['eye_parameters'])
                self.assertEqual(args.kwargs['silhouette_parameters'],spec['silhouette_parameters'])

if __name__=='__main__':unittest.main()

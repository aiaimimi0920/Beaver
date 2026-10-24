import ast
import hashlib
import json
import math
import unittest
from pathlib import Path
from test_style_math import pure_function, STYLE
from ast_contract import canonical_function_dump

ROOT = Path(__file__).parent
C = STYLE['calibration']
curve189,_=pure_function('face.py','curve',{})
outline189,_=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})
accent189,_=pure_function('eye_accents.py','accent_sections',{'STYLE':STYLE,'curve':curve189,'outline_sections':outline189})
volume, _ = pure_function('eye_accents.py', 'signed_volume', {})
sections, _ = pure_function('eye_accents.py', 'outline_sections', {'STYLE':STYLE})
mesh_data, _ = pure_function('eye_accents.py', 'closed_ribbon_mesh', {'signed_volume': volume})

class AccentVolumeTests(unittest.TestCase):
    def build(self, side):
        def point(t, h, offset):
            return (side*t*.025, -(.001*math.sin(t)+offset), h*.236)
        return mesh_data(accent189('upper_lash_rails'), point,
                         STYLE['accent_band_thickness_m'], STYLE['accent_band_roll_m'])

    def test_positive_closed_volume_and_consistent_shared_edges(self):
        for side in [-1, 1]:
            vs, fs, zones = self.build(side)
            self.assertGreater(volume(vs, fs), 1e-10)
            edges = {}
            for face in fs:
                self.assertEqual(len(set(face)), len(face))
                for a,b in zip(face, face[1:]+face[:1]):
                    key=tuple(sorted((a,b)))
                    edges.setdefault(key,[]).append(1 if a<b else -1)
            self.assertTrue(all(len(v)==2 and sum(v)==0 for v in edges.values()))
            self.assertTrue({1,2}.issubset(zones))

    def test_mirrored_geometry_and_finite_nonzero_triangles(self):
        a,af,_=self.build(1);b,bf,_=self.build(-1)
        self.assertAlmostEqual(volume(a,af),volume(b,bf),places=15)
        self.assertEqual(a,[(-p[0],p[1],p[2]) for p in b])
        for face in af:
            u=[a[face[1]][k]-a[face[0]][k] for k in range(3)]
            v=[a[face[2]][k]-a[face[0]][k] for k in range(3)]
            cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
            self.assertGreater(sum(x*x for x in cross),1e-20)
        self.assertTrue(all(math.isfinite(x) for p in a for x in p))

    def test_thin_depth_controls_are_bounded_not_a_large_solid(self):
        for k in ['accent_band_thickness_m','accent_wing_thickness_m',
                  'accent_lateral_thickness_m','accent_fold_thickness_m']:
            self.assertGreater(STYLE[k],.0001)
            self.assertLess(STYLE[k],.0008)
        self.assertLess(STYLE['accent_band_bow_m'],.001)
        self.assertLess(STYLE['accent_band_roll_m'],.004)  # fold relief, not solid thickness

    def test_fold_has_fuller_leading_head_and_tapered_ends(self):
        f,_=pure_function('eye_accents.py','fold_width',{'STYLE':STYLE})
        self.assertGreater(f(.2),f(.8))
        self.assertAlmostEqual(f(0),0)
        self.assertAlmostEqual(f(1),0)
        self.assertTrue(all(f(i/100)>=0 for i in range(101)))

    def test_unchanged_regions_have_exact_original_sources(self):
        frozen=json.loads((ROOT/'frozen177-contract.json').read_text())
        for name,sha in frozen['files'].items():
            raw = (ROOT/name).read_text()
            if name == 'shell.py':
                from sampling_legacy_contract import legacy_shell_source
                raw = legacy_shell_source(raw)
            if name == 'textures.py':
                # The only permitted addition is the isolated lash strip. Removing
                # that exact addition must recover the entire frozen texture source.
                start = raw.index('\ndef lash_atlas_color(u):')
                end = raw.index('\ndef bake_base_atlas',start)
                raw = raw[:start] + raw[end:]
                branch = '            if role == "Face" and 0.6 < u < 0.95 and 0.86 < v < 0.89:\n                pixels.extend(lash_atlas_color(u))\n                continue\n'
                self.assertEqual(raw.count(branch),1)
                raw = raw.replace(branch,'')
            if name == 'ears.py':
                from test_upper_ear_bridge519 import EXPECTED_UPPER_BRIDGE
                self.assertEqual(raw.count(EXPECTED_UPPER_BRIDGE),1)
                raw=raw.replace(EXPECTED_UPPER_BRIDGE,'')
                band_line='            obj["ear_return_band_faces"] = 2*(len(rows)-1)\n'
                self.assertEqual(raw.count(band_line),1)
                raw=raw.replace(band_line,'')
                # Reverse only the reviewed498 crown-row edits before legacy verification.
                self.assertEqual(raw.count('            crown = []\n'),1)
                raw=raw.replace('            crown = []\n','')
                self.assertEqual(raw.count('                    crown.append(outer)\n'),1)
                raw=raw.replace('                    crown.append(outer)\n','')
                self.assertEqual(raw.count('                crown.append(len(vs))\n                vs.append(return_crown_point(vs[outer],vs[rim[-1]],t,C["ear_return_crown_m"]))\n                uvpoints.append((0.93,0.02 + 0.96*t))\n'),1)
                raw=raw.replace('                crown.append(len(vs))\n                vs.append(return_crown_point(vs[outer],vs[rim[-1]],t,C["ear_return_crown_m"]))\n                uvpoints.append((0.93,0.02 + 0.96*t))\n','')
                self.assertEqual(raw.count('                for first,second in [([r[-1] for r in rows],crown),(crown,rim)]:\n                    corners = [first[j],second[j],second[j+1],first[j+1]]\n                    clean = list(dict.fromkeys(corners))\n                    if len(clean) >= 3:\n                        fs.append(tuple(clean))\n'),1)
                raw=raw.replace('                for first,second in [([r[-1] for r in rows],crown),(crown,rim)]:\n                    corners = [first[j],second[j],second[j+1],first[j+1]]\n                    clean = list(dict.fromkeys(corners))\n                    if len(clean) >= 3:\n                        fs.append(tuple(clean))\n','                corners = [rows[j][-1], rim[j], rim[j + 1], rows[j + 1][-1]]\n                clean = list(dict.fromkeys(corners))\n                if len(clean) >= 3:\n                    fs.append(tuple(clean))\n')
                self.assertEqual(raw.count('\n\ndef return_crown_point(outer,inner,t,amplitude):\n    """A shallow posterior crown inside the existing open return; endpoints fixed."""\n    from math import sin,pi\n    fade=sin(pi*max(0,min(1,t)))**2\n    return tuple((outer[k]+inner[k])/2+(amplitude*fade if k==1 else 0) for k in range(3))\n'),1)
                raw=raw.replace('\n\ndef return_crown_point(outer,inner,t,amplitude):\n    """A shallow posterior crown inside the existing open return; endpoints fixed."""\n    from math import sin,pi\n    fade=sin(pi*max(0,min(1,t)))**2\n    return tuple((outer[k]+inner[k])/2+(amplitude*fade if k==1 else 0) for k in range(3))\n','')
                addition = '    ease = min(1.0, max(0.0, t / config["ear_lower_sweep_ease_end"]))\n    ease = ease * ease * (3.0 - 2.0 * ease)\n'
                self.assertEqual(raw.count(addition),1)
                raw=raw.replace(addition,'')
                new='config["ear_lower_posterior_sweep"] * sin(pi*t)**0.4 * (1-t)**3 * ease'
                self.assertEqual(raw.count(new),1)
                raw=raw.replace(new,new.removesuffix(' * ease'))
                addition = '                    "lower_bridge_triangle_positions": [list(vs[i]) for i in bridge],\n'
                self.assertEqual(raw.count(addition),1)
                raw=raw.replace(addition,'')
                # Remove only the new audit position; geometry must match baseline.
                addition = '                    "lower_bridge_root_position": list(lower),\n'
                self.assertEqual(raw.count(addition),1)
                raw = raw.replace(addition,'')
                addition = '\n        + (high-low) * C["ear_lower_height_linear_blend"] * max(0,1-2*t)**2 * (t-(1-cos(pi*t))/2)'
                self.assertEqual(raw.count(addition),1)
                raw=raw.replace(addition,'')
            if name == 'shell.py':
                # Only the explicit posterior-return ablation guard may differ.
                new = '    rear_ids, rear_count = [], 0\n    if C["rear_return_enabled"]:\n        verts, shell_faces, rear_ids, rear_count = add_posterior_return(\n            verts, shell_faces, C, H, Y0, Z0\n        )'
                old = '    verts, shell_faces, rear_ids, rear_count = add_posterior_return(\n        verts, shell_faces, C, H, Y0, Z0\n    )'
                self.assertEqual(raw.count(new),1)
                raw=raw.replace(new,old)
                new='    if rear_ids:\n        face["rear_return_outer_vertex_ids"] = rear_ids'
                self.assertEqual(raw.count(new),1)
                raw=raw.replace(new,'    face["rear_return_outer_vertex_ids"] = rear_ids')
            self.assertEqual(hashlib.sha256(raw.encode()).hexdigest(),sha,name)
        # Ear-focused follow-up permits exactly one named calibration change.
        # Keep the historical baseline intact and every other region frozen.
        current={k:v for k,v in C.items() if k not in ('outer_liner_width','ear_return_crown_m','ear_return_crease_normals')}
        self.assertEqual(current['nose_stations'], [[.19,0,.045],[.22,.014,.05],[.25,.050,.066],[.286,.099,.064],[.303,.095,.062],[.34,.038,.041],[.4,0,.038]])
        current['nose_stations']=frozen['calibration']['nose_stations']
        for key,value in [('ear_lower_arc_power',.35),('ear_root_min_height',.25)]:
            self.assertEqual(current[key],value);current[key]=frozen['calibration'][key]
        self.assertEqual(C['ear_return_crown_m'],.002)
        self.assertEqual(frozen['calibration']['ear_lower_posterior_sweep'],.085)
        self.assertEqual(current['ear_lower_posterior_sweep'],.035)
        current['ear_lower_posterior_sweep']=.085
        # Ear-root follow-up repairs only the two explicitly diagnosed reversals.
        current['continuous_back_profile']=[row[:] for row in current['continuous_back_profile']]
        self.assertEqual(current['continuous_back_profile'][6],[.26,-.303])
        self.assertEqual(current['continuous_back_profile'][7],[.28,-.309])
        current['continuous_back_profile'][6]=[.26,-.309]
        current['continuous_back_profile'][7]=[.28,-.306]
        self.assertEqual(current['rear_return_depth_weight_profile'],[[.02,0],[.04,.05],[.08,.6],[.12,.82],[.16,.94],[.2,.5],[.25,.05],[.271,0],[.31,0]])
        current['rear_return_depth_weight_profile']=frozen['calibration']['rear_return_depth_weight_profile']
        self.assertEqual(current.pop('ear_lower_height_linear_blend'),.6)
        self.assertIs(current.pop('rear_return_enabled'),False)
        self.assertEqual(current.pop("ear_lower_fairing_neighbor_rings"),4)  #483 ear-only transition scope
        self.assertEqual(current.pop("ear_lower_sweep_ease_end"),.12)
        self.assertEqual(current.pop('ear_lower_fairing_blend_rings'),3)
        self.assertEqual(current['ear_return_basin_width_fraction'],1.4)
        current['ear_return_basin_width_fraction']=frozen['calibration']['ear_return_basin_width_fraction']
        self.assertEqual(current.pop('ear_root_posterior_support_m'),-.006)
        self.assertEqual(current.pop('ear_to_jaw_rear_curve_m'),.014)
        self.assertEqual(current.pop('ear_jaw_rim_fairing_limit_m'),.012)
        self.assertEqual(current.pop('ear_jaw_rim_fairing_iterations'),240)
        self.assertEqual(current['ear_depth_profile'],[[0,0],[.22,.32],[.7,.66],[.88,.79],[1,1]])
        current['ear_depth_profile']=frozen['calibration']['ear_depth_profile']
        self.assertEqual(current.pop('ear_under_lateral_width_m'),.003);self.assertEqual(current.pop('ear_post_lower_return_width_m'),.007);self.assertEqual(current.pop('ear_post_return_width_m'),.007)
        self.assertEqual(current.pop('ear_post_return_depth_m'),.002)
        self.assertEqual(current,frozen['calibration'])
        tree=ast.parse((ROOT/'face.py').read_text())
        for node in tree.body:
            if isinstance(node,ast.FunctionDef) and node.name in frozen['face_functions']:
                self.assertEqual(canonical_function_dump(ast.dump(node,include_attributes=False)),
                                 canonical_function_dump(frozen['face_functions'][node.name]))

    def test_actual_builder_uses_rolled_band_and_authored_normal_zones(self):
        face=(ROOT/'face.py').read_text();pipeline=(ROOT/'pipeline.py').read_text()
        self.assertIn('build_upper_accents(side)',face)
        self.assertIn('build_lateral_accent(side)',face)
        self.assertIn('build_lid_fold(side)',face)
        self.assertIn('accent_front_faces',pipeline)
        self.assertIn('AccentNormalZone',pipeline)

    def test_lateral_stations_avoid_redundant_near_coincident_loops(self):
        curve,_=pure_function('face.py','curve',{})
        f,_=pure_function('eye_accents.py','lateral_sections',{'STYLE':STYLE,'C':C,'curve':curve})
        station=[p[0] for p in f()]
        self.assertTrue(all(b-a>=.025 for a,b in zip(station,station[1:])))
        self.assertTrue(set(p[0] for p in STYLE['lateral_accent_profile']).issubset(station))

    def test_thin_curved_back_samples_same_ridge_as_front(self):
        # A curved front with an unsampled straight back chord used to cross
        # itself when thickness decreased, reversing the declared front faces.
        def point(t,h,offset):
            return (t*.025,-(10*h*h+offset),h*.236)
        vs,fs,zones=mesh_data(accent189('upper_lash_rails'),point,.00012,.00004)
        projected_front=0.0
        for face,zone in zip(fs,zones):
            if zone not in (1,2):continue
            for j in range(1,len(face)-1):
                a,b,c=[vs[i] for i in (face[0],face[j],face[j+1])]
                projected_front+=(b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])
        self.assertLess(projected_front,0)
        self.assertGreater(volume(vs,fs),0)

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
volume, _ = pure_function('eye_accents.py', 'signed_volume', {})
sections, _ = pure_function('eye_accents.py', 'outline_sections', {'STYLE':STYLE})
mesh_data, _ = pure_function('eye_accents.py', 'closed_ribbon_mesh', {'signed_volume': volume})

class AccentVolumeTests(unittest.TestCase):
    def build(self, side):
        def point(t, h, offset):
            return (side*t*.025, -(.001*math.sin(t)+offset), h*.236)
        return mesh_data(sections(STYLE['upper_lash_outline']), point,
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
        self.assertLess(STYLE['accent_band_roll_m'],.0006)

    def test_fold_has_fuller_leading_head_and_tapered_ends(self):
        f,_=pure_function('eye_accents.py','fold_width',{'STYLE':STYLE})
        self.assertGreater(f(.2),f(.8))
        self.assertAlmostEqual(f(0),0)
        self.assertAlmostEqual(f(1),0)
        self.assertTrue(all(f(i/100)>=0 for i in range(101)))

    def test_unchanged_regions_have_exact_original_sources(self):
        frozen=json.loads((ROOT/'frozen177-contract.json').read_text())
        for name,sha in frozen['files'].items():
            self.assertEqual(hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),sha,name)
        self.assertEqual({k:v for k,v in C.items() if k!='outer_liner_width'},frozen['calibration'])
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
        vs,fs,zones=mesh_data(sections(STYLE['upper_lash_outline']),point,.00012,.00004)
        projected_front=0.0
        for face,zone in zip(fs,zones):
            if zone not in (1,2):continue
            for j in range(1,len(face)-1):
                a,b,c=[vs[i] for i in (face[0],face[j],face[j+1])]
                projected_front+=(b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])
        self.assertLess(projected_front,0)
        self.assertGreater(volume(vs,fs),0)

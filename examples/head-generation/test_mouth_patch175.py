import unittest,math
from test_style_math import pure_function,STYLE
class MouthPatchTests(unittest.TestCase):
 def test_asymmetric_boundary_correspondence_has_positive_projected_quads(self):
  c=STYLE['calibration'];h=c['height_m'];y0=c['origin_y_m'];my=y0+h*c['mouth_height'];gap=h*c['mouth_rest_half_gap']
  seam=lambda x:my+h*c['mouth_corner_lift']*(x/(h*c['mouth_half_width']))**2
  contour,_=pure_function('mouth.py','mouth_half_contour',{'H':h,'C':c,'rest_seam_y':seam,'REST_GAP':gap})
  angles,_=pure_function('mouth.py','boundary_mouth_angles',{'math':math,'MOUTH_Y':my})
  points=[(x*.033,my-.010) for x in [0,.13,.27,.4,.57,.76,1]]
  points +=[(.033,my+y) for y in [0,.01,.020]]
  points +=[(x*.033,my+.020) for x in [.76,.57,.4,.27,.13,0]]
  aa=angles(points);self.assertTrue(all(b>a for a,b in zip(aa,aa[1:])))
  rings=[[(contour(a)[0]*(1-t)+p[0]*t,contour(a)[1]*(1-t)+p[1]*t) for a,p in zip(aa,points)] for t in [0,.3,.65,1]]
  for a,b in zip(rings,rings[1:]):
   for k in range(len(a)-1):
    q=[a[k],b[k],b[k+1],a[k+1]]
    area=sum(v[0]*w[1]-w[0]*v[1] for v,w in zip(q,q[1:]+q[:1]))
    self.assertGreater(area,0)

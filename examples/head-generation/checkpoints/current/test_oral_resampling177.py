import unittest,math
from test_style_math import pure_function,STYLE
class OralResamplingTests(unittest.TestCase):
 def test_inlet_preserved_and_rear_samples_are_elliptical(self):
  c=STYLE['calibration'];h=c['height_m'];my=c['origin_y_m']+h*c['mouth_height'];gap=h*c['mouth_rest_half_gap']
  seam=lambda x:my+h*c['mouth_corner_lift']*(x/(h*c['mouth_half_width']))**2
  fn,_=pure_function('mouth.py','oral_ring_xy',{'C':c,'H':h,'MOUTH_Y':my,'REST_GAP':gap,'rest_seam_y':seam})
  n=30;last=len(c['oral_rings'])-1;back,scale,up,lo=c['oral_rings'][-1]
  for k in range(n):
   angle=-math.pi/2+2*math.pi*k/n;x=h*c['mouth_half_width']*math.cos(angle);y=seam(x)+gap*math.sin(angle)
   self.assertEqual(fn(x,y,k,n,0,1,0,0),(x,y))
   xx,yy=fn(x,y,k,n,last,scale,up,lo);ry=gap+h*(up if math.sin(angle)>0 else lo)
   self.assertAlmostEqual((xx/(h*c['mouth_half_width']*scale))**2+((yy-my)/ry)**2,1,places=9)

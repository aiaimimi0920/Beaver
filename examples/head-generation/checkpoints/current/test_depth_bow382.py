import unittest
from lateral_fit import continuous_guide_moves
class DepthBow382(unittest.TestCase):
 def test_recovers_supported_single_bow_without_chord_gap(self):
  fs=[i/10 for i in range(11)]
  bases=[(.08-.003*f**1.8,-.1+.001*f+.002*4*f*(1-f),1-f) for f in fs]
  moves=continuous_guide_moves(fs,bases)
  self.assertLess(max(abs(d[1]) for d in moves),1e-12)
 def test_preserves_linear_depth(self):
  fs=[0,.25,.5,.75,1];bases=[(.08-.003*f**1.8,-.1+.001*f,1-f) for f in fs]
  self.assertLess(max(abs(d[1]) for d in continuous_guide_moves(fs,bases)),1e-12)
 def test_noise_yields_constant_second_difference(self):
  fs=[i/10 for i in range(11)];bases=[(.08-.003*f**1.8,-.1+.001*f+(.0002 if i%2 else -.0002),1-f) for i,f in enumerate(fs)]
  moves=continuous_guide_moves(fs,bases);ys=[b[1]+d[1] for b,d in zip(bases,moves)]
  second=[ys[i+1]-2*ys[i]+ys[i-1] for i in range(1,10)]
  self.assertLess(max(second)-min(second),1e-12)

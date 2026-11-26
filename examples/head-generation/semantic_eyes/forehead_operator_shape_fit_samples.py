"""Frozen sparse nearest-surface fitting constraints, never final acceptance."""
from collections import OrderedDict
import numpy as np
from chin_source_shape import ChinFrozenDistance

def make_forehead_shape_distance_cache(*args, **kwargs):
    return ForeheadExactDistanceCache(*args, **kwargs)

class ForeheadExactDistanceCache:
    def __init__(self,field,capacity=200000):
        self.field,self.capacity,self.cache=field,int(capacity),OrderedDict()
        self.hits,self.misses=0,0
    def distance(self,points):
        points=np.asarray(points);keys=[(points.dtype.str,p.tobytes())for p in points]
        unknown={k:i for i,k in enumerate(keys)if k not in self.cache}
        fresh={}
        if unknown:
            fresh=dict(zip(unknown,self.field.distance(points[list(unknown.values())])))
        self.misses+=len(unknown);self.hits+=len(keys)-len(unknown)
        result=np.array([fresh[k]if k in fresh else self.cache[k]for k in keys])
        self.cache.update(fresh)
        while len(self.cache)>self.capacity:self.cache.popitem(last=False)
        return result

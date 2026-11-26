"""Candidate caching preserves boundary ties, ambiguity and exact query ownership."""
import unittest
import numpy as np
from forehead_surface_batch import ForeheadSurfaceBatch


# Frozen pre-cache implementation: an independent equivalence oracle.
class LegacyForeheadSurfaceBatch:
    """Batch inversion of current convex authoring quads, with no index hints."""
    def __init__(self, chart, vertices, quads):
        self.xy = np.asarray(chart)[quads][:, :, [0, 2]]
        self.vertices = np.asarray(vertices)[quads]
        self.lower, self.upper = self.xy.min(axis=1), self.xy.max(axis=1)

    def sample(self, points):
        points = np.asarray(points)
        sample_ids, quad_ids = np.where(
            np.all(points[:, None, :] >= self.lower[None, :, :]-1e-8, axis=2)
            & np.all(points[:, None, :] <= self.upper[None, :, :]+1e-8, axis=2))
        q, target = self.xy[quad_ids], points[sample_ids]
        u, v = np.full(len(sample_ids), .5), np.full(len(sample_ids), .5)
        for _ in range(12):
            weights = np.column_stack(((1-u)*(1-v), u*(1-v), u*v, (1-u)*v))
            a = (1-v)[:, None]*(q[:, 1]-q[:, 0])+v[:, None]*(q[:, 2]-q[:, 3])
            b = (1-u)[:, None]*(q[:, 3]-q[:, 0])+u[:, None]*(q[:, 2]-q[:, 1])
            residual = np.einsum('ij,ijk->ik', weights, q)-target
            det = a[:, 0]*b[:, 1]-a[:, 1]*b[:, 0]
            valid = np.abs(det) > 1e-14
            u -= np.divide(residual[:, 0]*b[:, 1]-residual[:, 1]*b[:, 0], det,
                           out=np.zeros_like(det), where=valid)
            v -= np.divide(a[:, 0]*residual[:, 1]-a[:, 1]*residual[:, 0], det,
                           out=np.zeros_like(det), where=valid)
        weights = np.column_stack(((1-u)*(1-v), u*(1-v), u*v, (1-u)*v))
        error = np.linalg.norm(np.einsum('ij,ijk->ik', weights, q)-target, axis=1)
        margin = np.minimum.reduce([u, v, 1-u, 1-v])
        valid = (margin >= -1e-7) & (error < 1e-8)
        sample_ids, quad_ids = sample_ids[valid], quad_ids[valid]
        weights, margin = weights[valid], margin[valid]
        interior_counts = np.bincount(sample_ids[margin > 1e-7], minlength=len(points))
        if np.any(interior_counts > 1):
            return None
        order = np.argsort(margin)
        out = np.full((len(points), 3), np.nan)
        out[sample_ids[order]] = np.einsum('ij,ijk->ik', weights[order], self.vertices[quad_ids[order]])
        return out if np.isfinite(out).all() else None


class FrozenCandidateCacheTests(unittest.TestCase):
    def samplers(self, limit=8):
        chart=np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]]*2,float)
        values=chart.copy();values[4:,1]=1
        quads=np.arange(8).reshape(2,4)
        return (LegacyForeheadSurfaceBatch(chart,values,quads),
                ForeheadSurfaceBatch(chart,values,quads,cache_limit=limit))

    def test_boundary_order_duplicates_dtype_and_eviction(self):
        raw, cached=self.samplers()
        points=np.array([[0,t] for t in np.linspace(.01,.99,50)])
        queries=[points,points[::-1],points[[0,0,1,1,2,2]],points[:3],
                 points.astype(np.float32),points.copy(),np.empty((0,2))]
        for query in queries:
            np.testing.assert_array_equal(raw.sample(query),cached.sample(query))
            self.assertLessEqual(len(cached.candidate_cache),8)
        self.assertEqual(cached.sample(np.empty((0,2))).shape,(0,3))

    def test_invalid_and_ambiguous_candidates_remain_invalid(self):
        raw,cached=self.samplers()
        for query in (np.array([[.5,.5]]),np.array([[2.,2.]]),np.array([[np.nan,0.]])):
            self.assertIsNone(raw.sample(query))
            self.assertIsNone(cached.sample(query))
            self.assertIsNone(cached.sample(query))

    def test_fields_are_not_cached_as_positions(self):
        _,cached=self.samplers()
        points=np.array([[0,.25],[0,.75]])
        before=cached.sample(points)
        cached.vertices += 2.
        np.testing.assert_allclose(cached.sample(points),before+2.,atol=1e-15)


if __name__=='__main__':
    unittest.main()

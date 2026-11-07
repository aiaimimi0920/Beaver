"""Prospective births preserve immutable fields and structured semantic memberships."""
import copy
import unittest
import numpy as np
from forehead_birth_geometry import forehead_birth_solve


def solve_case(permutation,bad_field=False,bad_chart=False,bad_station=False,bad_membership=False):
    canonical=np.array([[-1,0,-1],[1,0,-1],[1,0,1],[-1,0,1],[0,0,-1],[1,0,0],[0,0,1],[-1,0,0],[0,0,0]],float);inverse=np.argsort(permutation);v=canonical[inverse];source=v[:4].copy();params=v[:,[0,2]];faces0=[[0,4,8,7],[4,1,5,8],[8,5,2,6],[7,8,6,3]];faces=[[int(permutation[i])for i in q]for q in faces0];fields=np.zeros((9,3));frozen=np.zeros((4,3));delta=np.zeros((4,3));center=int(permutation[8]);fixed={int(permutation[i])for i in range(4,8)}
    def row(point):
        u,w=(np.asarray(point)+1)/2;return {'donors':[int(permutation[i])for i in range(4)],'weights':[(1-u)*(1-w),u*(1-w),u*w,(1-u)*w]}
    def positions(points):
        if np.any(abs(points)>1+1e-12):return None
        return np.column_stack([points[:,0],np.zeros(len(points)),points[:,1]])
    def field(points):return np.zeros((len(points),3))
    if bad_field:fields[0]=[.001,0,0]
    chart=(lambda points:np.zeros((len(points),2)))if bad_chart else(lambda points:points[:,[0,2]])
    args=dict(vertices=v,faces=faces,paths={'active':[int(permutation[i])for i in [4,8,6]]},parameters=params,existing_count=4,frozen_old_fields=frozen,current_fields=fields,source_sampler=positions,field_sampler=field,row_sampler=row,source_vertices=source,source_fields=delta,actual_chart=chart,movable_old=[],held_nodes=range(4),free_births=[center],fixed_births=fixed,fixed_birth_rows={i:row(params[i])for i in fixed},canonical_parameters={center:[.003,.004]},active_roles={'active'},station_caps={'active':.1},source_discrete={'role':[1]*4,'deform_membership':[[[0,1.]]]*4},discrete_reference={'role':[1]*4,'deform_membership':[[[0,1.]]]*4},phase_evaluations=(30,30),wall_seconds=5.)
    if bad_station:
        args.update(paths={'active':[int(permutation[i])for i in [0,5,8]]},free_births=[],fixed_births=set(range(4,9)),fixed_birth_rows={i:row(params[i])for i in range(4,9)},canonical_parameters={},station_caps={'active':0.})
    if bad_membership:
        args['source_discrete']['.authored_role'] = [1, 1, 2, 1]
        args['discrete_reference']['.authored_role'] = [1, 1, 2, 1]
    before=copy.deepcopy(args['parameters']);out=forehead_birth_solve(**args);assert np.array_equal(before,args['parameters'])and np.array_equal(frozen,np.zeros((4,3)))
    if bad_field or bad_chart or bad_station or bad_membership:
        assert not out['admitted'];assert not any(k in out for k in ['new_birth_rows','old_current_row_updates','chosen_parameters']);return
    assert out['admitted'],out;assert len(out['new_birth_rows'])==5 and not out['old_current_row_updates'];assert out['quantized_audit']['held_exact'];return out['positions'][center]


class ForeheadBirthGeometryTests(unittest.TestCase):
    def test_permutations_and_list_valued_deform_membership(self):
        reference = solve_case(np.arange(9))
        rng = np.random.default_rng(44)
        for _ in range(5):
            permutation = np.r_[rng.permutation(4), 4+rng.permutation(5)]
            np.testing.assert_allclose(solve_case(permutation), reference, atol=1e-8)

    def test_global_old_field_is_not_reset(self):
        solve_case(np.arange(9), bad_field=True)

    def test_actual_chart_and_station_failures_produce_no_rows(self):
        solve_case(np.arange(9), bad_chart=True)
        solve_case(np.arange(9), bad_station=True)

    def test_dot_prefixed_authored_membership_is_not_ignored(self):
        solve_case(np.arange(9), bad_membership=True)


if __name__ == '__main__':
    unittest.main()

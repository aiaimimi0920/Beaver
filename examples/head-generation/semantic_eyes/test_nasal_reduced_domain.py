"""Low-density source topology and bounded lower-cheek domain regression."""
import unittest
from unittest.mock import patch
import numpy as np
from nasal_source_binding import extend_nasal_domain, validate_nasal_handoffs
from nasal_source_geometry import edge_incidence
from nasal_source_template import TEMPLATE
from nasal_source_mesh import claim_nasal_source_face

class ReducedNasalDomainTests(unittest.TestCase):
    def test_five_paths_are_real_disjoint_quad_edges(self):
        quads=TEMPLATE['quads'][:TEMPLATE['core_quads']]
        edges={tuple(sorted((a[1],b[1]))) for q in quads for a,b in zip(q,q[1:]+q[:1])}
        paths=[]
        for row in range(5):
            p=list(range(row*12,row*12+12))
            if row in (1,3):p.insert(9,73 if row==1 else 76)
            self.assertTrue(all(tuple(sorted(e)) in edges for e in zip(p,p[1:])))
            paths.append(p)
        self.assertEqual(len(set(sum(paths,[]))),sum(map(len,paths)))
        self.assertEqual([len(p)-1 for p in paths],[11,12,11,12,11])
        self.assertEqual(len(TEMPLATE['centers']),16)
        self.assertEqual(len(TEMPLATE['quads']),114)

    def test_reusing_selected_quad_requires_same_orientation_and_single_claim(self):
        class Face:
            verts=(0,1,2,3)
        face=Face();claimed=set()
        claim_nasal_source_face(face,[2,3,0,1],{face},claimed)
        self.assertEqual(claimed,{face})
        for selected,reused,order in (({face},claimed,[0,1,2,3]),
                                      (set(),set(),[0,1,2,3]),
                                      ({face},set(),[0,3,2,1])):
            with self.assertRaises(ValueError):claim_nasal_source_face(face,order,selected,reused)

    def test_lateral_handoffs_require_semantic_row_and_two_sided_real_edges(self):
        n=TEMPLATE['boundary_count'];boundary=list(range(100,100+n))
        slots=TEMPLATE['semantic_outer_anchor_slots'];anchors=[boundary[i] for i in slots]
        quads=[tuple(i if kind=='outer' else n+i+(78 if kind=='center' else 0)
                     for kind,i in q) for q in TEMPLATE['quads']]
        self.assertEqual(validate_nasal_handoffs(boundary,anchors,quads),slots)
        with self.assertRaisesRegex(ValueError,'ownership'):
            validate_nasal_handoffs(boundary,anchors[::-1],quads)
        bridge=set((slots[0],n))
        broken=[q for q in quads if not bridge<=set(q)]
        with self.assertRaisesRegex(ValueError,'missing'):
            validate_nasal_handoffs(boundary,anchors,broken)

    def fixture(self):
        v=np.array([(1,0,0),(2,0,0),(2,0,1),(1,0,1),(3,0,0),(3,0,1)],float)
        f=[(0,1,2,3),(1,4,5,2)]
        return v,f,[1,1],{0},[0,1,2,3],{'protected':set()},{e:0 for e in edge_incidence(f)}

    def test_extension_is_unique_adjacent_quad_and_keeps_base_faces(self):
        template={'base_domain':{},'extension_edge_slots':[1,2]}
        with patch('nasal_source_binding.TEMPLATE',template),patch('nasal_source_binding.validate_domain'),patch('nasal_source_binding.canonical_boundary',return_value=[0,1,4,5,2,3]):
            selected,boundary,index=extend_nasal_domain(*self.fixture())
        self.assertEqual(selected,{0,1});self.assertEqual(index,1)
        self.assertEqual(boundary,[0,1,4,5,2,3])

    def test_extension_refuses_protected_vertex_or_crease_group(self):
        template={'base_domain':{},'extension_edge_slots':[1,2]}
        for kind in ('protected','crease','region'):
            args=list(self.fixture())
            if kind=='protected':args[5]['protected'].add(4)
            elif kind=='crease':args[6][(1,4)]=1
            else:args[2][1]=2
            with patch('nasal_source_binding.TEMPLATE',template),patch('nasal_source_binding.validate_domain'):
                with self.assertRaisesRegex(ValueError,'protected interface'):extend_nasal_domain(*args)

    def test_frozen_support_slots_are_unique_and_keep_all_nasal_auxiliaries(self):
        pairs=TEMPLATE['frozen_core_slots']
        self.assertEqual(len({a for a,b in pairs}),len(pairs))
        self.assertEqual([b for a,b in pairs if a>=60],list(range(85,103)))
        self.assertTrue(all(0<=a<78 and 0<=b<103 for a,b in pairs))
        self.assertEqual(TEMPLATE['extension_edge_slots'],[6,7])

if __name__=='__main__':unittest.main()

import copy, unittest
from forehead_lower_rail_rebind import forehead_lower_rail_rebind_guard

class RebindTests(unittest.TestCase):

    def fixture(self):
        v = [[float(i), float(i % 3), 1.0] for i in range(12)]
        old = {'vertices': v, 'faces': [[0, 1, 2, 3], [4, 5, 6, 7]], 'paths': {'blue': [0, 1, 2],
            'purple': [4, 5, 6]}, 'mouth_height': 0.0}
        new = copy.deepcopy(old)
        new['faces'] = [[0, 1, 2, 3], [4, 8, 9, 7], [8, 5, 6, 9]]
        new['paths']['purple'] = [4, 8, 5, 6]
        return (old, new)

    def test_complete_replacement_and_alias(self):
        old, new = self.fixture()
        saved = copy.deepcopy((old, new))
        p, e = forehead_lower_rail_rebind_guard(old, new, {'blue': [[0, 1, 2]], 'purple': [[4, 5, 6]]},
            new['vertices'])
        self.assertEqual((old, new), saved)
        self.assertEqual(set(map(tuple, p['lower_turn_guard']['triples'])), {(0, 1, 2), (4, 8, 5), (8, 5, 6)})
        self.assertTrue(e['records'][1]['retired_numeric_is_offguide_diagnostic'])
        self.assertEqual(e['records'][1]['original_triple'], [4, 5, 6])
        p['vertices'][0][0] = 999
        self.assertEqual((old, new), saved)

    def test_reordered_common_rejected(self):
        old, new = self.fixture()
        new['paths']['purple'] = [6, 5, 8, 4]
        with self.assertRaises(ValueError):
            forehead_lower_rail_rebind_guard(old, new, {'purple': [[4, 5, 6]]}, new['vertices'])

    def test_id_permutation(self):
        old, new = self.fixture()
        for shift in range(5):
            perm = [(i + shift) % 12 for i in range(12)]

            def convert(p):
                q = copy.deepcopy(p)
                for i in range(12):
                    q['vertices'][perm[i]] = p['vertices'][i]
                q['faces'] = [[perm[i] for i in f] for f in p['faces']]
                q['paths'] = {k: [perm[i] for i in v] for k, v in p['paths'].items()}
                return q
            a, b = (convert(old), convert(new))
            p, e = forehead_lower_rail_rebind_guard(a, b, {'blue': [[perm[i] for i in [0, 1, 2]]],
                'purple': [[perm[i] for i in [4, 5, 6]]]}, b['vertices'])
            self.assertEqual(set(map(tuple, p['lower_turn_guard']['triples'])),
                {tuple((perm[i] for i in t)) for t in [(0, 1, 2), (4, 8, 5), (8, 5, 6)]})
if __name__ == '__main__':
    unittest.main()

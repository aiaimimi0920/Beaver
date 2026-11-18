"""Adversarial serializer equality/collision coverage, independent of cache tests."""
import unittest, random
import numpy as np
from forehead_exact_jacobian_cache import forehead_exact_epoch_digest as new

def old(value):
    """Legacy serializer oracle for supported plain-value equivalence tests."""
    import hashlib, json

    def pack(v):
        if isinstance(v, np.ndarray):
            if v.dtype.hasobject or v.dtype.fields is not None or v.dtype.kind not in 'biufc':
                raise ValueError('Unsupported array')
            a = np.ascontiguousarray(v)
            return ['array', a.dtype.str, list(a.shape), hashlib.sha256(a.tobytes()).hexdigest()]
        if isinstance(v, dict):
            if not all((isinstance(k, str) for k in v)):
                raise ValueError('Unsupported key')
            return ['dict', [[k, pack(v[k])] for k in sorted(v)]]
        if isinstance(v, (list, tuple)):
            return ['sequence', [pack(x) for x in v]]
        if v is None or type(v) in (str, bool, int, float):
            return [type(v).__name__, v]
        raise ValueError('Unsupported epoch state')
    return hashlib.sha256(json.dumps(pack(value), separators=(',', ':'), allow_nan=False).encode()).hexdigest()

class EpochSemantics(unittest.TestCase):

    def equivalent(self, a, b):
        self.assertEqual(old(a) == old(b), new(a) == new(b))

    def test_primitive_types(self):
        values = [None, False, True, 0, 1, -1, 0.0, -0.0, 1.0, 1e+20, 10 ** 20, '0', 'None', 'NaN', [], {}, [None], (None,)]
        for a in values:
            for b in values:
                self.equivalent(a, b)

    def test_array_cannot_impersonate_null_or_metadata(self):
        a = np.array([1.0, 2.0])
        meta = ['array', a.dtype.str, list(a.shape), 'crafted']
        for b in (None, [1.0, 2.0], meta, {'array': meta}, {'path': [], 'dtype': '<f8'}):
            self.equivalent(a, b)
        self.equivalent([a, None], [None, a])
        self.equivalent({'0': a}, [a])

    def test_array_path_and_dtype_and_order(self):
        a = np.arange(6.0).reshape(2, 3)
        for b in (a.astype('f4'), a.reshape(6), a.copy() + 1, a.T):
            self.equivalent(a, b)
        self.equivalent(a, np.asfortranarray(a))
        self.equivalent({'b': a, 'a': None}, {'a': None, 'b': a.copy()})
        self.equivalent([a, a], [a.copy(), a.copy()])

    def test_scalar_nan_and_unreviewed_types_rejected(self):

        class Str(str):
            pass

        class Int(int):
            pass
        bad = [float('nan'), float('inf'), -float('inf'), object(), set(), b'x', np.float64(1), np.int64(1), Str('x'), Int(1), {1: 'a'}, {None: 1}, np.zeros(1, dtype=[('a', 'f8')]), np.array(['x']), np.array([object()])]
        for v in bad:
            for fn in (old, new):
                with self.assertRaises((ValueError, TypeError)):
                    fn(v)

    def test_array_payload_bits_are_preserved(self):
        for a, b in [(np.array([0.0]), np.array([-0.0])), (np.array([1.0]), np.array([np.nextafter(1.0, 2.0)])), (np.array([complex(1, 2)]), np.array([complex(1, 3)])), (np.array([float('nan')]), np.array([float('inf')]))]:
            self.equivalent(a, b)

    def test_cyclic_containers_fail_closed(self):
        a = []
        a.append(a)
        for fn in (old, new):
            with self.assertRaises((RecursionError, ValueError)):
                fn(a)

    def test_generated_nested_epoch_equality(self):
        rng = random.Random(7)

        def value(depth=0):
            leaf = [None, False, True, 0, 1, -0.0, 3.14, 'nan', np.array([1.0, 2.0])]
            if depth > 3 or rng.random() < 0.5:
                return rng.choice(leaf)
            if rng.random() < 0.5:
                return [value(depth + 1) for _ in range(rng.randrange(4))]
            return {str(i): value(depth + 1) for i in range(rng.randrange(4))}
        old_seen = {}
        new_seen = {}
        for _ in range(300):
            v = value()
            a, b = (old(v), new(v))
            if a in old_seen:
                self.assertEqual(old_seen[a], b)
            if b in new_seen:
                self.assertEqual(new_seen[b], a)
            old_seen[a] = b
            new_seen[b] = a

class ScopeAndShapeRegressions(unittest.TestCase):

    def test_container_subclasses_fail_closed(self):

        class Mapping(dict):
            factor = 1

            def __getitem__(self, k):
                return dict.__getitem__(self, k) * self.factor

        class Sequence(list):
            pass

        class Tuple(tuple):
            pass

        class Key(str):
            pass
        for v in (Mapping(a=1), Sequence([1]), Tuple((1,)), {Key('x'): 1}):
            with self.assertRaises(ValueError):
                new(v)

    def test_later_container_subclass_rejected_without_side_effect(self):
        earlier = {'x': 1}

        class Later(list):

            def __iter__(self):
                earlier['x'] = 2
                return iter([])
        with self.assertRaises(ValueError):
            new([earlier, Later()])
        self.assertEqual(earlier['x'], 1)

    def test_zero_dimensional_shape_change_invalidates(self):
        a = np.array(1.0)
        b = np.array([1.0])
        self.assertNotEqual(new(a), new(b))
        before = new({'array': a})
        a.shape = (1,)
        self.assertNotEqual(before, new({'array': a}))

    def test_zero_dimensional_complex_bool_and_empty_dimensions(self):
        for scalar in (True, 1, 1.0, 1 + 2j):
            self.assertNotEqual(new(np.array(scalar)), new(np.array([scalar])))
        self.assertNotEqual(new(np.zeros((0,))), new(np.zeros((0, 1))))

    def test_cache_poison_on_zero_dimensional_shape_change(self):
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        a = np.array(3.0)
        f, m = cache(lambda x: np.array([[a.ndim + 1.0]]), lambda: {'a': a}, lambda _: None, 1, 1)
        self.assertEqual(f([0])[0, 0], 1)
        a.shape = (1,)
        with self.assertRaisesRegex(ValueError, 'epoch changed'):
            f([0])
        self.assertTrue(m['invalidated'])

    def test_owned_graph_shape_change_poison(self):
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        from forehead_cache_graph import ForeheadCacheSourceManifest as Manifest, ForeheadCacheGraph as Graph
        text = 'import numpy as np\nARRAY=np.array(3.)\ndef derivative(x):return np.array([[ARRAY.ndim+1.]])\n'
        ns = {'__name__': 'scalar_shape_regression'}
        exec(compile(text, '<shape-regression>', 'exec'), ns)
        manifest = Manifest({'<shape-regression>': text})
        f, m = cache(ns['derivative'], lambda: Graph(manifest).walk(ns['derivative']), lambda _: None, 1, 1)
        self.assertEqual(f([0])[0, 0], 1)
        ns['ARRAY'].shape = (1,)
        with self.assertRaisesRegex(ValueError, 'epoch changed'):
            f([0])
        self.assertTrue(m['invalidated'])

    def test_array_subclasses_and_dtype_metadata_rejected(self):

        class Array(np.ndarray):
            pass
        values = [np.array([1.0]).view(Array), np.ma.array([1.0], mask=False), np.array([1.0], dtype=np.dtype('f8', metadata={'scale': 1})), np.array([1.0], dtype=np.dtype('f8', metadata={}))]
        for v in values:
            with self.assertRaises(ValueError):
                new(v)

    def test_later_metadata_or_masked_array_poison_cache(self):
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        for replacement in (np.ma.array([1.0], mask=True), np.array([1.0], dtype=np.dtype('f8', metadata={'scale': 2}))):
            state = {'a': np.array([1.0])}
            f, m = cache(lambda x: np.array([[1.0]]), lambda: state, lambda _: None, 1, 1)
            f([0])
            state['a'] = replacement
            with self.assertRaises(ValueError):
                f([0])
            self.assertTrue(m['invalidated'])
            state['a'] = np.array([1.0])
            with self.assertRaises(ValueError):
                f([0])

    def test_plain_nested_payload_stays_canonical(self):
        a = np.arange(6.0).reshape(2, 3)
        self.assertEqual(new({'z': [a, ()], 'a': 1}), new({'a': 1, 'z': (a.copy(), [])}))

class SourceGraphScopeRegressions(unittest.TestCase):

    def test_original_container_subclasses_fail_before_graph_erasure(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest

        class Mapping(dict):
            pass

        class Sequence(list):
            pass

        class Tuple(tuple):
            pass

        class Set(set):
            pass

        class Frozen(frozenset):
            pass
        for v in (Mapping(x=1), Sequence([1]), Tuple((1,)), Set([1]), Frozen([1])):
            with self.assertRaises(ValueError):
                Graph(Manifest({})).walk({'nested': v})

    def test_original_array_and_scalar_subclasses_and_metadata_rejected(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest

        class Scalar(np.float64):
            pass
        for v in (Scalar(1), np.ma.array([1.0], mask=True), np.array([1.0], dtype=np.dtype('f8', metadata={'scale': 1}))):
            with self.assertRaises(ValueError):
                Graph(Manifest({})).walk({'nested': v})

    def test_original_plain_numpy_scalar_still_supported(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        for v in (np.float32(1), np.float64(1), np.int32(1), np.int64(1), np.bool_(True)):
            Graph(Manifest({})).walk(v)

    def test_owned_derivative_container_replacement_poison(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        text = 'import numpy as np\nSTATE={"factor":1}\ndef derivative(x):return np.array([[STATE["factor"]]])\n'
        ns = {'__name__': 'container_scope_regression'}
        exec(compile(text, '<container-regression>', 'exec'), ns)
        manifest = Manifest({'<container-regression>': text})
        f, m = cache(ns['derivative'], lambda: Graph(manifest).walk(ns['derivative']), lambda _: None, 1, 1)
        f([0])

        class Mapping(dict):

            def __getitem__(self, k):
                return dict.__getitem__(self, k) * 2
        ns['STATE'] = Mapping(factor=1)
        with self.assertRaises(ValueError):
            f([0])
        self.assertTrue(m['invalidated'])
        ns['STATE'] = {'factor': 1}
        with self.assertRaises(ValueError):
            f([0])
    def test_scalar_subclass_cannot_override_dtype_or_item_to_bypass_guard(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        from types import SimpleNamespace
        invoked = []

        class Evil(np.float64):
            @property
            def dtype(self):
                invoked.append('dtype')
                return SimpleNamespace(type=Evil, metadata=None)

            def item(self):
                invoked.append('item')
                return 1.0

        with self.assertRaisesRegex(ValueError, 'scalar subclass'):
            Graph(Manifest({})).walk(Evil(2.0))
        self.assertEqual(invoked, [])

    def test_sparse_subclasses_rejected_before_state_erasure(self):
        from scipy.sparse import csr_matrix
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest

        class Custom(csr_matrix):
            def tocsr(self, copy=False):
                raise AssertionError('Custom conversion must never run')

        with self.assertRaisesRegex(ValueError, 'sparse subclass'):
            Graph(Manifest({})).walk(Custom([[1.0]]))

    def test_official_sparse_formats_remain_supported(self):
        from scipy import sparse
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        for format_name in ('bsr', 'coo', 'csc', 'csr', 'dia', 'dok', 'lil'):
            for container in ('matrix', 'array'):
                value = getattr(sparse, format_name + '_' + container)(np.eye(2))
                graph = Graph(Manifest({})).walk(value)
                self.assertEqual(graph['shape'], [2, 2])
                new(graph)

    def test_owned_derivative_sparse_subclass_replacement_poison(self):
        from scipy.sparse import csr_matrix
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        text = 'import numpy as np\nfrom scipy.sparse import csr_matrix\nM=csr_matrix([[1.]])\ndef derivative(x):return np.array([[M[0,0]]])\n'
        ns = {'__name__': 'sparse_scope_regression'}
        exec(compile(text, '<sparse-regression>', 'exec'), ns)
        manifest = Manifest({'<sparse-regression>': text})
        f, metadata = cache(ns['derivative'], lambda: Graph(manifest).walk(ns['derivative']), lambda _: None, 1, 1)
        f([0])

        class Custom(csr_matrix):
            pass

        ns['M'] = Custom([[1.]])
        ns['M'].factor = 2
        with self.assertRaisesRegex(ValueError, 'sparse subclass'):
            f([0])
        self.assertTrue(metadata['invalidated'])
        ns['M'] = csr_matrix([[1.]])
        with self.assertRaises(ValueError):
            f([0])

    def test_numpy_scalar_dtype_change_poison_preserves_numeric_behavior(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        from forehead_exact_jacobian_cache import forehead_exact_fit_cache as cache
        text = 'import numpy as np\nS=np.int8(127)\ndef derivative(x):\n    with np.errstate(over="ignore"):\n        return np.array([[float(S+np.int8(1))]])\n'
        ns = {'__name__': 'scalar_dtype_regression'}
        exec(compile(text, '<dtype-regression>', 'exec'), ns)
        manifest = Manifest({'<dtype-regression>': text})
        f, metadata = cache(ns['derivative'], lambda: Graph(manifest).walk(ns['derivative']), lambda _: None, 1, 1)
        self.assertEqual(f([0])[0, 0], -128.0)
        ns['S'] = np.int64(127)
        self.assertEqual(ns['derivative']([0])[0, 0], 128.0)
        with self.assertRaisesRegex(ValueError, 'epoch changed'):
            f([0])
        self.assertTrue(metadata['invalidated'])

    def test_numpy_scalar_bits_dtype_and_extended_precision(self):
        from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as Manifest
        def digest(v):
            return new(Graph(Manifest({})).walk(v))
        for a, b in ((np.int8(1), np.int64(1)), (np.float32(1), np.float64(1)), (np.float64(0.), np.float64(-0.))):
            self.assertNotEqual(digest(a), digest(b))
        value = np.longdouble('1.000000000000000001')
        self.assertEqual(digest(value), digest(value))
        self.assertNotEqual(digest(value), digest(np.float64(1.)))

if __name__ == '__main__':
    unittest.main()

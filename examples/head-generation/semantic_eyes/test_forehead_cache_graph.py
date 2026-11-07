import unittest, types
from forehead_cache_graph import ForeheadCacheGraph as Graph, ForeheadCacheSourceManifest as SourceManifest
from forehead_exact_jacobian_cache import forehead_exact_fit_cache as exact_fit_cache, forehead_exact_epoch_digest as epoch_digest
import numpy as np
from scipy.sparse import csr_matrix
SOURCE = '\nimport numpy as np\nCONST=np.array([2.])\ndef helper(x, factor=3., *, offset=1.):\n    return x*factor + offset + CONST[0]\ndef maker():\n    private=np.array([4.])\n    def derivative(x):\n        def nested():\n            return helper(x) + private\n        return np.asarray([nested()])\n    return derivative\n'

class Tests(unittest.TestCase):

    def setUp(self):
        self.ns = {'__name__': 'not_registered_anywhere'}
        exec(compile(SOURCE, '<unit-owned>', 'exec'), self.ns)
        self.f = self.ns['maker']()
        self.manifest = SourceManifest({'<unit-owned>': SOURCE})

    def epoch(self):
        return Graph(self.manifest).walk(self.f)

    def changed(self, mutate):
        before = epoch_digest(self.epoch())
        mutate()
        self.assertNotEqual(before, epoch_digest(self.epoch()))

    def test_module_name_is_not_ownership(self):
        self.f.__module__ = 'unregistered_flattened'
        self.epoch()

    def test_transitive_global_array(self):
        self.changed(lambda: self.ns['CONST'].__setitem__(0, 8))

    def test_transitive_default(self):
        self.changed(lambda: setattr(self.ns['helper'], '__defaults__', (8.0,)))

    def test_transitive_kwdefault(self):
        self.changed(lambda: self.ns['helper'].__kwdefaults__.__setitem__('offset', 8))

    def test_private_cell(self):
        self.changed(lambda: self.f.__closure__[0].cell_contents.__setitem__(0, 8))

    def test_callable_identity(self):
        h = self.ns['helper']
        replacement = types.FunctionType(h.__code__, h.__globals__, h.__name__, h.__defaults__)
        replacement.__kwdefaults__ = h.__kwdefaults__
        self.changed(lambda: self.ns.__setitem__('helper', replacement))

    def test_unmanifested_code_fails(self):
        self.ns['helper'].__code__ = (lambda x: x).__code__
        with self.assertRaises(ValueError):
            self.epoch()

    def test_same_filename_changed_code_fails(self):
        bad = {}
        exec(compile('def helper(x):return x', '<unit-owned>', 'exec'), bad)
        self.ns['helper'].__code__ = bad['helper'].__code__
        with self.assertRaises(ValueError):
            self.epoch()

    def test_nested_code_names(self):
        self.assertIn('helper', repr(self.epoch()))

    def test_cache_count_capacity_exact_bytes_copy_poison(self):
        charges = []
        request, meta = exact_fit_cache(self.f, self.epoch, charges.append, 1, 1)
        x = np.array([0.0])
        a = request(x)
        a[0, 0] = 999
        self.assertNotEqual(request(x)[0, 0], 999)
        request(np.array([-0.0]))
        self.assertEqual(meta['misses'], 2)
        request(np.array([1.0]))
        request(x)
        self.assertEqual(meta['misses'], 4)
        self.ns['CONST'][0] += 1
        with self.assertRaises(ValueError):
            request(x)
        self.ns['CONST'][0] -= 1
        with self.assertRaises(ValueError):
            request(x)
        self.assertEqual(len(charges), 7)
        self.assertTrue(meta['invalidated'])
if __name__ == '__main__':
    unittest.main()

class ManifestTests(unittest.TestCase):

    def test_manifest_text_mutation_rejected(self):
        manifest = SourceManifest({'<owned>': 'X=1'})
        manifest.sources['<owned>'] = 'X=2'
        with self.assertRaises(ValueError):
            manifest.verify()

    def test_disk_source_mutation_rejected(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'packaged_chunk.py'
            path.write_text('def f():return 1\n')
            manifest = SourceManifest({str(path): path.read_text()})
            manifest.verify()
            path.write_text('def f():return 2\n')
            with self.assertRaises(ValueError):
                manifest.verify()

    def test_reachable_owned_class_methods_and_inheritance(self):
        text = '\nGLOBAL=[1.]\nclass Parent:\n    def helper(self):return GLOBAL[0]\nclass Child(Parent):\n    def method(self,x):return x+self.helper()\ndef root(x):return Child().method(x)\n'
        ns = {'__name__': 'no_module_registration'}
        exec(compile(text, '<classes>', 'exec'), ns)
        manifest = SourceManifest({'<classes>': text})
        first = epoch_digest(Graph(manifest).walk(ns['root']))
        ns['GLOBAL'][0] = 2
        self.assertNotEqual(first, epoch_digest(Graph(manifest).walk(ns['root'])))

class ManifestRegressionTests(unittest.TestCase):

    def test_each_mapping_deleted_key_rejected(self):
        for name in ('sources', 'hashes', 'codes'):
            with self.subTest(mapping=name):
                m = SourceManifest({'<owned>': 'X=1'})
                del getattr(m, name)['<owned>']
                with self.assertRaises(ValueError):
                    m.verify()

    def test_each_mapping_added_key_rejected(self):
        for name in ('sources', 'hashes', 'codes'):
            with self.subTest(mapping=name):
                m = SourceManifest({'<owned>': 'X=1'})
                getattr(m, name)['<extra>'] = 'extra'
                with self.assertRaises(ValueError):
                    m.verify()

    def test_mapping_replacement_and_private_snapshot_rejected(self):
        m = SourceManifest({'<owned>': 'X=1'})
        for name in ('sources', 'hashes', 'codes', '_ForeheadCacheSourceManifest__original'):
            with self.subTest(mapping=name):
                with self.assertRaises(AttributeError):
                    setattr(m, name, {})
                with self.assertRaises(AttributeError):
                    delattr(m, name)
        with self.assertRaises(TypeError):
            m._ForeheadCacheSourceManifest__original['<owned>'] = ('new', 'hash', frozenset())

    def test_sources_and_hashes_cannot_reauthorize_changed_source(self):
        import hashlib
        m = SourceManifest({'<owned>': 'X=1'})
        m.sources['<owned>'] = 'X=2'
        m.hashes['<owned>'] = hashlib.sha256(b'X=2').hexdigest()
        with self.assertRaises(ValueError):
            m.verify()

    def test_codes_cannot_reauthorize_replacement_function(self):
        from forehead_cache_graph import forehead_cache_code_digest as digest, forehead_cache_code_tree as code_tree
        m = SourceManifest({'<owned>': 'def f():return 1'})
        code = compile('def f():return 2', '<owned>', 'exec')
        ns = {}
        exec(code, ns)
        m.codes['<owned>'] = {digest(c) for c in code_tree(code)}
        with self.assertRaises(ValueError):
            m.verify()
        with self.assertRaises(ValueError):
            m.owns(ns['f'])

    def test_deleted_entry_then_changed_disk_rejected(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'chunk.py'
            p.write_bytes(b'def f():return 1\n')
            m = SourceManifest({str(p): p.read_bytes().decode('utf-8')})
            del m.sources[str(p)]
            p.write_bytes(b'def f():return 2\n')
            with self.assertRaises(ValueError):
                m.verify()

    def test_lf_to_crlf_disk_byte_mutation_rejected(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'chunk.py'
            p.write_bytes(b'def f():\n    return 1\n')
            m = SourceManifest({str(p): p.read_bytes().decode('utf-8')})
            m.verify()
            p.write_bytes(b'def f():\r\n    return 1\r\n')
            with self.assertRaises(ValueError):
                m.verify()

    def test_each_mapping_rekey_rejected(self):
        for name in ('sources', 'hashes', 'codes'):
            with self.subTest(mapping=name):
                m = SourceManifest({'<owned>': 'X=1'})
                mapping = getattr(m, name)
                mapping['<other>'] = mapping.pop('<owned>')
                with self.assertRaises(ValueError):
                    m.verify()

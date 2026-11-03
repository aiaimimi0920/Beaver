from pathlib import Path
import sys, tempfile, json, hashlib, types, unittest, os
D = Path(__file__).resolve().parent
R = D.parent
sys.path.insert(0, str(R))
from source_layout.module_layout import module_layout, execute_ordered_module
from source_layout.test_module_layout import SOURCE
from forehead_cache_manifest import forehead_cache_source_manifest as create_source_manifest_for_fit

class FactoryTests(unittest.TestCase):

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='factory-package-', dir=Path.cwd())
        self.root = Path(self.temp.name)
        self.files, self.record = module_layout(SOURCE, 'face.py')
        for name, text in self.files.items():
            (self.root / name).write_bytes(text.encode())
        self.manifest = {name: hashlib.sha256(text.encode()).hexdigest() for name, text in self.files.items()}
        self.write_manifest()
        self.frame = types.ModuleType('unregistered_package_frame')
        execute_ordered_module(self.root / 'face.py', self.record, self.frame.__dict__)

    def tearDown(self):
        self.temp.cleanup()

    def write_manifest(self):
        (self.root / 'input-manifest.json').write_text(json.dumps(self.manifest))

    def factory(self):
        return create_source_manifest_for_fit(self.frame.__dict__, self.frame.ear_shell_boundary_edges)

    def test_actual_ordered_package_factory(self):
        m = self.factory()
        m.verify()
        self.assertTrue(m.owns(self.frame.ear_shell_boundary_edges))
        self.assertIn(str(self.root / 'input-manifest.json'), m.sources)

    def test_missing_package_manifest_fails_closed(self):
        (self.root / 'input-manifest.json').unlink()
        with self.assertRaises(ValueError):
            self.factory()

    def test_unlisted_owned_chunk_fails(self):
        del self.manifest['face_source_ear_membership.py']
        self.write_manifest()
        with self.assertRaises(ValueError):
            self.factory()

    def test_changed_source_bytes_fail(self):
        p = self.root / 'face_source_ear_membership.py'
        p.write_bytes(p.read_bytes() + b'\n')
        with self.assertRaises(ValueError):
            self.factory()

    def test_wrong_authoritative_digest_fails(self):
        self.manifest['face_source_ear_membership.py'] = '0' * 64
        self.write_manifest()
        with self.assertRaises(ValueError):
            self.factory()

    def test_manifest_traversal_key_fails(self):
        self.manifest['../escape.py'] = '0' * 64
        self.write_manifest()
        with self.assertRaises(ValueError):
            self.factory()

    def test_duplicate_manifest_key_fails(self):
        (self.root / 'input-manifest.json').write_text('{"face.py":"' + self.manifest['face.py'] + '","face.py":"' + '0' * 64 + '"}')
        with self.assertRaises(ValueError):
            self.factory()

    def test_symlinked_chunk_fails(self):
        p = self.root / 'face_source_ear_membership.py'
        q = self.root / 'other.py'
        q.write_bytes(p.read_bytes())
        p.unlink()
        p.symlink_to(q)
        with self.assertRaises(ValueError):
            self.factory()

    def test_runtime_code_replacement_fails(self):
        old = self.frame.ear_shell_boundary_edges
        old.__code__ = old.__code__.replace(co_consts=(None, 99))
        with self.assertRaises(ValueError):
            self.factory()

    def test_manifest_mutation_after_factory_fails(self):
        m = self.factory()
        p = self.root / 'input-manifest.json'
        p.write_bytes(p.read_bytes() + b'\n')
        with self.assertRaises(ValueError):
            m.verify()

    def test_relative_compile_filename_kept_exact(self):
        frame = {'__file__': str(self.root / 'face.py'), '__name__': 'relative_unregistered'}
        for row in self.record['stages']:
            p = self.root / row['path']
            name = os.path.relpath(p, Path.cwd())
            exec(compile(p.read_text(), name, 'exec'), frame)
        f = frame['ear_shell_boundary_edges']
        m = create_source_manifest_for_fit(frame, f)
        self.assertIn(f.__code__.co_filename, m.sources)
        self.assertFalse(Path(f.__code__.co_filename).is_absolute())
        m.verify()

    def test_unknown_compile_filename_fails(self):
        f = self.frame.ear_shell_boundary_edges
        f.__code__ = f.__code__.replace(co_filename='<unknown>')
        with self.assertRaises(ValueError):
            self.factory()

    def test_symlink_after_factory_fails_even_identical_bytes(self):
        m = self.factory()
        p = self.root / 'face_source_ear_membership.py'
        q = self.root / 'same.py'
        q.write_bytes(p.read_bytes())
        p.unlink()
        p.symlink_to(q)
        with self.assertRaises(ValueError):
            m.verify()

    def test_manifest_symlink_after_factory_fails(self):
        m = self.factory()
        p = self.root / 'input-manifest.json'
        q = self.root / 'same.json'
        q.write_bytes(p.read_bytes())
        p.unlink()
        p.symlink_to(q)
        with self.assertRaises(ValueError):
            m.verify()

    def test_missing_frame_file_fails(self):
        with self.assertRaises(ValueError):
            create_source_manifest_for_fit({}, self.frame.ear_shell_boundary_edges)

    def test_unmanifested_reachable_helper_fails(self):
        self.frame.surface = lambda value: value
        with self.assertRaises(ValueError):
            self.factory()

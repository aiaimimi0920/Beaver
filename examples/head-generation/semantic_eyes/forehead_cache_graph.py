"""PRIVATE native-portable reachable derivative graph. No optimizer entry point."""
from pathlib import Path as ForeheadCachePath
import builtins as forehead_cache_builtins, hashlib as forehead_cache_hashlib, json as forehead_cache_json, types as forehead_cache_types, sys as forehead_cache_sys
import numpy as forehead_cache_np
import scipy as forehead_cache_scipy
from scipy.sparse import issparse as forehead_cache_issparse
from forehead_cache_schema import FOREHEAD_CACHE_SCHEMAS, FOREHEAD_CACHE_IGNORED, FOREHEAD_CACHE_IGNORED_CLOSURES, FOREHEAD_CACHE_NAMES
from scipy import sparse as forehead_cache_sparse
FOREHEAD_CACHE_SPARSE_TYPES = (
    forehead_cache_sparse.bsr_matrix, forehead_cache_sparse.bsr_array,
    forehead_cache_sparse.coo_matrix, forehead_cache_sparse.coo_array,
    forehead_cache_sparse.csc_matrix, forehead_cache_sparse.csc_array,
    forehead_cache_sparse.csr_matrix, forehead_cache_sparse.csr_array,
    forehead_cache_sparse.dia_matrix, forehead_cache_sparse.dia_array,
    forehead_cache_sparse.dok_matrix, forehead_cache_sparse.dok_array,
    forehead_cache_sparse.lil_matrix, forehead_cache_sparse.lil_array,
)

def forehead_cache_code_digest(code):

    def stable(v):
        if isinstance(v, forehead_cache_types.CodeType):
            return {n: stable(getattr(v, n)) for n in ('co_code', 'co_consts', 'co_names', 'co_varnames', 'co_freevars', 'co_cellvars', 'co_filename', 'co_name', 'co_qualname', 'co_firstlineno', 'co_flags', 'co_argcount', 'co_posonlyargcount', 'co_kwonlyargcount', 'co_stacksize', 'co_linetable', 'co_exceptiontable')}
        if isinstance(v, bytes):
            return {'bytes': v.hex()}
        if isinstance(v, tuple):
            return [stable(x) for x in v]
        if isinstance(v, frozenset):
            return {'frozenset': sorted((stable(x) for x in v), key=repr)}
        if isinstance(v, complex):
            return {'complex': repr(v)}
        if v is Ellipsis:
            return {'ellipsis': True}
        return v
    return forehead_cache_hashlib.sha256(forehead_cache_json.dumps(stable(code), sort_keys=True, separators=(',', ':')).encode()).hexdigest()

def forehead_cache_code_tree(code):
    yield code
    for value in code.co_consts:
        if isinstance(value, forehead_cache_types.CodeType):
            yield from forehead_cache_code_tree(value)

class ForeheadCacheSourceManifest:
    """Exact compile filename -> source text and optional on-disk source.

    Caller supplies reviewed packaged bytes, never discovers ownership from
    module names. Diagnostic synthetic filenames require explicit source bytes.
    """
    __slots__ = ('sources', 'hashes', 'codes', '__original')

    def __setattr__(self, name, value):
        raise AttributeError('Source manifest bindings are immutable')

    def __delattr__(self, name):
        raise AttributeError('Source manifest bindings are immutable')

    def __init__(self, sources):
        records = {}
        for name, text in sources.items():
            if type(name) is not str or type(text) is not str:
                raise ValueError('Manifest requires exact filename/source strings')
            records[name] = (text, forehead_cache_hashlib.sha256(text.encode('utf-8')).hexdigest(), frozenset((forehead_cache_code_digest(c) for c in forehead_cache_code_tree(compile(text, name, 'exec')))))
        object.__setattr__(self, '_ForeheadCacheSourceManifest__original', forehead_cache_types.MappingProxyType(records))
        object.__setattr__(self, 'sources', {n: r[0] for n, r in records.items()})
        object.__setattr__(self, 'hashes', {n: r[1] for n, r in records.items()})
        object.__setattr__(self, 'codes', {n: set(r[2]) for n, r in records.items()})

    def _require_record(self, name):
        original = self.__original[name]
        if self.sources.get(name) != original[0] or self.hashes.get(name) != original[1] or self.codes.get(name) != original[2]:
            raise ValueError('Source manifest record changed: ' + name)
        return original

    def verify(self):
        expected = self.__original.keys()
        if any((values.keys() != expected for values in (self.sources, self.hashes, self.codes))):
            raise ValueError('Source manifest keyset changed')
        for name in expected:
            text, _, _ = self._require_record(name)
            if not name.startswith('<') and ForeheadCachePath(name).read_bytes() != text.encode('utf-8'):
                raise ValueError('Packaged source bytes changed: ' + name)
        return {name: record[1] for name, record in self.__original.items()}

    def owns(self, f):
        filename = f.__code__.co_filename
        if filename not in self.__original:
            return False
        _, _, allowed = self._require_record(filename)
        if forehead_cache_code_digest(f.__code__) not in allowed:
            raise ValueError('Owned runtime code differs from source manifest: ' + f.__qualname__)
        return True

class ForeheadCacheGraph:

    def __init__(self, manifest):
        self.manifest = manifest
        self.seen = {}

    def walk(self, value):
        if value is None or type(value) in (str, int, bool):
            return value
        if type(value) is float:
            return {'float64_bits': forehead_cache_np.asarray(value).tobytes().hex()}
        if type(value) is bytes:
            return {'bytes': value.hex()}
        if type(value) is object:
            return {'sentinel': id(value)}
        if isinstance(value, forehead_cache_np.generic):
            if type(value) is not forehead_cache_np.dtype(type(value)).type or value.dtype.metadata is not None:
                raise ValueError('Unreviewed scalar subclass or dtype metadata')
            if value.dtype.hasobject or value.dtype.fields is not None:
                raise ValueError('Unreviewed object or structured scalar dtype')
            return {'numpy_scalar_type': type(value).__module__ + '.' + type(value).__qualname__,
                    'dtype': value.dtype.str, 'bytes': value.tobytes().hex()}
        if id(value) in self.seen:
            return {'ref': self.seen[id(value)]}
        ident = len(self.seen)
        self.seen[id(value)] = ident
        base = {'node': ident, 'identity': id(value)}
        if isinstance(value, forehead_cache_np.ndarray):
            if type(value) is not forehead_cache_np.ndarray or value.dtype.metadata is not None:
                raise ValueError('Unreviewed array subclass or dtype metadata')
            return {**base, 'array': value}
        if forehead_cache_issparse(value):
            if type(value) not in FOREHEAD_CACHE_SPARSE_TYPES:
                raise ValueError('Unreviewed sparse subclass')
            c = value.tocsr(copy=True)
            return {**base, 'shape': list(value.shape), 'format': value.format, 'data': c.data, 'indices': c.indices, 'indptr': c.indptr}
        if isinstance(value, dict):
            if type(value) is not dict:
                raise ValueError('Unreviewed dictionary subclass')
            return {**base, 'map': [[self.walk(k), self.walk(v)] for k, v in sorted(value.items(), key=lambda item: repr(item[0]))]}
        if isinstance(value, (tuple, list, set, frozenset)):
            if type(value) not in (tuple, list, set, frozenset):
                raise ValueError('Unreviewed sequence or set subclass')
            items = sorted(value, key=repr) if isinstance(value, (set, frozenset)) else value
            return {**base, 'kind': type(value).__name__, 'items': [self.walk(v) for v in items]}
        if isinstance(value, forehead_cache_types.MethodType):
            return {**base, 'function': self.walk(value.__func__), 'self': self.walk(value.__self__)}
        if isinstance(value, forehead_cache_types.FunctionType) and self.manifest.owns(value):
            code = value.__code__
            names = set().union(*(set(c.co_names) for c in forehead_cache_code_tree(code)))
            cells = {}
            for name, cell in zip(code.co_freevars, value.__closure__ or ()):
                if (value.__qualname__, name) in FOREHEAD_CACHE_IGNORED_CLOSURES:
                    continue
                try:
                    cells[name] = self.walk(cell.cell_contents)
                except ValueError:
                    raise
            return {**base, 'function': value.__qualname__, 'code': forehead_cache_code_digest(code), 'defaults': self.walk(value.__defaults__), 'kwdefaults': self.walk(value.__kwdefaults__), 'cells': cells, 'globals': {n: self.walk(value.__globals__[n]) for n in sorted(names) if n in value.__globals__}, 'builtins': {n: self.walk(value.__builtins__[n]) for n in sorted(names) if n not in value.__globals__ and n in value.__builtins__}}
        if isinstance(value, type) and (value.__name__ in FOREHEAD_CACHE_SCHEMAS or any((isinstance(v, forehead_cache_types.FunctionType) and self.manifest.owns(v) for v in vars(value).values()))):
            methods = {}
            for n, v in sorted(vars(value).items()):
                if n in ('__module__', '__doc__', '__dict__', '__weakref__'):
                    continue
                if isinstance(v, (staticmethod, classmethod)):
                    v = v.__func__
                if isinstance(v, property):
                    methods[n] = [self.walk(f) for f in (v.fget, v.fset, v.fdel)]
                else:
                    methods[n] = self.walk(v)
            return {**base, 'class': value.__qualname__, 'bases': [self.walk(v) for v in value.__bases__], 'members': methods}
        if isinstance(value, forehead_cache_types.ModuleType):
            root = value.__name__.split('.')[0]
            if root in ('numpy', 'scipy') or root in forehead_cache_sys.stdlib_module_names:
                return {**base, 'module': value.__name__, 'version': getattr(value, '__version__', None)}
            raise ValueError('Unreviewed module boundary ' + value.__name__)
        module = getattr(value, '__module__', '') or ''
        if isinstance(value, (forehead_cache_types.FunctionType, forehead_cache_types.BuiltinFunctionType, type, forehead_cache_np.ufunc)):
            root = module.split('.')[0]
            if root in ('numpy', 'scipy', 'builtins') or root in forehead_cache_sys.stdlib_module_names or isinstance(value, forehead_cache_np.ufunc):
                return {**base, 'external': module + '.' + getattr(value, '__qualname__', getattr(value, '__name__', ''))}
            raise ValueError('Unmanifested callable ' + module + '.' + getattr(value, '__qualname__', ''))
        name = type(value).__name__
        if name not in FOREHEAD_CACHE_SCHEMAS or set(vars(value)) != set(FOREHEAD_CACHE_SCHEMAS[name].split()):
            raise ValueError('Unreviewed dependency schema ' + name)
        return {**base, 'class': self.walk(type(value)), 'state': {k: self.walk(v) for k, v in sorted(vars(value).items()) if k not in FOREHEAD_CACHE_IGNORED.get(name, set())}}

def forehead_cache_bind_epoch(state, scaled_derivative, manifest):
    for name in FOREHEAD_CACHE_NAMES:
        if name not in state:
            raise ValueError('Missing derivative dependency ' + name)

    def epoch():
        if (forehead_cache_np.__version__, forehead_cache_scipy.__version__) != ('2.3.5', '1.17.0'):
            raise ValueError('Unreviewed vendor version')
        graph = ForeheadCacheGraph(manifest)
        return {'source_manifest': manifest.verify(), 'numpy': forehead_cache_np.__version__, 'scipy': forehead_cache_scipy.__version__, 'python': forehead_cache_sys.version, 'derivative': graph.walk(scaled_derivative), 'state': {n: graph.walk(state[n]) for n in FOREHEAD_CACHE_NAMES}}
    return epoch

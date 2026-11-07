"""PRIVATE factory; production names need forehead_ prefix before flattening."""
from pathlib import Path as ForeheadCacheManifestPath
import hashlib as forehead_cache_manifest_hashlib, json as forehead_cache_manifest_json, sys as forehead_cache_manifest_sys, types as forehead_cache_manifest_types
from forehead_cache_graph import ForeheadCacheSourceManifest, ForeheadCacheGraph, forehead_cache_code_tree
from forehead_cache_schema import FOREHEAD_CACHE_SCHEMAS

def forehead_cache_safe_file(path):
    path = ForeheadCacheManifestPath(path)
    if '..' in path.parts:
        raise ValueError('Source path traversal')
    absolute = path.absolute()
    if any((p.is_symlink() for p in (absolute, *absolute.parents))):
        raise ValueError('Symlinked source path')
    if not absolute.is_file():
        raise ValueError('Missing source file ' + str(path))
    return absolute.resolve()

class ForeheadFitSourceManifest(ForeheadCacheSourceManifest):
    """Reviewed manifest plus frozen filesystem resolution/no-symlink checks."""
    __slots__ = ('__paths',)

    def __init__(self, sources):
        paths = {name: forehead_cache_safe_file(name) for name in sources}
        super().__init__(sources)
        object.__setattr__(self, '_ForeheadFitSourceManifest__paths', forehead_cache_manifest_types.MappingProxyType(paths))

    def verify(self):
        for name, path in self.__paths.items():
            if forehead_cache_safe_file(name) != path:
                raise ValueError('Source path resolution changed ' + name)
        return super().verify()

def forehead_cache_source_manifest(fit_globals, actualscaledderivative):
    """Use existing packaged input-manifest; explicit imported-source baseline.

    Exact relative code filenames are retained only when they resolve from cwd
    to the verified source root, as required by the frozen SourceManifest API.
    No cwd changes, filename rewriting, or synthetic-source fallback.
    """
    if type(fit_globals) is not dict or '__file__' not in fit_globals:
        raise ValueError('Fit globals require actual filesystem source frame')
    entry = forehead_cache_safe_file(fit_globals['__file__'])
    root = entry.parent
    manifest_path = root / 'input-manifest.json'
    packaged = manifest_path.exists()
    approved = None
    if packaged:
        forehead_cache_safe_file(manifest_path)

        def unique_pairs(pairs):
            result = {}
            for k, v in pairs:
                if k in result:
                    raise ValueError('Duplicate packaged manifest key')
                result[k] = v
            return result
        manifest_bytes = manifest_path.read_bytes()
        approved = forehead_cache_manifest_json.loads(manifest_bytes.decode('utf-8'), object_pairs_hook=unique_pairs)
        if type(approved) is not dict or entry.name not in approved:
            raise ValueError('Packaged entry absent from input manifest')
        for name, value in approved.items():
            if type(name) is not str or ForeheadCacheManifestPath(name).name != name or name in ('.', '..') or ('/' in name) or ('\\' in name):
                raise ValueError('Non-basename packaged manifest key')
            if type(value) is not str or len(value) != 64 or any((c not in '0123456789abcdef' for c in value)):
                raise ValueError('Invalid packaged manifest digest')
    elif entry.name != 'forehead_shared_fit.py' or root.name != 'semantic_eyes':
        raise ValueError('Missing authoritative input-manifest for packaged/unknown fit frame')
    sources = {}
    functions = []
    seen = set()
    if packaged:
        sources[str(manifest_path)] = manifest_bytes.decode('utf-8')

    def source_for(filename, required=False):
        if filename.startswith('<'):
            if required:
                raise ValueError('Synthetic/unmanifested fit code filename ' + filename)
            return False
        try:
            path = forehead_cache_safe_file(filename)
        except ValueError:
            if required:
                raise
            return False
        if path.parent != root:
            if required:
                raise ValueError('Fit-owned code outside verified source root ' + filename)
            return False
        raw = path.read_bytes()
        if approved is not None and approved.get(path.name) != forehead_cache_manifest_hashlib.sha256(raw).hexdigest():
            raise ValueError('Packaged source not bound to input manifest ' + path.name)
        sources[filename] = raw.decode('utf-8')
        return True
    source_for(str(entry), True)

    def walk(value, required=False):
        if id(value) in seen:
            return
        seen.add(id(value))
        if isinstance(value, forehead_cache_manifest_types.MethodType):
            walk(value.__func__, required)
            return
        if isinstance(value, forehead_cache_manifest_types.FunctionType):
            if not source_for(value.__code__.co_filename, required):
                return
            functions.append(value)
            for cell in value.__closure__ or ():
                try:
                    walk(cell.cell_contents)
                except ValueError:
                    raise
            walk(value.__defaults__)
            walk(value.__kwdefaults__)
            names = set().union(*(set(c.co_names) for c in forehead_cache_code_tree(value.__code__)))
            for name in names:
                if name in value.__globals__:
                    walk(value.__globals__[name])
        elif isinstance(value, type):
            members = tuple(vars(value).values())
            methods = [item.__func__ if isinstance(item, (staticmethod, classmethod)) else item for item in members]
            if not any((isinstance(item, forehead_cache_manifest_types.FunctionType) and (not item.__code__.co_filename.startswith('<')) and (ForeheadCacheManifestPath(item.__code__.co_filename).absolute().parent == root) for item in methods)):
                return
            for item in members:
                if isinstance(item, (staticmethod, classmethod)):
                    item = item.__func__
                if isinstance(item, forehead_cache_manifest_types.FunctionType):
                    walk(item)
                elif isinstance(item, property):
                    for method in (item.fget, item.fset, item.fdel):
                        walk(method)
            for base in value.__bases__:
                if base is not object:
                    walk(base)
        elif isinstance(value, dict):
            for item in value.values():
                if isinstance(item, (forehead_cache_manifest_types.FunctionType, forehead_cache_manifest_types.MethodType, type, tuple, list, dict)):
                    walk(item)
        elif isinstance(value, (tuple, list)):
            for item in value:
                if isinstance(item, (forehead_cache_manifest_types.FunctionType, forehead_cache_manifest_types.MethodType, type, tuple, list, dict)):
                    walk(item)
        elif type(value).__name__ in FOREHEAD_CACHE_SCHEMAS:
            walk(type(value))
            for item in vars(value).values():
                if isinstance(item, (forehead_cache_manifest_types.FunctionType, forehead_cache_manifest_types.MethodType, type)) or type(item).__name__ in FOREHEAD_CACHE_SCHEMAS:
                    walk(item)
    walk(actualscaledderivative, True)
    for value in fit_globals.values():
        if isinstance(value, (forehead_cache_manifest_types.FunctionType, type)):
            walk(value)
    if not packaged:
        for module in tuple(forehead_cache_manifest_sys.modules.values()):
            filename = getattr(module, '__file__', None)
            if not filename:
                continue
            try:
                local = forehead_cache_safe_file(filename).parent == root
            except ValueError:
                continue
            if local:
                for value in tuple(vars(module).values()):
                    if isinstance(value, (forehead_cache_manifest_types.FunctionType, type)):
                        walk(value)
    result = ForeheadFitSourceManifest(sources)
    result.verify()
    for function in functions:
        if not result.owns(function):
            raise ValueError('Unverified discovered project function')
    ForeheadCacheGraph(result).walk(actualscaledderivative)
    return result

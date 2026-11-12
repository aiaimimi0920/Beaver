"""Keep bounded source/triangulation rules separate from surface fairing."""
import ast
import builtins
import symtable
from .layout import digest


def surface_layout(source):
    tree=ast.parse(source)
    wanted=('polygon_diagonals','triangulate_valid_skin','verify_skin_triangles','sample_source_cage')
    if any(isinstance(n,ast.ImportFrom) and n.module=='__future__' for n in tree.body):
        raise ValueError('Future semantics require explicit migration')
    functions=[n for n in tree.body if isinstance(n,ast.FunctionDef)]
    selected=[n for n in functions if n.name in wanted]
    if tuple(n.name for n in selected)!=wanted:
        raise ValueError('Unexpected paired polygon rule boundary')
    start,end=selected[0].lineno-1,selected[-1].end_lineno
    nodes=[n for n in tree.body if start<n.lineno<=end]
    if nodes!=selected:raise ValueError('Unexpected shared module state inside rules')
    for node in selected:
        args=[*node.args.posonlyargs,*node.args.args,*node.args.kwonlyargs]
        args += [x for x in (node.args.vararg,node.args.kwarg) if x is not None]
        if node.decorator_list or node.returns is not None or any(a.annotation is not None for a in args):
            raise ValueError('Decorators or annotations require explicit migration')
        for default in [*node.args.defaults,*[x for x in node.args.kw_defaults if x is not None]]:
            try:ast.literal_eval(default)
            except (ValueError,TypeError):raise ValueError('Dynamic defaults require explicit migration')
        if any(isinstance(n,(ast.Global,ast.Nonlocal)) for n in ast.walk(node)):
            raise ValueError('Shared mutable scope requires explicit migration')
    lines=source.splitlines(keepends=True)
    before=''.join(lines[:start]);rules=''.join(lines[start:end]);after=''.join(lines[end:])
    allowed=set(dir(builtins))|set(wanted)
    forbidden={'__name__','__file__','__package__','__spec__','__loader__','globals','locals','eval','exec','compile','vars','__import__'}
    original_symbols=symtable.symtable(source,'ear_surface.py','exec')
    shadowed={x.get_name() for x in original_symbols.get_symbols() if x.is_assigned() or x.is_imported() or x.is_namespace()}-set(wanted)
    tables=[symtable.symtable(rules,'source_mesh_rules.py','exec')]
    while tables:
        table=tables.pop();tables.extend(table.get_children())
        for symbol in table.get_symbols():
            if symbol.is_referenced() and symbol.is_global() and (symbol.get_name() not in allowed or symbol.get_name() in forbidden or symbol.get_name() in shadowed):
                raise ValueError('Cross-boundary dependency: '+symbol.get_name())
    imports='from source_mesh_rules import ('+', '.join(wanted)+')\n'
    entry=before+imports+after
    compile(entry,'ear_surface.py','exec');compile(rules,'source_mesh_rules.py','exec')
    if before+rules+after!=source:raise AssertionError('Source reconstruction changed')
    record={'kind':'surface_rules','original_sha256':digest(source),'entry_sha256':digest(entry),
            'helper_sha256':digest(rules),'entry_cut_bytes':len(before.encode()),
            'entry_import_bytes':len(imports.encode()),'imported_functions':list(wanted)}
    return {'ear_surface.py':entry,'source_mesh_rules.py':rules},record

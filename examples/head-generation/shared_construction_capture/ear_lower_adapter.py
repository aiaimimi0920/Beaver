"""Private exact-source lower fairing instrument: retain math, remove ID lookup."""
import ast
import hashlib


def instrument_lower(source):
    module=ast.parse(source)
    nodes=[n for n in module.body if isinstance(n,ast.FunctionDef) and n.name=='fair_lower_ear_boundaries']
    if len(nodes)!=1:raise ValueError('Unique original lower fairing function required')
    node=nodes[0]
    function=ast.get_source_segment(source,node)
    start=function.index('    from mathutils.kdtree import KDTree\n')
    end=function.index('    uses={}\n')
    expected='''    from mathutils.kdtree import KDTree
    data.update()
    tree=KDTree(len(data.vertices))
    for v in data.vertices:tree.insert(v.co,v.index)
    tree.balance()
    for report in reports:
        ids=[]
        for p in report['lower_bridge_triangle_positions']:
            _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
            assert d<.000002,'Authored bridge vertex missing after welding'
            ids.append(i)
        assert len(set(ids))==3
        report['lower_bridge_vertex_ids']=ids
    locked=set()
    for report in reports:
        for p in report['root_positions']:
            _,i,d=tree.find(Vector((p[0],p[1],p[2]-.021)))
            assert d<.000002
            locked.add(i)
'''
    if function[start:end]!=expected:raise ValueError('Actual lower fairing identity lookup source changed')
    replace='''    data.update()
    locked=set()
    for report in reports:
        ids=list(report['_identity_bridge_triangle'])
        if len(ids)!=3 or len(set(ids))!=3:
            raise ValueError('Missing actual lower bridge face identity')
        report['lower_bridge_vertex_ids']=ids
        roots=report['_identity_root_indices']
        if len(roots)!=len(report['root_positions']):
            raise ValueError('Changed actual root station count')
        locked.update(roots)
'''
    function=function[:start]+replace+function[end:]
    old="        _,seed,d=tree.find(Vector((p[0],p[1],p[2]-.021)))\n        assert d<.000002 and len(neighbors.get(seed,[]))==2"
    new="        seed=report['_identity_lower']\n        assert len(neighbors.get(seed,[]))==2"
    if function.count(old)!=1:raise ValueError('Actual lower fairing seed source changed')
    function=function.replace(old,new)
    if 'tree.find' in function or 'KDTree' in function:raise ValueError('Unbound spatial identity remains')
    ast.parse(function)
    if function.replace(new,old,1).replace(replace,expected,1)!=ast.get_source_segment(source,node):
        raise ValueError('Lower fairing observer edits are not exactly reversible')
    return function,{'original_function_sha256':hashlib.sha256(ast.get_source_segment(source,node).encode()).hexdigest(),
                     'instrumented_function_sha256':hashlib.sha256(function.encode()).hexdigest(),
                     'scope':'identity lookup replacement only; original fair_chain/backtracking/displacement constraints retained',
                     'reverse_source_exact':True, 'native_verified':False,'visual_approved':False}

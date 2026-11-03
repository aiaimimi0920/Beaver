"""Read live BMesh non-geometric fields through the admitted native schema."""
from joint_binding import require


def values(element,domain,bm,inventory):
    name={'POINT':'verts','EDGE':'edges','FACE':'faces'}[domain]
    collection=getattr(bm,name).layers
    special={'.select_vert':lambda:bool(element.select),'.select_edge':lambda:bool(element.select),
             '.select_poly':lambda:bool(element.select),'.hide_vert':lambda:bool(element.hide),
             '.hide_edge':lambda:bool(element.hide),'.hide_poly':lambda:bool(element.hide),
             'sharp_face':lambda:not element.smooth,'sharp_edge':lambda:not element.smooth}
    kinds={'FLOAT':'float','INT':'int','BOOLEAN':'int','FLOAT_VECTOR':'float_vector','FLOAT_COLOR':'float_color'}
    result={}
    for a in inventory:
        if a['domain']!=domain:continue
        if a['name'] in special:
            result[a['name']]=special[a['name']]();continue
        require(a['type'] in kinds,'Unsupported cap field type '+a['type'])
        typed=getattr(collection,kinds[a['type']],None)
        require(typed is not None,'Unsupported BMesh domain field '+a['name'])
        layer=typed.get(a['name']);require(layer is not None,'Missing cap field '+a['name'])
        value=element[layer]
        result[a['name']]=bool(value) if a['type']=='BOOLEAN' else value if isinstance(value,(int,float)) else list(value)
    result['editor:select']=bool(element.select);result['editor:hide']=bool(element.hide)
    if domain=='FACE':result.update({'face:material':element.material_index,'face:smooth':element.smooth})
    if domain=='EDGE':result.update({'edge:seam':element.seam,'edge:smooth':element.smooth})
    if domain=='POINT':
        result['deforms']=[sorted((int(k),float(v))for k,v in element[layer].items())for layer in collection.deform.values()]
    return result

"""Observer-only triangle-pair→quad identity transport inside the live ear lease."""
from joint_binding import require,digest,oriented_cycle
from cap_fields import values


def directed_boundary(triangles):
    require(len(triangles)==2 and all(len(t)==3 and len(set(t))==3 for t in triangles),
            'Cap needs two distinct source triangles')
    edges=[(a,b)for t in triangles for a,b in zip(t,t[1:]+t[:1])]
    shared={tuple(sorted(e))for e in edges if edges.count(e[::-1])==1}
    require(len(shared)==1,'Cap source winding/shared-edge mismatch')
    boundary=[e for e in edges if e[::-1] not in edges]
    require(len(boundary)==4,'Cap boundary changed')
    out=[boundary[0][0]]
    for _ in range(3):
        choices=[b for a,b in boundary if a==out[-1] and b not in out]
        require(len(choices)==1,'Cap boundary is not a simple directed cycle')
        out.append(choices[0])
    require((out[-1],out[0]) in boundary,'Cap boundary does not close')
    return out


def validate_allocation(before,output,original_ids):
    boundary=directed_boundary(before)
    require(oriented_cycle(boundary)==oriented_cycle(output),'Allocated cap winding/boundary mismatch')
    require(len(output)==4 and len(set(output))==4,'Allocated cap is not a unique quad')
    moving=set(output)-set(original_ids)
    require(len(moving)==2 and len(set(output)&set(original_ids))==2,
            'Cap original/new POINT ownership mismatch')
    return boundary,moving


class CapObserver:
    def __init__(self,ledger,points,faces,inventory,samples):
        self.ledger=ledger;self.points=points;self.faces=faces
        self.inventory=inventory;self.samples=samples
        self.pending=None;self.receipts=[]

    def __call__(self,event,s):
        first,second=s['first'],s['second']
        require(first in self.faces and second in self.faces,'Cap source FACE has no stable identity')
        source_faces=[self.faces[first],self.faces[second]]
        triangles=[[self.points[v]for v in f.verts]for f in (first,second)]
        originals={self.points[v]for v in s['vertices'] if v.index in s['original_ids']}
        proposed=[self.points[s['vertices'][i]]for i in s['ids']]
        boundary,moving=validate_allocation(triangles,proposed,originals)
        if event=='before':
            require(self.pending is None,'Previous cap transaction unfinished')
            face_fields=[values(f,'FACE',s['bm'],self.inventory) for f in (first,second)]
            require(face_fields[0]==face_fields[1], 'Source cap categorical face boundary differs')
            shared=set(first.edges)&set(second.edges)
            require(len(shared)==1,'Cap source internal edge is not unique')
            common=next(iter(shared))
            require(not common.seam and common.smooth,'Protected seam/sharp edge cannot be removed')
            for name in ('crease_edge','FacialCreaseGroup'):
                kind='float' if name=='crease_edge' else 'int'
                layer=getattr(s['bm'].edges.layers,kind).get(name)
                require(layer is not None and common[layer]==0,'Protected source cap edge '+name)
            coords={self.points[v]:list(v.co)for v in s['vertices']}
            edge_fields={}
            for f in (first,second):
                for e in f.edges:
                    key=tuple(sorted(self.points[v]for v in e.verts))
                    edge_fields[key]=values(e,'EDGE',s['bm'],self.inventory)
            plan=self.ledger.donors(source_faces,boundary)
            self.pending={'source_faces':source_faces,'source_triangles':triangles,
                          'coords':coords,'boundary':boundary,'moving':moving,
                          'edge_fields':edge_fields,'plan':plan,'point_set':set(self.points.values()),
                          'point_fields':{self.points[v]:values(v,'POINT',s['bm'],self.inventory)for v in s['vertices']},
                          'source_face_fields':face_fields}
            return
        require(event=='allocated' and self.pending is not None,'Missing cap allocation begin')
        pending=self.pending
        require(source_faces==pending['source_faces'] and triangles==pending['source_triangles'],
                'Source pair changed during cap allocation')
        face=s['face'];output=[self.points[v]for v in face.verts]
        validate_allocation(triangles,output,originals)
        require(face not in self.faces,'Cap allocation reused a live FACE identity')
        require(set(self.points.values())==pending['point_set'] and set(s['bm'].verts)==set(s['vertices']),
                'Cap unexpectedly allocated or removed a POINT')
        fid='return-cap:face:'+digest(source_faces)
        require(fid not in self.faces.values(),'Duplicate cap allocation event identity')
        stored={self.points[v]:list(v.co)for v in s['vertices']}
        require({self.points[v]:values(v,'POINT',s['bm'],self.inventory)for v in s['vertices']}==pending['point_fields'],
                'Cap changed a non-geometric POINT field or deform')
        face_fields=values(face,'FACE',s['bm'],self.inventory)
        expected_face=dict(pending['source_face_fields'][0],**{'face:smooth':True,
                           'editor:select':False,'editor:hide':False})
        if '.select_poly' in expected_face:expected_face['.select_poly']=False
        if '.hide_poly' in expected_face:expected_face['.hide_poly']=False
        if 'sharp_face' in expected_face:expected_face['sharp_face']=False
        require(face_fields==expected_face,'Cap allocation changed categorical FACE fields')
        changed=[p for p in stored if stored[p]!=pending['coords'][p]]
        require(set(changed)<=pending['moving'],'Cap moved an original anchor or outside POINT')
        require(all(sum((a-b)**2 for a,b in zip(stored[p],pending['coords'][p]))<=.0005**2
                    for p in changed),'Actual cap movement exceeds original bound')
        for e in face.edges:
            key=tuple(sorted(self.points[v]for v in e.verts))
            require(key in pending['edge_fields'],'Cap invented an external boundary edge')
            now=values(e,'EDGE',s['bm'],self.inventory)
            require(now==pending['edge_fields'][key],'Cap changed categorical boundary edge')
        ss=self.samples(face,self.points,self.inventory)
        ids=self.ledger.allocate(fid,output,ss,[{p:1.}for p in output],pending['plan'])
        self.faces[face]=fid
        self.receipts.append({'source_faces':source_faces,'source_triangles':triangles,
            'new_face':fid,'output_cycle':output,'corner_birth_ids':ids,
            'moved_points':[{'id':p,'before':pending['coords'][p],'after':stored[p]}for p in changed],
            'original_anchor_ids':sorted(set(output)&originals),'point_births':0,
            'boundary_fields_preserved':True,'non_geometric_point_fields_and_deforms_exact':True,
            'face_fields_exact_except_original_smooth_and_new_face_editor_defaults':True,
            'source_face_fields':pending['source_face_fields'],'new_face_fields':face_fields,'source_seam_refusal_is_additional_admission':True})
        self.pending=None

    def finish(self):
        require(self.pending is None and self.receipts,'Missing or incomplete return-cap receipt')
        return self.receipts

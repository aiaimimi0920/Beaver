"""Private immutable directed CORNER identities and explicit allocation stencils.
No geometry matching, global indices, Blender imports, or ordinal fallback.
"""
import copy
import struct
from joint_binding import digest, require, oriented_cycle


def corner_id(face_id, point, next_point):
    return 'corner:'+digest({'face':face_id,'point':point,'next':next_point})


def f32(value):
    return struct.unpack('<f',struct.pack('<f',float(value)))[0]


def vector(value):
    return list(value) if isinstance(value,(tuple,list)) else [value]


class CornerLedger:
    def __init__(self):
        self.faces={}
        self.corners={}
        self.births={}

    def capture(self, face_id, points, samples, layer_storage):
        """Capture while source loops are alive, using existing FACE/POINT IDs.
        Samples are ordered {point, values:{layer: scalar/vector}, categorical}.
        UV/color layer namespace and storage must be explicit in the caller.
        """
        require(layer_storage and all(v=='float32' for v in layer_storage.values()),
                'Unsupported corner storage; BYTE_COLOR needs an explicit quantization contract')
        require(isinstance(face_id,str) and face_id, 'Missing producer FACE identity')
        require(len(points)>=3 and len(set(points))==len(points), 'Invalid face cycle')
        require(all(isinstance(p,str) and p for p in points), 'Missing producer POINT identity')
        require([s['point'] for s in samples]==list(points), 'Source loop order mismatch')
        ids=[];rows={}
        for i,s in enumerate(samples):
            require(isinstance(s['values'],dict) and isinstance(s['categorical'],dict),
                    'Corner fields missing')
            require(set(s['values'])==set(layer_storage), 'Storage field inventory mismatch')
            cid=corner_id(face_id,points[i],points[(i+1)%len(points)])
            row={'id':cid,'face':face_id,'point':points[i],
                 'next_point':points[(i+1)%len(points)],
                 'values':copy.deepcopy(s['values']),
                 'categorical':copy.deepcopy(s['categorical']),
                 'layer_storage':dict(layer_storage)}
            digest(row)  # Reject non-finite/unserializable field values.
            ids.append(cid);rows[cid]=row
        signature={'cycle':list(oriented_cycle(points)),
                   'corners':{cid:rows[cid] for cid in sorted(rows)}}
        if face_id in self.faces:
            require(self.faces[face_id]==signature,
                    'Existing FACE winding, identity, or corner fields changed')
        else:
            self.faces[face_id]=signature
            self.corners.update(rows)
        return ids

    def donors(self, ordered_faces, parent_points, tolerance=1e-7):
        """Mirror ear_loop_values' explicit first-source-face traversal.
        Capture that function's actual face order, not current mesh enumeration.
        Same-point seam alternatives are recorded, not confused with the chosen
        donor; distinct UV/color values trigger the original unsupported seam gate.
        """
        require(len(ordered_faces)==len(set(ordered_faces)), 'Duplicate donor source cell')
        require(len(parent_points)==len(set(parent_points)), 'Duplicate parent POINT')
        require(tolerance==1e-7, 'Original seam tolerance cannot be relaxed')
        require(all(fid in self.faces for fid in ordered_faces), 'Uncaptured source cell')
        selected={};alternatives={}
        for fid in ordered_faces:
            for row in self.faces[fid]['corners'].values():
                p=row['point']
                require(p in parent_points, 'Source loop lacks an explicit local parent')
                if p not in selected:
                    selected[p]=row['id'];alternatives[p]=[]
                first=self.corners[selected[p]]
                require(first['values'].keys()==row['values'].keys(), 'Source layer schema changed')
                require(first['categorical']==row['categorical'], 'Categorical corner seam')
                for layer,v in row['values'].items():
                    a,b=vector(first['values'][layer]),vector(v)
                    require(len(a)==len(b) and max(abs(x-y) for x,y in zip(a,b))<=tolerance,
                            'Unsupported internal UV/color seam')
                alternatives[p].append(row['id'])
        require(set(selected)==set(parent_points), 'Parent has no captured source corner')
        return {'ordered_source_faces':list(ordered_faces),'chosen':selected,
                'alternatives':alternatives,'selection':'actual ear_loop_values first source face',
                'source_guard_tolerance':tolerance,'parent_points':list(parent_points)}

    def allocate(self, face_id, points, samples, point_stencils, donor_plan):
        """Verify observed float32 UV/color values before committing birth receipt.
        point_stencils aligns with directed output corners and names source POINT
        identities; convert each explicitly through the recorded donor plan.
        """
        require(face_id not in self.faces, 'New face must have a fresh allocation FACE ID')
        require(len(samples)==len(point_stencils)==len(points), 'Birth corner count mismatch')
        require(donor_plan==self.donors(donor_plan['ordered_source_faces'],
                                       donor_plan['parent_points'],donor_plan['source_guard_tolerance']),
                'Donor plan changed from captured source traversal')
        records=[]
        storage=None
        for sample,weights in zip(samples,point_stencils):
            require(weights and all(w>=0 for w in weights.values()) and
                    abs(sum(weights.values())-1)<1e-8, 'Invalid parent-corner stencil')
            require(set(weights)<=set(donor_plan['chosen']), 'Unknown parent-corner donor')
            parents={p:self.corners[donor_plan['chosen'][p]] for p in weights}
            require(all(row['point']==p for p,row in parents.items()), 'Donor POINT identity mismatch')
            first=parents[max(weights,key=weights.get)]
            require(all(row['layer_storage']==first['layer_storage'] for row in parents.values()),
                    'Parent storage mismatch')
            if storage is None: storage=first['layer_storage']
            require(storage==first['layer_storage'], 'Birth corner storage mismatch')
            require(sample['values'].keys()==first['values'].keys(), 'Birth layer schema mismatch')
            require(all(p['categorical']==first['categorical'] for p in parents.values()),
                    'Birth crosses categorical corner boundary')
            require(sample['categorical']==first['categorical'], 'Birth categorical fields changed')
            expected={}
            for layer,value in first['values'].items():
                require(all(layer in p['values'] and len(vector(p['values'][layer]))==len(vector(value))
                            for p in parents.values()), 'Parent layer dimensions mismatch')
                val=[f32(sum(w*vector(parents[p]['values'][layer])[k] for p,w in weights.items()))
                     for k in range(len(vector(value)))]
                expected[layer]=val if isinstance(value,(list,tuple)) else val[0]
                require(vector(sample['values'][layer])==val, 'Observed corner allocation differs from source stencil')
            records.append({'source_corners':{donor_plan['chosen'][p]:w for p,w in weights.items()},
                            'expected_float32_values':expected})
        # No partial identity commit before every corner field has passed.
        ids=self.capture(face_id,points,samples,storage)
        for cid,record in zip(ids,records):
            self.births[cid]={'id':cid,**record,'donor_plan':copy.deepcopy(donor_plan)}
        return ids

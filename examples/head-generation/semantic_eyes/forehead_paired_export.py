"""Authoritative float32 neutral/MouthOpen polygon admission, not a shape guard."""
import numpy as np
from source_polygon_validity import polygon_diagonals


def forehead_paired_export_report(points, delta, faces, face_ids):
    """Match exported float32 positions, fields and float32 vector addition."""
    neutral = np.asarray(points, dtype=np.float32)
    fields = np.asarray(delta, dtype=np.float32)
    if neutral.ndim != 2 or neutral.shape[1:] != (3,) or fields.shape != neutral.shape:
        raise ValueError('Invalid paired-export point/field arrays')
    if not np.isfinite(neutral).all() or not np.isfinite(fields).all():
        raise ValueError('Nonfinite paired-export point/field arrays')
    ids = list(face_ids)
    if (not ids or any(isinstance(i, (bool, np.bool_)) or not isinstance(i, (int, np.integer))
                       or not 0 <= i < len(faces) for i in ids) or len(set(ids)) != len(ids)):
        raise ValueError('Invalid affected paired-export face ownership')
    posed = (neutral + fields).astype(np.float32)
    if not np.isfinite(posed).all():
        raise ValueError('Nonfinite posed float32 coordinates')
    neutral, posed = neutral.astype(float), posed.astype(float)
    invalid = []
    for index in ids:
        face = list(faces[index])
        if (len(face) not in (3, 4) or len(set(face)) != len(face) or
                any(isinstance(i, (bool, np.bool_)) or not isinstance(i, (int, np.integer))
                    or not 0 <= i < len(neutral) for i in face)):
            raise ValueError('Invalid affected export polygon')
        # Convert scalars to Python float: polygon_diagonals then uses the same
        # double arithmetic on stored float32 values as native tuple(v.co).
        if not polygon_diagonals(neutral, face, posed):
            invalid.append(int(index))
    return {'passed': not invalid, 'invalid_faces': invalid, 'checked_faces': len(ids),
            'scope': 'Common actual neutral/MouthOpen export diagonal; not dense shape or all poses'}


def forehead_export_skin_face_ids(proposal, author):
    """Use actual native region inheritance, including untouched skin polygons."""
    regions = author['regions']
    owners = proposal['topology']['face_owners']
    if len(owners) != len(proposal['faces']) or len(regions) != len(author['faces']):
        raise ValueError('Invalid export-region ownership')
    if any(isinstance(i, (bool, np.bool_)) or not isinstance(i, (int, np.integer)) or not 0 <= i < len(regions) for i in owners):
        raise ValueError('Invalid original export face owner')
    # Exactly triangulate_valid_skin's region1/2 triangle/quad scope. Existing
    # ngons remain explicitly unverified by that exporter, not silently certified.
    return [i for i, owner in enumerate(owners) if regions[owner] in (1, 2)]


def make_forehead_paired_export_guard(proposal, author=None):
    return ForeheadPairedExportGuard(proposal, author)


class ForeheadPairedExportGuard:
    def __init__(self, proposal, author=None):
        self.faces = [list(q) for q in proposal['faces']]
        self.face_ids = list(proposal['affected_new']) if author is None else forehead_export_skin_face_ids(proposal, author)
        self.fields = np.asarray(proposal['open_delta'], dtype=np.float32).copy()
        self.require(proposal['vertices'], self.fields)

    def require(self, points, fields):
        report = forehead_paired_export_report(points, fields, self.faces, self.face_ids)
        if not report['passed']:
            raise ValueError('Invalid paired-export affected polygons: ' + str(report['invalid_faces']))
        return report

    def accepts(self, points, variable_ids, sampled_fields):
        fields = self.fields.copy()
        ids = np.asarray(variable_ids)
        values = np.asarray(sampled_fields, dtype=np.float32)
        if ids.ndim != 1 or ids.dtype.kind not in 'iu' or len(set(ids.tolist())) != len(ids) or np.any(ids < 0) or np.any(ids >= len(fields)) or values.shape != (len(ids), 3):
            raise ValueError('Invalid paired-export current-field replacement')
        fields[ids] = values
        return forehead_paired_export_report(points, fields, self.faces, self.face_ids)['passed']

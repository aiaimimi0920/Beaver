"""Optional fixed eligible-diagonal, local continuous export guidance.

This is a conservative optimization chart, not an alternative export gate.
The authoritative float32 polygon_diagonals check remains mandatory.
"""
import numpy as np
from source_polygon_validity import polygon_diagonals
from forehead_paired_export import forehead_export_skin_face_ids, make_forehead_paired_export_guard

def forehead_require_paired_input(proposal, author):
    """Reject an invalid actual export input before constructing any fit stages."""
    return make_forehead_paired_export_guard(proposal, author)

class ForeheadPairedDiagonalStage:

    def __init__(self, proposal, author, variable_ids):
        self.export_guard = make_forehead_paired_export_guard(proposal, author)
        self.fields = np.asarray(proposal['open_delta'], float).copy()
        self.ids = np.asarray(variable_ids, int)
        faces = proposal['faces']
        moving = set(variable_ids)
        self.face_ids = [i for i in forehead_export_skin_face_ids(proposal, author) if moving.intersection(faces[i])]
        if not self.face_ids:
            raise ValueError('No movable export skin quads for paired guidance')
        if any((len(faces[i]) != 4 for i in self.face_ids)):
            raise ValueError('Continuous paired guidance currently requires movable skin quads')
        self.quads = np.asarray([faces[i] for i in self.face_ids], int)
        v = np.asarray(proposal['vertices'], np.float32)
        posed = (v + self.fields.astype(np.float32)).astype(np.float32)
        # Fixed surrogate anchor only: never added to saved geometry or fields.
        self.pose_rounding_offset = posed.astype(float) - (v.astype(float) + self.fields)
        triangles = []
        for q in self.quads:
            options = polygon_diagonals(v.astype(float), q.tolist(), posed.astype(float))
            chosen = max(options, key=options.get)
            triangles.append(((0, 1, 2), (0, 2, 3)) if chosen == 0 else ((0, 1, 3), (1, 2, 3)))
        self.triangles = np.asarray(triangles, int)
        self.axes = []
        self.signs = []
        self.scales = []
        for p in (v.astype(float), posed.astype(float)):
            q = p[self.quads]
            area = np.cross(q, np.roll(q, -1, axis=1)).sum(axis=1)
            axes = np.argmax(abs(area), axis=1)
            sign = np.sign(area[np.arange(len(q)), axes]) * np.where(axes == 1, -1, 1)
            self.axes.append(axes)
            self.signs.append(sign)
            self.scales.append(np.max(np.linalg.norm(q[:, :, None] - q[:, None, :], axis=3), axis=(1, 2)) ** 2)
        self.count = 29 * len(self.quads)
        self.groups = [(29 * k, 29 * k + 29, q.tolist()) for k, q in enumerate(self.quads)]
        initial = self.raw(v.astype(float), self.fields[self.ids])
        if np.any(initial <= 0) or not np.isfinite(initial).all():
            raise ValueError('Continuous initial reference not strictly eligible')
        self.floor = np.minimum(initial * 0.5, 1e-07)

    def raw(self, points, sampled_fields):
        fields = self.fields.copy()
        fields[self.ids] = sampled_fields
        parts = []
        areas = []
        for pose, p in enumerate((
            np.asarray(points, float),
            np.asarray(points, float) + fields + self.pose_rounding_offset,
        )):
            q = p[self.quads]
            edge = np.roll(q, -1, axis=1) - q
            length = np.linalg.norm(edge, axis=2)
            scale = np.max(np.linalg.norm(q[:, :, None] - q[:, None, :], axis=3), axis=(1, 2))
            atol = np.maximum(2e-12, scale ** 2 * 1e-08)
            etol = np.maximum(1e-10, scale * 1e-08)
            area = np.cross(q, np.roll(q, -1, axis=1)).sum(axis=1)
            areas.append(area)
            an = np.linalg.norm(area, axis=1)
            tri = q[np.arange(len(q))[:, None, None], self.triangles]
            normal = np.cross(tri[:, :, 1] - tri[:, :, 0], tri[:, :, 2] - tri[:, :, 0])
            nn = np.linalg.norm(normal, axis=2)
            axis = self.axes[pose]
            project = (
                normal[np.arange(len(q))[:, None], np.arange(2)[None, :], axis[:, None]]
                * np.where(axis[:, None] == 1, -1, 1)
                * self.signs[pose][:, None]
            )
            ref = self.scales[pose]
            alignment = np.einsum('qti,qi->qt', normal, area)
            coherence = np.einsum('qi,qi->q', normal[:, 0], normal[:, 1])
            main = area[np.arange(len(q)), axis] * self.signs[pose] * np.where(axis == 1, -1, 1)
            other = np.asarray([[j for j in range(3) if j != int(ax)] for ax in axis])
            dominance = (main[:, None] - abs(area[np.arange(len(q))[:, None], other])) / ref[:, None]
            # Per pose: edges[4], area[1], triangle norms[2], winding[2],
            # normal/area alignment[2], coherence[1], axis dominance[2].
            # All residuals are dimensionless using fixed initial length² ref.
            rows = np.column_stack((
                (length - etol[:, None]) / np.sqrt(ref)[:, None],
                (an - atol) / ref,
                (nn - atol[:, None]) / ref[:, None],
                (project - atol[:, None]) / ref[:, None],
                alignment / ref[:, None] ** 2,
                coherence / ref ** 2,
                dominance,
            ))
            parts.append(rows)
        a, b = areas
        # Row28: cross-pose area alignment, normalized by fixed area².
        pose_dot = (
            np.einsum('qi,qi->q', a, b)
            - 0.1 * np.linalg.norm(a, axis=1) * np.linalg.norm(b, axis=1)
        ) / (self.scales[0] * self.scales[1])
        return np.column_stack((parts[0], parts[1], pose_dot)).ravel()

    def constraints(self, points, sampled_fields):
        return self.raw(points, sampled_fields) - self.floor

class ForeheadInactivePairedStage:
    groups = ()
    count = 0

    def constraints(self, points, sampled_fields):
        return np.empty(0)

def forehead_install_paired_stage(proposal, author, ids, layout, enabled=False):
    """Retain mandatory export admission; optionally add conservative local rows.

    Frozen axes/diagonal are per-fit choices only. Ties or a nonpositive initial
    continuous reference fail closed, not a global geometry-infeasibility proof.
    """
    if type(enabled) is not bool:
        raise ValueError('Invalid optional paired diagonal guidance')
    stage = ForeheadPairedDiagonalStage(proposal, author, ids) if enabled else ForeheadInactivePairedStage()
    guard = stage.export_guard if enabled else make_forehead_paired_export_guard(proposal, author)
    offset = layout['count']
    layout['paired_diagonal'] = (offset, offset + stage.count)
    layout['count'] += stage.count
    return (guard, stage)

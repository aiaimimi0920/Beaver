"""Pure source/current affected-domain closure; no geometry or baseline changes."""
import copy
import numbers
import numpy as np


def forehead_close_affected_domain(
    proposal, source_vertices, source_faces, *, allowed_source_owners=None
):
    """Return a copied proposal with conservative original/current face masks.

    Full original faces are mandatory: source sampler faces are not an indexed
    original-owner table. Every moved vertex is compared with immutable birth
    geometry, and every moved original additionally with original source geometry.
    Including a face does not recursively mark all its vertices moved.
    """
    if source_faces is None:
        raise ValueError("Full original source-face table required for affected closure")
    p = copy.deepcopy(proposal)
    source = np.asarray(source_vertices, dtype=float)
    current = np.asarray(p["vertices"], dtype=float)
    frozen = np.asarray(p["topology"]["vertices"], dtype=float)
    original_count = len(source)
    if (source.ndim != 2 or source.shape[1] != 3 or current.shape != frozen.shape
            or len(current) < original_count or not np.isfinite(source).all()
            or not np.isfinite(current).all() or not np.isfinite(frozen).all()):
        raise ValueError("Invalid affected-domain geometry")
    if not np.array_equal(frozen[:original_count], source):
        raise ValueError("Original frozen source geometry differs")
    faces = p["faces"]
    owners = p["topology"]["face_owners"]
    if len(owners) != len(faces):
        raise ValueError("Missing current-face source ownership")

    def index(value, count):
        if isinstance(value, bool) or not isinstance(value, numbers.Integral):
            raise ValueError("Noninteger affected-domain ownership")
        value = int(value)
        if not 0 <= value < count:
            raise ValueError("Invalid affected-domain ownership index")
        return value

    owners = [index(owner, len(source_faces)) for owner in owners]
    source_rows = [
        [index(vertex, original_count) for vertex in face] for face in source_faces
    ]
    current_rows = [
        [index(vertex, len(current)) for vertex in face] for face in faces
    ]
    original = {index(i, len(source_rows)) for i in p["affected_old"]}
    affected = {index(i, len(faces)) for i in p["affected_new"]}
    before_original = set(original)
    before_current = set(affected)
    changed_original = set(np.flatnonzero(
        np.any(current[:original_count] != source, axis=1)
    ).tolist())
    changed = set(np.flatnonzero(np.any(current != frozen, axis=1)).tolist())
    changed.update(changed_original)
    original.update(
        i for i, face in enumerate(source_rows) if changed_original.intersection(face)
    )
    affected.update(
        i for i, face in enumerate(current_rows) if changed.intersection(face)
    )
    original.update(owners[i] for i in affected)
    affected.update(i for i, owner in enumerate(owners) if owner in original)
    allowed = None if allowed_source_owners is None else set(allowed_source_owners)
    for owner in original:
        face = source_rows[owner]
        if (len(face) != 4 or len(set(face)) != 4
                or np.min(source[face, 0]) < -1e-8
                or (allowed is not None and owner not in allowed)):
            raise ValueError("Unsupported/nonpositive source-owner face in affected closure")
    for i in affected:
        face = current_rows[i]
        if (len(face) != 4 or len(set(face)) != 4
                or np.min(current[face, 0]) < -1e-8):
            raise ValueError("Unsupported/nonpositive current face in affected closure")
    p["affected_old"] = sorted(original)
    p["affected_new"] = sorted(affected)
    audit = {
        "scope": "Domain coverage only; expanded independent 1mm checks remain mandatory",
        "added_original_faces": sorted(original - before_original),
        "added_current_faces": sorted(affected - before_current),
        "changed_original_vertices": sorted(changed_original),
        "changed_birth_or_original_vertices": sorted(changed),
        "original_face_count": len(original),
        "current_face_count": len(affected),
        "all_current_owners_covered": all(owners[i] in original for i in affected),
        "all_owned_current_children_covered": all(
            i in affected for i, owner in enumerate(owners) if owner in original
        ),
    }
    p["affected_domain_closure"] = audit
    return p, audit

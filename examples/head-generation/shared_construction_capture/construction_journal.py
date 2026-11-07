"""Opt-in journal emitted by real allocation sites, before native mesh creation."""
from copy import deepcopy
from construction_topology import cycle, digest, role_segments, stable_id, topology
from construction_roles import outer_correspondence
from construction_boundaries import boundary_facts


class ConstructionJournal:
    def __init__(self, recipe, levels, lateral, *, calibration=None, style=None):
        self.recipe = {**deepcopy(recipe), "calibration_sha256": digest(calibration), "style_sha256": digest(style)}
        self.namespace = digest({"recipe": self.recipe, "levels": list(levels), "lateral": list(lateral)})
        self.points, self.roles, self.events, self.faces = [], {}, [], {}
        self.face_registry, self.seam, self.stage_keys = {}, set(), set()
        self.precompact = None
        self.outer_roles = []
        self.phase = "constructing"

    def _require_constructing(self):
        if self.phase != "constructing":
            raise ValueError("Constructor journal is sealed for compaction")

    def allocate(self, key, slot, position, provenance, seam=False):
        self._require_constructing()
        if len(position) != 3 or any(type(v) not in (int, float) for v in position):
            raise ValueError("Malformed constructor point values")
        digest(position)
        if type(slot) is not int or slot != len(self.points):
            raise ValueError("Constructor allocation is not the actual next appended slot")
        pid = stable_id(self.namespace, "point", key)
        if any(row["point_id"] == pid for row in self.points):
            raise ValueError("Reused constructor allocation identity")
        self.points.append({"point_id": pid, "allocation_slot": slot, "allocation_key": list(key),
                            "initial_position": list(position), "provenance": deepcopy(provenance),
                            "field_admission": "pending_independent_raw_author_birth_authority"})
        if seam:
            self.seam.add(slot)

    def allocate_batch(self, key, slots, vertices, provenance, seam_slots=()):
        for ordinal, slot in enumerate(slots):
            self.allocate([*key, ordinal], slot, vertices[slot], provenance, slot in seam_slots)

    def role(self, key, slots, *, kind, closed=False, replace=False):
        self._require_constructing()
        name = "/".join(str(item) for item in key)
        slots = list(slots)
        if (len(slots) < 2 or len(slots) != len(set(slots))
                or any(type(i) is not int or not 0 <= i < len(self.points) for i in slots)):
            raise ValueError("Invalid explicitly emitted constructor role")
        if name in self.roles and not replace:
            if self.roles[name] == {"slots": slots, "kind": kind, "closed": bool(closed)}:
                return
            raise ValueError("Constructor role changed without explicit replacement")
        self.roles[name] = {"slots": slots, "kind": kind, "closed": bool(closed)}

    def checkpoint(self, key, vertices, faces, *, preserve_boundary=False):
        self._require_constructing()
        if len(vertices) != len(self.points) or key in self.stage_keys:
            raise ValueError("Unreported constructor allocation or duplicate stage")
        topology(faces, len(vertices))
        before = self.faces
        current = {}
        registry = deepcopy(self.face_registry)
        prior_cycles = {cycle(row): fid for fid, row in before.items()}
        for ordinal, face in enumerate(faces):
            ids = [self.points[i]["point_id"] for i in face]
            fid = prior_cycles.get(cycle(ids))
            if fid is None:
                fid = stable_id(self.namespace, "face", [key, "emitted_face", ordinal])
                if fid in self.face_registry:
                    raise ValueError("Reused retired constructor face identity")
                registry[fid] = {"allocation_key": [key, "emitted_face", ordinal], "point_ids": ids,
                    "corner_ids": [stable_id(self.namespace, "corner", [fid, pid]) for pid in ids]}
            current[fid] = ids
        if preserve_boundary:
            def boundary(rows):
                pairs = {(a, b) for face in rows.values() for a, b in zip(face, face[1:] + face[:1])}
                return {edge for edge in pairs if edge[::-1] not in pairs}
            if boundary(before) != boundary(current):
                raise ValueError("Constructor replacement changed original patch boundary")
        self.events.append({"stage": key, "before_faces_sha256": digest(before),
            "after_faces_sha256": digest(current), "geometry_sha256": digest(vertices),
            "added_face_ids": sorted(set(current) - set(before)), "removed_face_ids": sorted(set(before) - set(current)),
            "reported_allocations": len(self.points), "preserved_boundary": preserve_boundary})
        self.faces, self.face_registry, self.stage_keys = current, registry, self.stage_keys | {key}

    def chart_point(self, row, column, vertices, source_chart, mapped_chart):
        self.allocate(["chart", row, column], len(vertices) - 1, vertices[-1],
            {"producer": "feature_flow_chart", "source_chart": list(source_chart), "mapped_chart": list(mapped_chart)}, seam=(column == 0))

    def chart_rows(self, rows, columns):
        for row in range(rows):
            self.role(["chart_row", row], range(row * columns, (row + 1) * columns), kind="dense_chart_row")

    def chin_strip(self, vertices, row, columns, low, high, start):
        self.allocate_batch(["chin_front_strip", row], range(start, len(vertices)), vertices,
                            {"producer": "feature_chin_front_strip", "row": row})
        self.role(["chart_row", row], [row * columns + i for i in low], kind="sparse_chin_row", replace=True)
        self.role(["chart_row", row + 1], [(row + 1) * columns + i for i in high], kind="sparse_chin_row", replace=True)

    def patch(self, kind, rings, closed, vertices):
        for ring_index, slots in enumerate(rings[:-1]):
            self.allocate_batch(["feature_patch", kind, ring_index], slots, vertices,
                {"producer": "feature_patch", "kind": kind, "ring": ring_index},
                (slots[0], slots[-1]) if kind == "mouth" and not closed else ())
        for ring_index, slots in enumerate(rings):
            self.role([kind, "ring", ring_index], slots, kind="feature_interface", closed=closed)

    def outer(self, report, vertices):
        self._require_constructing()
        allocations = []
        for ordinal, row in enumerate(report["rows"]):
            slots = row["vertex_ids"]
            if len(slots) < 3 or slots[0] >= len(self.points) or slots[-1] >= len(self.points):
                raise ValueError("Outer row must explicitly retain its two original endpoints")
            for station, slot in enumerate(slots[1:-1], 1):
                allocations.append((slot, ["outer_row", ordinal, station],
                    {"producer": "outer_flow_arc_stations", "height": row["height"], "q": row["q"][station],
                     "endpoint_point_ids": [self.points[slots[i]]["point_id"] for i in (0, -1)]}))
        for side in ("lower_transition", "upper_transition"):
            for station, slot in enumerate(report[side]["vertex_ids"]):
                allocations.append((slot, ["outer_fan", side, station],
                    {"producer": "outer_flow_fan", "parameters": report[side]["parameters"][station]}))
        for slot, key, provenance in sorted(allocations):
            self.allocate(key, slot, vertices[slot], provenance)
        for ordinal, row in enumerate(report["rows"]):
            slots = row["vertex_ids"]
            self.role(["outer_row", ordinal], slots, kind="outer_constructor_row")
            self.outer_roles.append("outer_row/" + str(ordinal))
            for name, existing in list(self.roles.items()):
                if existing["kind"] not in ("dense_chart_row", "sparse_chin_row"):
                    continue
                path = existing["slots"]
                for i, (a, b) in enumerate(zip(path, path[1:])):
                    if [a, b] == [slots[0], slots[-1]]:
                        self.roles[name] = {**existing, "slots": path[:i] + list(slots) + path[i + 2:]}
                        break
        for side in ("lower_transition", "upper_transition"):
            self.role(["outer_fan", side], report[side]["vertex_ids"], kind="fan_station_path")

    def outer_stage(self, report, vertices, faces):
        self.outer(report, vertices)
        self.checkpoint("outer_flow", vertices, faces, preserve_boundary=True)

    def underside(self, rows, vertices):
        for ordinal, slots in enumerate(rows):
            fresh = [slot for slot in slots if slot >= len(self.points)]
            self.allocate_batch(["chin_underside", ordinal], fresh, vertices,
                                {"producer": "chin_underside_row", "row": ordinal}, (slots[0],))
            self.role(["underside_row", ordinal], slots, kind="underside_constructor_row")

    def side_strip(self, vertices, start, stations):
        self.allocate_batch(["chin_side_strip"], range(start, len(vertices)), vertices,
                            {"producer": "feature_chin_side_strip"})
        self.role(["chin_side_fan"], stations, kind="fan_station_path")

    def posterior(self, before_vertices, vertices, mapping):
        self._require_constructing()
        if any(type(old) is not int or not 0 <= old < len(before_vertices)
               or type(new) is not int or (new != old and new < len(before_vertices))
               for old, new in mapping.items()):
            raise ValueError("Invalid explicit posterior allocation parent")
        fresh = [new for new in mapping.values() if new >= len(before_vertices)]
        if len(fresh) != len(set(fresh)):
            raise ValueError("Ambiguous posterior allocation parent")
        reverse = {new: old for old, new in mapping.items() if new >= len(before_vertices)}
        if set(reverse) != set(range(len(before_vertices), len(vertices))):
            raise ValueError("Posterior-return constructor omitted an allocation parent")
        for slot in range(len(before_vertices), len(vertices)):
            parent = reverse[slot]
            self.allocate(["posterior_return", self.points[parent]["point_id"]], slot, vertices[slot],
                          {"producer": "add_posterior_return", "parent_point_id": self.points[parent]["point_id"]}, parent in self.seam)

    def before_compact(self, vertices, faces):
        self.checkpoint("final_half_before_compaction", vertices, faces)
        self.precompact = deepcopy((vertices, faces))
        self.phase = "sealed"

    def compact(self, vertices, faces, remap):
        if self.precompact is None or self.phase != "sealed":
            raise ValueError("Missing actual precompaction snapshot")
        old_vertices, old_faces = self.precompact
        used = {slot for face in old_faces for slot in face}
        if (set(remap) != used or set(remap.values()) != set(range(len(used)))
                or any(type(i) is not int for pair in remap.items() for i in pair)):
            raise ValueError("Actual compaction remap is not an exact live-point bijection")
        if len(vertices) != len(used) or any(digest(vertices[new]) != digest(old_vertices[old]) for old, new in remap.items()):
            raise ValueError("Compaction changed constructor point values")
        topology(faces, len(vertices))
        if [list(face) for face in faces] != [[remap[i] for i in face] for face in old_faces]:
            raise ValueError("Compaction changed constructor topology")
        edges, _ = topology(old_faces, len(old_vertices))
        roles = role_segments(self.roles, used, edges)
        point_ids = [None] * len(vertices)
        for old, new in remap.items():
            point_ids[new] = self.points[old]["point_id"]
        source_faces = []
        for fid, ids in self.faces.items():
            born = self.face_registry[fid]
            corners = dict(zip(born["point_ids"], born["corner_ids"]))
            source_faces.append({"face_id": fid, "point_ids": ids, "corner_ids": [corners[pid] for pid in ids]})
        result = {"schema": "CONSTRUCTION_HALF_CAPTURE_V1", "source_namespace": self.namespace, "recipe": self.recipe,
            "point_ids": point_ids, "faces": source_faces, "allocation_history": deepcopy(self.points),
            "face_history": deepcopy(self.face_registry), "events": deepcopy(self.events),
            "old_to_new_slots": [[old, remap[old]] for old in sorted(remap)],
            "dropped_point_ids": [self.points[i]["point_id"] for i in range(len(self.points)) if i not in used],
            "seam_point_ids": [self.points[i]["point_id"] for i in sorted(self.seam & used)],
            "construction_roles": {name: {**{key: value for key, value in row.items() if key != "slots"},
                "allocation_slots_before_compaction": row["slots"], "segments": [[self.points[i]["point_id"] for i in segment] for segment in row["segments"]]}
                                   for name, row in roles.items()},
            "geometry_sha256": digest(vertices), "solver_ready": False,
            "pending": ["native_mirror_join_and_surgery_lineage", "complete_semantic_joined_rail_handoffs",
                        "complete_cross_band_comparison_roles", "independent_raw_author_birth_fields"]}

        for kind in ("eye", "mouth"):
            row = result["construction_roles"].get(kind + "/ring/0")
            if row is not None and (not row["complete_declared_path"] or len(row["segments"]) != 1):
                raise ValueError("Constructor aperture path was removed/split without a handoff")
        apertures = {kind + "_aperture": {"point_ids": result["construction_roles"][kind + "/ring/0"]["segments"][0],
            "closed": result["construction_roles"][kind + "/ring/0"]["closed"]}
            for kind in ("eye", "mouth") if kind + "/ring/0" in result["construction_roles"]}
        result["construction_boundary_facts"] = boundary_facts([face["point_ids"] for face in source_faces], point_ids,
            declared_seam=result["seam_point_ids"], aperture_paths=apertures)
        result["constructor_correspondence"] = (outer_correspondence(result["construction_roles"], source_faces, self.outer_roles)
                                                if self.outer_roles else None)
        self.phase = "finished"
        return result

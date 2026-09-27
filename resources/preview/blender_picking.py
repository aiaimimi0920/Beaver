"""Immutable static mesh queries; box selection includes occluded geometry."""
from mathutils import Vector
from mathutils.geometry import intersect_ray_tri


class FrozenMeshes:
    def __init__(self, scene, camera, projection):
        self.matrix = projection @ camera.matrix_world.inverted()
        self.inverse = self.matrix.inverted()
        self.meshes = []
        self.triangles = 0
        self.skipped = 0
        objects = {}
        def collect(layer):
            if layer.exclude or layer.collection.hide_render:
                return
            for obj in layer.collection.objects:
                objects[obj.name] = obj
            for child in layer.children:
                collect(child)
        for layer in scene.view_layers:
            if layer.use:
                collect(layer.layer_collection)
        for obj in sorted(objects.values(), key=lambda obj: obj.name):
            if obj.hide_render:
                continue
            if obj.type != "MESH":
                if obj.type in {"CURVE", "SURFACE", "META", "FONT", "VOLUME", "POINTCLOUD"} or obj.is_instancer:
                    self.skipped += 1
                continue
            if obj.modifiers or obj.data.shape_keys or obj.is_instancer:
                self.skipped += 1
                continue
            mesh = obj.data
            mesh.calc_loop_triangles()
            if self.triangles + len(mesh.loop_triangles) > 100000:
                self.skipped += 1
                continue
            faces = [tuple(obj.matrix_world @ mesh.vertices[i].co for i in tri.vertices)
                     for tri in mesh.loop_triangles]
            # JSON pointer escaping gives a stable, unambiguous object identity.
            name = obj.name.replace("~", "~0").replace("/", "~1")
            path = "/objects/" + name
            if len(path.encode("utf-8")) > 4096:
                self.skipped += 1
                continue
            self.meshes.append((path, faces))
            self.triangles += len(faces)

    def metadata(self):
        return {"capability": "frozen-static-mesh-ray",
                "triangles": self.triangles, "skipped": self.skipped}

    def unproject(self, point, depth):
        v = self.inverse @ Vector((2 * point["x"] - 1, 1 - 2 * point["y"], depth, 1))
        return v.to_3d() / v.w

    def pick(self, point, rectangle=None):
        result = self.metadata()
        result["hit"] = None
        if rectangle is not None:
            result.update(self.box(rectangle))
            return result
        start, end = self.unproject(point, -1), self.unproject(point, 1)
        direction = (end - start).normalized()
        limit = (end - start).length
        for path, faces in self.meshes:
            for index, (a, b, c) in enumerate(faces):
                hit = intersect_ray_tri(a, b, c, direction, start, True)
                if hit is None:
                    continue
                distance = (hit - start).dot(direction)
                if distance < 0 or distance > limit:
                    continue
                limit = distance
                normal = (b - a).cross(c - a).normalized()
                result["hit"] = {"nodePath": path, "triangle": index,
                                 "position": list(hit), "normal": list(normal),
                                 "distance": distance}
        return result

    def box(self, rect):
        left, right = 2 * rect["x"] - 1, 2 * (rect["x"] + rect["width"]) - 1
        top, bottom = 1 - 2 * rect["y"], 1 - 2 * (rect["y"] + rect["height"])
        planes = [Vector(p) for p in [(1, 0, 0, -left), (-1, 0, 0, right),
                  (0, 1, 0, -bottom), (0, -1, 0, top), (0, 0, 1, 1), (0, 0, -1, 1)]]
        paths, total, truncated = [], 0, False
        for path, faces in self.meshes:
            for face in faces:
                polygon = [self.matrix @ v.to_4d() for v in face]
                for plane in planes:
                    polygon = clip(polygon, plane)
                    if not polygon:
                        break
                if not polygon:
                    continue
                size = len(path.encode("utf-8"))
                if len(paths) == 32 or total + size > 32768:
                    truncated = True
                else:
                    paths.append(path)
                    total += size
                break
        return {"capability": "frozen-static-mesh-frustum", "nodePaths": paths,
                "truncated": truncated}


def clip(polygon, plane):
    if not polygon:
        return []
    result = []
    previous = polygon[-1]
    previous_distance = plane.dot(previous)
    for current in polygon:
        distance = plane.dot(current)
        if (distance >= 0) != (previous_distance >= 0):
            result.append(previous.lerp(current, previous_distance / (previous_distance - distance)))
        if distance >= 0:
            result.append(current)
        previous, previous_distance = current, distance
    return result

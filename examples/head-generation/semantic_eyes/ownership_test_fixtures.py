"""Small CPU fixtures; these do not stand in for native Blender acceptance."""

from types import SimpleNamespace
import numpy as np


class KDTree:
    def __init__(self, size):
        self.rows = []

    def insert(self, point, index):
        self.rows.append((np.array(point), index))

    def balance(self):
        pass

    def find_range(self, point, radius):
        return [(value, index, float(np.linalg.norm(value - point)))
                for value, index in self.rows
                if np.linalg.norm(value - point) <= radius]


class Matrix:
    def __init__(self, value=None):
        self.value = np.eye(4) if value is None else np.asarray(value, float)

    def inverted(self):
        return Matrix(np.linalg.inv(self.value))

    def __getitem__(self, index):
        return self.value[index]

    def __matmul__(self, other):
        if isinstance(other, Matrix):
            return Matrix(self.value @ other.value)
        return (self.value @ np.r_[np.asarray(other), 1.0])[:3]


class Attributes(dict):
    def __iter__(self):
        return iter(self.values())


class Group:
    def __init__(self, name, index, owner):
        self.name, self.index, self.owner = name, index, owner

    def add(self, ids, weight, mode):
        for index in ids:
            self.owner.data.vertices[index].groups.append(
                SimpleNamespace(group=self.index, weight=weight))

    def remove(self, ids):
        for index in ids:
            vertex = self.owner.data.vertices[index]
            vertex.groups = [item for item in vertex.groups if item.group != self.index]


class Groups(list):
    def get(self, name):
        return next((group for group in self if group.name == name), None)


class Object:
    def get(self, key):
        return getattr(self, key, None)


def add_attribute(owner, name, kind, domain, values, field):
    owner.data.attributes[name] = SimpleNamespace(
        name=name, data_type=kind, domain=domain,
        data=[SimpleNamespace(**{field: value}) for value in values])


def mesh_object(name, points, faces, regions, skin=False):
    owner = Object()
    owner.name, owner.skin_detail = name, skin
    owner.matrix_world, owner.vertex_groups = Matrix(), Groups()
    owner.data = SimpleNamespace(
        vertices=[SimpleNamespace(index=i, co=np.array(point, float), groups=[])
                  for i, point in enumerate(points)],
        polygons=[SimpleNamespace(index=i, vertices=face) for i, face in enumerate(faces)],
        attributes=Attributes(), update=lambda: None)
    color = (.985, .895, .865, 1) if skin else (.26, .065, .072, 1)
    add_attribute(owner, 'NormalRegion', 'INT', 'FACE', regions, 'value')
    add_attribute(owner, 'Color', 'FLOAT_COLOR', 'POINT', [color] * len(points), 'color')
    add_attribute(owner, 'MouthOpenDelta', 'FLOAT_VECTOR', 'POINT',
                  [(0, .002, -.008)] * len(points), 'vector')
    if skin:
        owner.vertex_groups.append(Group('Continuous facial shell', 0, owner))
        owner.vertex_groups[0].add(range(len(points)), 1.0, 'REPLACE')
    return owner


def boundary_pair():
    skin = mesh_object('Face shell editable half',
                       [(0, 0, 0), (1, 0, 0), (0, 1, 0)], [[0, 1, 2]], [1], True)
    oral = mesh_object('Oral cavity inner lips and back wall',
                       [(0, 0, 0), (1, 0, 0), (0, -1, 0)], [[0, 1, 2]], [10])
    add_attribute(oral, 'BlinkSection', 'INT', 'POINT', [0] * 3, 'value')
    return skin, oral

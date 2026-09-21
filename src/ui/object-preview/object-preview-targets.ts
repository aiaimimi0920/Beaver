import { objectById, type DemoObject } from "./mock-objects";
import {
  childPreview,
  mainPreview,
  type PreviewTarget,
} from "./preview-target";

export function referencePreview(
  object: DemoObject,
  reference: DemoObject["references"][number],
): PreviewTarget {
  const referenced = objectById(reference.objectId);
  return {
    key: `${object.id}:${object.version}:reference:${referenced.id}:${reference.version}`,
    object: referenced,
    version: reference.version,
    parentName: object.name,
    engine:
      referenced.objectType === "模型"
        ? "Blender"
        : referenced.objectType === "图像"
          ? "image"
          : "Godot",
  };
}

export function objectPreviewTargets(object: DemoObject) {
  return [
    { value: "", label: "整个对象", target: mainPreview(object) },
    ...object.components.map((part) => ({
      value: `component:${part.name}`,
      label: part.name,
      target: childPreview(object, part),
    })),
    ...object.references.map((reference) => ({
      value: `reference:${reference.objectId}`,
      label: objectById(reference.objectId).name,
      target: referencePreview(object, reference),
    })),
  ];
}

import type { Point } from "../../shared/asset-task";
import type { DemoObject } from "./mock-objects";

export type PreviewMode = "point" | "box";
export type PreviewEngine = "Godot" | "Blender" | "image";
export type PreviewSelection =
  | { kind: "whole" }
  | { kind: "point"; point: Point }
  | { kind: "box"; from: Point; to: Point };

export interface PreviewTarget {
  key: string;
  object: DemoObject;
  version: string;
  part?: DemoObject["components"][number];
  parentName?: string;
  engine: PreviewEngine;
}

export interface PreviewFeedback {
  target: PreviewTarget;
  selection: PreviewSelection;
}

export interface PreviewAnnotation {
  number: number;
  target: PreviewTarget;
  selection: Exclude<PreviewSelection, { kind: "whole" }>;
  prompt: string;
}

export function mainPreview(object: DemoObject): PreviewTarget {
  return {
    key: `${object.id}:${object.version}`,
    object,
    version: object.version,
    engine: "Godot",
  };
}

export function childPreview(
  object: DemoObject,
  part: DemoObject["components"][number],
): PreviewTarget {
  return {
    key: `${object.id}:${object.version}:component:${part.name}`,
    object,
    version: object.version,
    part,
    parentName: object.name,
    engine:
      part.objectType === "模型"
        ? "Blender"
        : part.objectType === "图像"
          ? "image"
          : "Godot",
  };
}

export function targetName(target: PreviewTarget): string {
  return target.part?.name ?? target.object.name;
}

export function engineName(engine: PreviewEngine): string {
  return engine === "image"
    ? "原图"
    : engine === "Blender"
      ? "Blender 制作视图"
      : "Godot 游戏效果";
}

export function selectionName(selection: PreviewSelection): string {
  if (selection.kind === "whole") return "整体内容";
  if (selection.kind === "point")
    return `画面点位 ${Math.round(selection.point[0] * 100)}%, ${Math.round(selection.point[1] * 100)}%`;
  return `框选区域 (${Math.round(selection.from[0] * 100)}%, ${Math.round(selection.from[1] * 100)}%) 至 (${Math.round(selection.to[0] * 100)}%, ${Math.round(selection.to[1] * 100)}%)`;
}

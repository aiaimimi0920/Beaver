import type { DemoObject } from "./mock-objects";
import { ManufactureEvidenceArt } from "./ManufactureEvidenceArt";
import { PreviewArtwork } from "./PreviewArtwork";
import { childPreview, type PreviewTarget } from "./preview-target";

export function manufactureViews(object: DemoObject) {
  return object.objectType === "模型"
    ? ["正面视图", "侧面视图", "背面视图"]
    : ["整体效果", "局部细节", "效果对照"];
}

export function manufactureTargets(
  object: DemoObject,
  version: string,
  stage: number,
  round: number,
) {
  const result = {
    ...object,
    version: `${version} · 阶段 ${stage + 1} · 第 ${round} 轮`,
  };
  return {
    evidence: manufactureViews(object).map((name, index): PreviewTarget => ({
      ...childPreview(result, { name, format: "PNG", objectType: "图像" }),
      key: `${result.id}:${result.version}:evidence:${index}`,
    })),
    output: object.components.map((part) => childPreview(result, part)),
  };
}

function evidenceView(target: PreviewTarget) {
  const match = /:evidence:([0-2])$/.exec(target.key);
  return match ? Number(match[1]) : undefined;
}

export function manufactureImageSize(
  target: PreviewTarget,
): readonly [number, number] {
  return evidenceView(target) !== undefined &&
    target.object.objectType === "模型"
    ? [240, 280]
    : [600, 420];
}

export function manufactureArtwork(target: PreviewTarget) {
  const view = evidenceView(target);
  return view === undefined ? (
    <PreviewArtwork target={target} />
  ) : (
    <ManufactureEvidenceArt object={target.object} view={view} />
  );
}

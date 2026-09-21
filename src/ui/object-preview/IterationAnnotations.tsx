import type { CSSProperties, ReactNode } from "react";
import { PreviewArtwork } from "./PreviewArtwork";
import { PreviewAnnotationMarks } from "./PreviewAnnotationMarks";
import {
  engineName,
  selectionName,
  targetName,
  type PreviewAnnotation,
  type PreviewTarget,
} from "./preview-target";

export function IterationAnnotations({
  annotations,
  edit,
  remove,
  gallery = false,
  renderArtwork,
  thumbnailStyle,
}: {
  annotations: PreviewAnnotation[];
  edit?: (number: number, prompt: string) => void;
  remove?: (number: number) => void;
  gallery?: boolean;
  renderArtwork?: (target: PreviewTarget) => ReactNode;
  thumbnailStyle?: (target: PreviewTarget) => CSSProperties;
}) {
  if (!annotations.length) return null;
  return (
    <div
      className={`op-annotation-list${gallery ? " is-gallery" : ""}`}
      aria-label="位置标注"
    >
      {annotations.map((annotation) => {
        const { number, target, selection, prompt } = annotation;
        const context = `${target.object.id} · ${targetName(target)} · ${target.version} · ${engineName(target.engine)} · ${selectionName(selection)}`;
        return (
          <div key={number} className="op-annotation-entry">
            <div className="op-annotation-context" title={context}>
              <span
                className="op-annotation-thumbnail"
                aria-label={`序号 ${number} 的演示预览`}
                style={thumbnailStyle?.(target)}
              >
                {renderArtwork ? (
                  renderArtwork(target)
                ) : (
                  <PreviewArtwork target={target} />
                )}
                <PreviewAnnotationMarks annotations={[annotation]} />
                <span className="op-annotation-number op-annotation-index">
                  {number}
                </span>
              </span>
              <span className="op-annotation-caption">
                <strong>
                  序号 {number} · {targetName(target)}
                </strong>
                <small>
                  {target.object.id} · {target.version} ·{" "}
                  {engineName(target.engine)}
                </small>
                <small>{selectionName(selection)}</small>
              </span>
              {remove && (
                <button
                  type="button"
                  className="op-annotation-remove"
                  aria-label={`删除序号 ${number}`}
                  title="删除标注，其他编号保持不变"
                  onClick={() => remove(number)}
                >
                  ×
                </button>
              )}
            </div>
            {edit || gallery ? (
              <textarea
                rows={2}
                aria-label={`序号 ${number} 的修改需求`}
                placeholder={
                  gallery
                    ? `序号 ${number} 的要求（可选）`
                    : `对序号 ${number} 做什么？可留空，在主需求中统一描述`
                }
                value={prompt}
                readOnly={!edit}
                onChange={(event) => edit?.(number, event.target.value)}
              />
            ) : (
              <p className="op-annotation-instruction">
                {prompt || "使用主需求中的描述"}
              </p>
            )}
          </div>
        );
      })}
      {!gallery && (
        <small className="op-annotation-note">
          UI 演示 · 保留画面坐标；截图与模型射线待接入。
        </small>
      )}
    </div>
  );
}

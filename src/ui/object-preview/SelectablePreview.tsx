import { useRef, useState, type PointerEvent, type ReactNode } from "react";
import type { Point } from "../../shared/asset-task";
import { imagePoint } from "../asset-task/preview-controls";
import { objectCardStyle } from "./object-categories";
import { PreviewArtwork } from "./PreviewArtwork";
import { PreviewAnnotationMarks } from "./PreviewAnnotationMarks";
import {
  engineName,
  targetName,
  type PreviewAnnotation,
  type PreviewFeedback,
  type PreviewMode,
  type PreviewSelection,
  type PreviewTarget,
} from "./preview-target";

export function SelectablePreview({
  target,
  mode,
  selected,
  selection,
  annotations = [],
  annotating = false,
  onSelect,
  open,
  className = "",
  renderArtwork,
  imageSize = [600, 420],
}: {
  target: PreviewTarget;
  mode: PreviewMode | null;
  selected: boolean;
  selection?: PreviewSelection;
  annotations?: PreviewAnnotation[];
  annotating?: boolean;
  onSelect: (feedback: PreviewFeedback) => void;
  open?: () => void;
  className?: string;
  renderArtwork?: (target: PreviewTarget) => ReactNode;
  imageSize?: readonly [number, number];
}) {
  const surface = useRef<HTMLSpanElement>(null);
  const drag = useRef<{ id: number; point: Point; client: Point } | null>(null);
  const [draft, setDraft] = useState<PreviewSelection>();
  const coordinates = (
    event: PointerEvent<HTMLButtonElement>,
    clamp = false,
  ): Point | null => {
    const rect = surface.current?.getBoundingClientRect();
    if (!rect) return null;
    const x = clamp
      ? Math.max(rect.left, Math.min(rect.right, event.clientX))
      : event.clientX;
    const y = clamp
      ? Math.max(rect.top, Math.min(rect.bottom, event.clientY))
      : event.clientY;
    return imagePoint(rect, imageSize[0], imageSize[1], x, y);
  };
  const box = (from: Point, to: Point): PreviewSelection => ({
    kind: "box",
    from: [Math.min(from[0], to[0]), Math.min(from[1], to[1])],
    to: [Math.max(from[0], to[0]), Math.max(from[1], to[1])],
  });
  const cancel = () => {
    drag.current = null;
    setDraft(undefined);
  };
  const targetAnnotations = annotations.filter(
    (value) => value.target.key === target.key,
  );
  const mark = mode
    ? (draft ??
      (selected && !annotating && !targetAnnotations.length
        ? selection
        : undefined))
    : undefined;
  return (
    <button
      type="button"
      className={`op-selectable-preview ${className}${selected ? " is-selected" : ""}`}
      style={objectCardStyle(
        target.part?.objectType ?? target.object.objectType,
      )}
      aria-label={
        target.parentName && !target.part
          ? `引用对象：${targetName(target)}`
          : targetName(target)
      }
      aria-pressed={selected}
      title={`${targetName(target)} · ${engineName(target.engine)}（演示画面，未连接引擎）`}
      data-preview-key={target.key}
      data-selection-mode={mode}
      data-annotating={annotating}
      onPointerDown={(event) => {
        if (!mode || event.button !== 0 || !event.isPrimary) return;
        const point = coordinates(event);
        if (!point) return;
        drag.current = {
          id: event.pointerId,
          point,
          client: [event.clientX, event.clientY],
        };
        event.currentTarget.setPointerCapture(event.pointerId);
        setDraft(undefined);
      }}
      onPointerMove={(event) => {
        if (drag.current?.id !== event.pointerId || mode !== "box") return;
        const point = coordinates(event, true);
        if (point) setDraft(box(drag.current.point, point));
      }}
      onPointerUp={(event) => {
        const origin = drag.current;
        if (!origin || origin.id !== event.pointerId) return;
        const point = coordinates(event, true);
        cancel();
        if (event.currentTarget.hasPointerCapture(event.pointerId))
          event.currentTarget.releasePointerCapture(event.pointerId);
        if (!mode || !point) return;
        const moved = Math.hypot(
          event.clientX - origin.client[0],
          event.clientY - origin.client[1],
        );
        if (mode === "point" && moved > 5) return;
        const region = box(origin.point, point);
        const validBox =
          region.kind === "box" &&
          region.to[0] - region.from[0] > 0.01 &&
          region.to[1] - region.from[1] > 0.01;
        if (annotating && mode === "box" && !validBox) return;
        onSelect({
          target,
          selection:
            mode === "box" && validBox ? region : { kind: "point", point },
        });
      }}
      onPointerCancel={cancel}
      onLostPointerCapture={cancel}
      onClick={(event) => {
        if (!mode || event.detail === 0)
          onSelect({ target, selection: { kind: "whole" } });
      }}
      onDoubleClick={() => {
        if (mode !== "box" && !annotating) open?.();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") cancel();
        if (
          event.key === "Enter" &&
          !annotating &&
          target.parentName &&
          !target.part &&
          open
        ) {
          event.preventDefault();
          open();
        }
      }}
    >
      <span
        className="op-selection-surface"
        ref={surface}
        style={{ aspectRatio: `${imageSize[0]} / ${imageSize[1]}` }}
      >
        {renderArtwork ? (
          renderArtwork(target)
        ) : (
          <PreviewArtwork target={target} />
        )}
        <PreviewAnnotationMarks annotations={targetAnnotations} />
        {mark?.kind === "box" && (
          <span
            className="op-preview-box"
            style={{
              left: `${mark.from[0] * 100}%`,
              top: `${mark.from[1] * 100}%`,
              width: `${(mark.to[0] - mark.from[0]) * 100}%`,
              height: `${(mark.to[1] - mark.from[1]) * 100}%`,
            }}
          />
        )}
        {mark?.kind === "point" && (
          <span
            className="op-preview-point"
            style={{
              left: `${mark.point[0] * 100}%`,
              top: `${mark.point[1] * 100}%`,
            }}
          />
        )}
      </span>
    </button>
  );
}

import type { PreviewAnnotation } from "./preview-target";
import "./preview-annotations.css";

export function PreviewAnnotationMarks({
  annotations,
}: {
  annotations: PreviewAnnotation[];
}) {
  return annotations.map(({ number, selection }) => {
    const point = selection.kind === "point" ? selection.point : selection.from;
    return (
      <span
        key={number}
        className={`op-annotation-mark is-${selection.kind}`}
        style={{
          left: `${point[0] * 100}%`,
          top: `${point[1] * 100}%`,
          ...(selection.kind === "box"
            ? {
                width: `${(selection.to[0] - selection.from[0]) * 100}%`,
                height: `${(selection.to[1] - selection.from[1]) * 100}%`,
              }
            : {}),
        }}
        aria-label={`序号 ${number}，${selection.kind === "point" ? "点选" : "框选"}`}
      >
        <span
          className="op-annotation-number"
          style={{
            left: point[0] > 0.9 ? "auto" : 0,
            right: point[0] > 0.9 ? 0 : "auto",
            top: point[1] > 0.9 ? "auto" : 0,
            bottom: point[1] > 0.9 ? 0 : "auto",
          }}
        >
          {number}
        </span>
      </span>
    );
  });
}

import { useId } from "react";
import type { AssetAnnotation, AssetPick } from "../../shared/asset-task";

export function AnnotationOverlay({
  width,
  height,
  annotations,
  pick,
}: {
  width: number;
  height: number;
  annotations: AssetAnnotation[];
  pick?: AssetPick | null;
}) {
  const arrowId = useId().replace(/:/g, "");
  return (
    <svg
      className="asset-annotation-overlay"
      viewBox={`0 0 ${width} ${height}`}
      aria-hidden="true"
    >
      <defs>
        <marker
          id={arrowId}
          markerWidth="8"
          markerHeight="8"
          refX="7"
          refY="4"
          orient="auto"
        >
          <path d="M0,0 L8,4 L0,8" fill="none" stroke="currentColor" />
        </marker>
      </defs>
      <g
        fill="none"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinejoin="round"
        strokeLinecap="round"
      >
        {annotations.map((mark, i) => {
          const [a, b] = mark.points;
          if (!a || !b) return null;
          return mark.kind === "box" ? (
            <rect
              key={i}
              x={Math.min(a[0], b[0]) * width}
              y={Math.min(a[1], b[1]) * height}
              width={Math.abs(a[0] - b[0]) * width}
              height={Math.abs(a[1] - b[1]) * height}
            />
          ) : (
            <polyline
              key={i}
              points={mark.points
                .map(([x, y]) => `${x * width},${y * height}`)
                .join(" ")}
              markerEnd={mark.kind === "arrow" ? `url(#${arrowId})` : undefined}
            />
          );
        })}
        {pick && (
          <g
            transform={`translate(${pick.point[0] * width},${pick.point[1] * height})`}
          >
            <circle r="10" />
            <path d="M-16,0 H16 M0,-16 V16" />
          </g>
        )}
      </g>
    </svg>
  );
}

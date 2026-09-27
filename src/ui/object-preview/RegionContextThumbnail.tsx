import { useId } from "react";
import type { ImageRegion } from "../../shared/object-rework-image";

// The caller owns decoding, source identity and the image URL lifetime.
export function RegionContextThumbnail({
  source,
  width,
  height,
  region,
  number,
}: {
  source: string;
  width: number;
  height: number;
  region: ImageRegion;
  number: number;
}) {
  const cropId = useId();
  const x = region.x * width;
  const y = region.y * height;
  const w = region.width * width;
  const h = region.height * height;
  const left = Math.max(0, x - Math.max(w / 4, width * 0.02));
  const top = Math.max(0, y - Math.max(h / 4, height * 0.02));
  const right = Math.min(width, x + w + Math.max(w / 4, width * 0.02));
  const bottom = Math.min(height, y + h + Math.max(h / 4, height * 0.02));
  return (
    <figure style={{ margin: "0.5rem 0" }}>
      <div style={{ display: "flex", flexWrap: "wrap", gap: "0.5rem" }}>
        {[
          { label: "原图位置", viewBox: `0 0 ${width} ${height}` },
          {
            label: "局部上下文",
            viewBox: `${left} ${top} ${right - left} ${bottom - top}`,
          },
        ].map(({ label, viewBox }) => (
          <svg
            key={label}
            role="img"
            aria-label={`区域 ${number} ${label}`}
            viewBox={viewBox}
            preserveAspectRatio="xMidYMid meet"
            width="160"
            height="100"
            style={{ maxWidth: "100%", background: "#202020", borderRadius: 4 }}
          >
            <title>{`区域 ${number} ${label}`}</title>
            <defs>
              <clipPath id={`${cropId}-${label}`}>
                <rect
                  x={label === "局部上下文" ? left : 0}
                  y={label === "局部上下文" ? top : 0}
                  width={label === "局部上下文" ? right - left : width}
                  height={label === "局部上下文" ? bottom - top : height}
                />
              </clipPath>
            </defs>
            <g clipPath={`url(#${cropId}-${label})`}>
              <image href={source} width={width} height={height} />
              <rect
                x={x}
                y={y}
                width={w}
                height={h}
                fill="none"
                stroke="#ffcc00"
                strokeWidth="2"
                vectorEffect="non-scaling-stroke"
              />
            </g>
          </svg>
        ))}
      </div>
      <figcaption>
        区域 {number}：原图位置 / 局部上下文（黄框内为选区）
      </figcaption>
    </figure>
  );
}

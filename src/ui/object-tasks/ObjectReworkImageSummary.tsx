import type { ObjectReworkImage } from "../../shared/object-rework-image";

export function ObjectReworkImageSummary({
  image,
}: {
  image?: ObjectReworkImage;
}) {
  if (!image) return null;
  return (
    <div aria-label="冻结图片区域反馈">
      <p>
        {image.path} · {image.width} × {image.height} · SHA-256: {image.sha256}
      </p>
      <ol>
        {image.regions.map((region, index) => (
          <li key={index}>
            区域 {index + 1}：左上 ({region.x.toFixed(4)}, {region.y.toFixed(4)}
            )， 宽 {region.width.toFixed(4)}，高 {region.height.toFixed(4)}。
            {region.prompt || "使用整体修改意见"}
          </li>
        ))}
      </ol>
    </div>
  );
}

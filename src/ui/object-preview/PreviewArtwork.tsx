import { Icon, type IconName } from "../Icon";
import { ObjectArt } from "./ObjectArt";
import type { PreviewTarget } from "./preview-target";

export function PreviewArtwork({ target }: { target: PreviewTarget }) {
  const { object, part } = target;
  if (!part) return <ObjectArt kind={object.kind} annotations={false} />;
  const image = part.format.includes("PNG");
  const normal = image && part.name.includes("法线");
  const model = /BLEND|GLB|TSCN/.test(part.format);
  const icon: IconName = /SCRIPT|SHADER/.test(part.format)
    ? "code"
    : part.format === "ANIMATION"
      ? "play"
      : "layers";
  return (
    <div className={`op-content-thumbnail${normal ? " is-normal" : ""}`}>
      {image && object.kind !== "texture" ? (
        <div
          className="op-content-texture"
          role="img"
          aria-label={`${part.name}演示色块，非实际生成产物`}
        />
      ) : image || model ? (
        <ObjectArt kind={image ? "texture" : object.kind} annotations={false} />
      ) : (
        <Icon name={icon} />
      )}
    </div>
  );
}

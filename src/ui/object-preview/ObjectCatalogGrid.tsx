import { Icon } from "../Icon";
import type { ObjectType } from "./object-categories";
import { objectCardStyle } from "./object-categories";
import type { ObjectCatalogRecord } from "./use-object-catalog";

const extensionTypes: Record<string, ObjectType> = {
  png: "图像",
  jpg: "图像",
  jpeg: "图像",
  wav: "音频",
  mp3: "音频",
  glb: "模型",
  tscn: "场景",
  gd: "脚本",
  po: "翻译",
  csv: "其他",
};

function typeOf(object: ObjectCatalogRecord): ObjectType {
  const kind = object.components[0]?.kind.toLowerCase();
  if (kind?.includes("audio")) return "音频";
  if (kind?.includes("scene")) return "场景";
  if (kind?.includes("model")) return "模型";
  if (kind?.includes("script")) return "脚本";
  const extension = object.files[0]?.path.split(".").pop()?.toLowerCase();
  return (extension && extensionTypes[extension]) || "其他";
}

export function ObjectCatalogGrid({
  objects,
  selected,
  filter,
  search,
  thumbnailSize,
  select,
}: {
  objects: ObjectCatalogRecord[];
  selected?: string;
  filter: ObjectType | "全部";
  search: string;
  thumbnailSize: number;
  select: (id: string) => void;
}) {
  const visible = objects.filter((object) => {
    const type = typeOf(object);
    return (
      (filter === "全部" || type === filter) &&
      `${object.name} ${object.id}`
        .toLowerCase()
        .includes(search.trim().toLowerCase())
    );
  });
  if (!visible.length) {
    return (
      <section className="op-empty" aria-live="polite">
        <Icon name="assets" />
        <h2>{objects.length ? "没有匹配的对象" : "项目中尚无登记对象"}</h2>
      </section>
    );
  }
  return (
    <div
      className="op-object-grid"
      style={{
        gridTemplateColumns: `repeat(auto-fill, minmax(0, min(${thumbnailSize}px, 100%)))`,
      }}
    >
      {visible.map((object) => {
        const type = typeOf(object);
        return (
          <button
            type="button"
            className={`op-object-card${selected === object.id ? " is-selected" : ""}`}
            style={objectCardStyle(type)}
            key={object.id}
            aria-pressed={selected === object.id}
            onClick={() => select(object.id)}
          >
            <div className="op-object-cover" aria-hidden="true">
              <Icon name="assets" />
            </div>
            <strong>{object.name}</strong>
            <small>
              {type} · {object.components.length} 项内容
            </small>
            <small>{object.id}</small>
          </button>
        );
      })}
    </div>
  );
}

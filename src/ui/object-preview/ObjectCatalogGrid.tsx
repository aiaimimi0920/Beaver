import { useState } from "react";
import { assetUrl } from "../api";
import { Icon } from "../Icon";
import type { ObjectType } from "./object-categories";
import { objectCardStyle } from "./object-categories";
import type { ObjectCatalogRecord } from "./use-object-catalog";
import { FrozenCatalogThumbnail } from "./FrozenCatalogThumbnail";
import { catalogSceneTarget } from "./object-thumbnail-cache";

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
  const category = object.category.trim();
  if (
    ["图像", "音频", "模型", "场景", "翻译", "脚本", "其他"].includes(category)
  ) {
    return category as ObjectType;
  }
  const kind = object.components[0]?.kind.toLowerCase();
  if (kind?.includes("audio")) return "音频";
  if (kind?.includes("scene")) return "场景";
  if (kind?.includes("model")) return "模型";
  if (kind?.includes("script")) return "脚本";
  const extension = object.files[0]?.path.split(".").pop()?.toLowerCase();
  return (extension && extensionTypes[extension]) || "其他";
}

function searchableText(object: ObjectCatalogRecord) {
  return [
    object.name,
    object.id,
    object.category,
    ...object.tags,
    ...object.components.flatMap((component) => [
      component.name,
      component.kind,
    ]),
    ...object.files.map((file) => file.path),
    object.thumbnailPath ?? "",
    object.parentObjectId ?? "",
  ]
    .join("\n")
    .toLocaleLowerCase();
}

function ObjectThumbnail({ object }: { object: ObjectCatalogRecord }) {
  const [unavailable, setUnavailable] = useState(false);
  const path = object.thumbnailPath;
  if (!path) {
    return (
      <div className="op-object-cover-placeholder">
        <Icon name="assets" />
        <span>未引用缩略图</span>
      </div>
    );
  }
  if (unavailable) {
    return (
      <div className="op-object-cover-placeholder" role="status">
        <Icon name="review" />
        <span>缩略图不可用</span>
        <code title={path}>{path}</code>
      </div>
    );
  }
  return (
    <div className="op-object-thumbnail">
      <img
        src={assetUrl(object.projectId, path, String(object.revision))}
        alt={`${object.name} 缩略图`}
        loading="lazy"
        onError={() => setUnavailable(true)}
      />
      <small title={path}>{path}</small>
    </div>
  );
}

export function ObjectCatalogGrid({
  objects,
  selected,
  filter,
  search,
  tags,
  thumbnailSize,
  select,
}: {
  objects: ObjectCatalogRecord[];
  selected?: string;
  filter: ObjectType | "全部";
  search: string;
  tags: string[];
  thumbnailSize: number;
  select: (id: string) => void;
}) {
  const selectedTags = tags.map((tag) => tag.toLocaleLowerCase());
  const visible = objects.filter((object) => {
    const type = typeOf(object);
    const categoryMatches = filter === "全部" || object.category === filter;
    const tagsMatch = selectedTags.every((tag) =>
      object.tags.some((candidate) => candidate.toLocaleLowerCase() === tag),
    );
    return (
      categoryMatches &&
      tagsMatch &&
      searchableText(object).includes(search.trim().toLocaleLowerCase())
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
            <div className="op-object-cover">
              {catalogSceneTarget(object) ? (
                <FrozenCatalogThumbnail
                  key={JSON.stringify(catalogSceneTarget(object))}
                  object={object}
                />
              ) : (
                <ObjectThumbnail key={object.thumbnailPath} object={object} />
              )}
            </div>
            <strong>{object.name}</strong>
            <small>
              {object.category} · {object.components.length} 项内容
            </small>
            <small>{object.id}</small>
          </button>
        );
      })}
    </div>
  );
}

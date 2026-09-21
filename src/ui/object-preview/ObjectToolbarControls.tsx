import { useEffect, useRef, type CSSProperties } from "react";
import { Icon } from "../Icon";
import { objectCategories, type ObjectType } from "./object-categories";
import type { PreviewMode } from "./preview-target";

export function ObjectToolbarControls({
  filter,
  setFilter,
  search,
  setSearch,
  tags,
  setTags,
  thumbnailSize,
  setThumbnailSize,
  mode,
  canAnnotate,
  setMode,
  inspect,
  availableTags,
  importObjects,
  generateObjects,
  unavailableReason,
}: {
  filter: ObjectType | "全部";
  setFilter: (filter: ObjectType | "全部") => void;
  search: string;
  setSearch: (search: string) => void;
  tags: string[];
  setTags: (tags: string[]) => void;
  thumbnailSize: number;
  setThumbnailSize: (size: number) => void;
  mode: PreviewMode | null;
  canAnnotate: boolean;
  setMode: (mode: PreviewMode) => void;
  inspect?: () => void;
  availableTags: string[];
  importObjects?: () => void;
  generateObjects?: () => void;
  unavailableReason?: string;
}) {
  const tagMenu = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const closeOutside = (event: PointerEvent) => {
      if (
        event.target instanceof Node &&
        !tagMenu.current?.contains(event.target)
      ) {
        tagMenu.current?.removeAttribute("open");
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && tagMenu.current?.open) {
        tagMenu.current.open = false;
        tagMenu.current.querySelector("summary")?.focus();
      }
    };
    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, []);
  return (
    <>
      <header className="op-object-toolbar" aria-label="对象工具栏">
        <div className="op-object-toolbar-left">
          <div className="op-object-types" role="group" aria-label="对象分类">
            {objectCategories.map((category) => (
              <button
                key={category.name}
                type="button"
                title={category.name}
                aria-label={category.name}
                aria-pressed={filter === category.name}
                style={{ "--category-color": category.color } as CSSProperties}
                onClick={() => setFilter(category.name)}
              >
                <Icon name={category.icon} />
              </button>
            ))}
          </div>
          <details
            className="op-tag-menu"
            data-filtered={tags.length > 0}
            ref={tagMenu}
            onBlur={(event) => {
              if (!event.currentTarget.contains(event.relatedTarget)) {
                event.currentTarget.open = false;
              }
            }}
          >
            <summary aria-label="标签筛选" title="按标签筛选">
              <Icon name="tag" />
              <Icon name="chevronDown" />
            </summary>
            <div className="op-tag-dropdown" role="group" aria-label="选择标签">
              {availableTags.length === 0 && <small>暂无可用标签</small>}
              {availableTags.map((tag) => (
                <label key={tag}>
                  <input
                    type="checkbox"
                    checked={tags.includes(tag)}
                    onChange={() =>
                      setTags(
                        tags.includes(tag)
                          ? tags.filter((value) => value !== tag)
                          : [...tags, tag],
                      )
                    }
                  />
                  <span>{tag}</span>
                </label>
              ))}
            </div>
          </details>
          <label className="op-thumbnail-control" title="对象缩略图大小">
            <input
              type="range"
              aria-label="对象缩略图大小"
              aria-valuetext={`${thumbnailSize} 像素`}
              min={160}
              max={360}
              step={10}
              value={thumbnailSize}
              onChange={(event) => setThumbnailSize(Number(event.target.value))}
            />
          </label>
          <div
            className="op-preview-tools op-segment"
            role="group"
            aria-label="预览操作"
          >
            <button
              type="button"
              aria-label="点选画面"
              title={canAnnotate ? "点选画面" : unavailableReason}
              aria-pressed={mode === "point"}
              disabled={!canAnnotate}
              onClick={() => setMode("point")}
            >
              <Icon name="target" />
            </button>
            <button
              type="button"
              aria-label="框选画面"
              title={canAnnotate ? "框选画面" : unavailableReason}
              aria-pressed={mode === "box"}
              disabled={!canAnnotate}
              onClick={() => setMode("box")}
            >
              <Icon name="maximize" />
            </button>
            <button
              type="button"
              aria-label="观察选中内容"
              title={inspect ? "观察选中内容" : unavailableReason}
              disabled={!inspect}
              onClick={inspect}
            >
              <Icon name="search" />
            </button>
          </div>
        </div>
        <div className="op-object-toolbar-right">
          <input
            className="op-object-search"
            aria-label="搜索对象"
            placeholder="搜索对象名称或描述"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <button
            type="button"
            className="op-object-action"
            aria-label="导入对象"
            title={importObjects ? "导入对象" : unavailableReason}
            disabled={!importObjects}
            onClick={importObjects}
          >
            <Icon name="import" />
          </button>
          <button
            type="button"
            className="op-object-action"
            aria-label="生成对象"
            title={generateObjects ? "生成对象" : unavailableReason}
            disabled={!generateObjects}
            onClick={generateObjects}
          >
            <Icon name="sparkles" />
          </button>
        </div>
      </header>
    </>
  );
}

import { useEffect, useState } from "react";
import { assetUrl } from "../api";
import { Icon } from "../Icon";
import type { ObjectCatalogRecord } from "./use-object-catalog";
import { readFrozenObjectVersion } from "../../shared/object-catalog";
import { ObjectVersionFilePreview } from "./ObjectVersionFilePreview";

export function ObjectCatalogDetails({
  object,
  objects,
  currentProjectId,
  select,
}: {
  object: ObjectCatalogRecord;
  objects: ObjectCatalogRecord[];
  currentProjectId: string;
  select: (id: string) => void;
}) {
  const [thumbnailUnavailable, setThumbnailUnavailable] = useState(false);
  const foreignReferences = object.references.filter(
    (reference) => reference.projectId !== currentProjectId,
  );
  const parent = object.parentObjectId
    ? objects.find(
        (candidate) =>
          candidate.projectId === currentProjectId &&
          candidate.id === object.parentObjectId,
      )
    : undefined;
  const children = objects.filter(
    (candidate) =>
      candidate.projectId === currentProjectId &&
      candidate.parentObjectId === object.id,
  );
  const [viewedVersionId, setViewedVersionId] = useState(
    object.versions[object.versions.length - 1]?.versionId ?? "",
  );
  const viewedVersion = object.versions.find(
    (version) => version.versionId === viewedVersionId,
  );
  useEffect(() => setThumbnailUnavailable(false), [object.thumbnailPath]);
  useEffect(() => {
    setViewedVersionId(
      object.versions[object.versions.length - 1]?.versionId ?? "",
    );
  }, [object.projectId, object.id, object.versions]);
  return (
    <section className="op-catalog-details" aria-label="对象详情">
      <div className="op-catalog-details-heading">
        <div>
          <span className="op-muted">对象详情</span>
          <h2>{object.name}</h2>
        </div>
        <code>{object.id}</code>
      </div>
      <div className="op-catalog-details-columns">
        <section aria-label="对象元数据">
          <h3>元数据</h3>
          <ul>
            <li>
              <Icon name="folder" />
              <span>分类</span>
              <small>{object.category}</small>
            </li>
            <li>
              <Icon name="tag" />
              <span>标签</span>
              <small>
                {object.tags.length ? object.tags.join("、") : "暂无"}
              </small>
            </li>
            <li>
              <Icon name="assets" />
              <span>缩略图</span>
              {object.thumbnailPath ? (
                <code title={object.thumbnailPath}>{object.thumbnailPath}</code>
              ) : (
                <small>未引用</small>
              )}
            </li>
          </ul>
          {object.thumbnailPath && (
            <div className="op-catalog-thumbnail">
              {thumbnailUnavailable ? (
                <p role="status">缩略图不可用：{object.thumbnailPath}</p>
              ) : (
                <img
                  src={assetUrl(
                    object.projectId,
                    object.thumbnailPath,
                    String(object.revision),
                  )}
                  alt={`${object.name} 缩略图`}
                  loading="lazy"
                  onError={() => setThumbnailUnavailable(true)}
                />
              )}
            </div>
          )}
        </section>
        <section aria-label="对象层级">
          <h3>对象层级</h3>
          <ul>
            <li>
              <Icon name="back" />
              <span>父对象</span>
              {parent ? (
                <button type="button" onClick={() => select(parent.id)}>
                  {parent.name}
                </button>
              ) : (
                <small>
                  {object.parentObjectId ? "未在当前项目目录中找到" : "无"}
                </small>
              )}
            </li>
            {children.map((child) => (
              <li key={child.id}>
                <Icon name="layers" />
                <span>子对象</span>
                <button type="button" onClick={() => select(child.id)}>
                  {child.name}
                </button>
              </li>
            ))}
          </ul>
          {!children.length && <p className="op-muted">暂无子对象</p>}
        </section>
        <section aria-label="对象组件">
          <h3>组件 ({object.components.length})</h3>
          {object.components.length ? (
            <ul>
              {object.components.map((component) => (
                <li key={component.id}>
                  <Icon name="layers" />
                  <span>{component.name}</span>
                  <small>{component.kind}</small>
                </li>
              ))}
            </ul>
          ) : (
            <p className="op-muted">暂无组件</p>
          )}
        </section>
        <section aria-label="对象文件">
          <h3>文件 ({object.files.length})</h3>
          {object.files.length ? (
            <ul>
              {object.files.map((file) => (
                <li key={`${file.role}:${file.path}`}>
                  <Icon name="folder" />
                  <span title={file.path}>{file.path}</span>
                  <small>{file.role}</small>
                </li>
              ))}
            </ul>
          ) : (
            <p className="op-muted">暂无文件</p>
          )}
        </section>
        <section aria-label="对象引用">
          <h3>引用 ({object.references.length})</h3>
          {object.references.length ? (
            <ul>
              {object.references.map((reference) => {
                const foreign = reference.projectId !== currentProjectId;
                return (
                  <li key={`${reference.projectId}:${reference.objectId}`}>
                    <Icon name="arrowUpRight" />
                    {foreign ? (
                      <span title="跨项目引用不可在此工作区打开">
                        {reference.objectId} · 其他项目
                      </span>
                    ) : (
                      <button
                        type="button"
                        onClick={() => select(reference.objectId)}
                      >
                        {reference.objectId}
                      </button>
                    )}
                    {reference.versionId && (
                      <small>{reference.versionId}</small>
                    )}
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="op-muted">暂无引用</p>
          )}
          {foreignReferences.length > 0 && (
            <p className="op-catalog-warning">
              已隐藏 {foreignReferences.length} 个跨项目打开入口。
            </p>
          )}
        </section>
        <section aria-label="对象版本">
          <h3>版本 ({object.versions.length})</h3>
          {object.versions.length ? (
            <>
              <label className="op-catalog-version-select">
                <span>查看版本</span>
                <select
                  aria-label="显示的对象版本"
                  value={viewedVersionId}
                  onChange={(event) => setViewedVersionId(event.target.value)}
                >
                  {object.versions.map((version) => (
                    <option key={version.versionId} value={version.versionId}>
                      {version.versionId}
                    </option>
                  ))}
                </select>
              </label>
              {viewedVersion && (
                <ObjectVersionFilePreview
                  key={JSON.stringify([
                    object.projectId,
                    object.id,
                    viewedVersion.versionId,
                  ])}
                  object={object}
                  version={viewedVersion}
                />
              )}
              {viewedVersion && (
                <p className="op-muted">
                  <Icon name="lock" /> {viewedVersion.versionId} ·
                  {readFrozenObjectVersion(object, viewedVersion)?.status ===
                  "importedPendingValidation"
                    ? "导入待验证，尚未接受"
                    : "只读查看，不修改登记"}
                </p>
              )}
            </>
          ) : (
            <p className="op-muted">暂无版本</p>
          )}
        </section>
      </div>
    </section>
  );
}

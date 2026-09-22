import { Icon } from "../Icon";
import type { ObjectCatalogRecord } from "./use-object-catalog";

export function ObjectCatalogDetails({
  object,
  currentProjectId,
  select,
}: {
  object: ObjectCatalogRecord;
  currentProjectId: string;
  select: (id: string) => void;
}) {
  const foreignReferences = object.references.filter(
    (reference) => reference.projectId !== currentProjectId,
  );
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
            <ul>
              {object.versions.map((version) => (
                <li key={version.versionId}>
                  <Icon name="lock" />
                  <span>{version.versionId}</span>
                  <small>只读清单</small>
                </li>
              ))}
            </ul>
          ) : (
            <p className="op-muted">暂无版本</p>
          )}
        </section>
      </div>
    </section>
  );
}

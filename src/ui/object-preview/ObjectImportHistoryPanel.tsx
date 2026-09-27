import { useEffect, useState, useSyncExternalStore } from "react";
import type {
  ImportHistoryEntry,
  ImportHistoryReceipt,
} from "../../shared/object-import-history";
import { call } from "../api";
import { ObjectImportHistory } from "./object-import-history";
import { ObjectFileImportPanel } from "./ObjectFileImportPanel";

export function ImportPreparationDetails({
  receipt,
}: {
  receipt: ImportHistoryReceipt;
}) {
  return (
    <section aria-label="已保存的导入准备">
      <p role="status">
        准备已保存。以下是准备时的冻结记录，未重新检查源文件；准备记录本身不表示正式导入已完成。
      </p>
      <p>
        准备 ID：<code>{receipt.preparationId}</code>
      </p>
      <p>
        请求 ID：<code>{receipt.requestId}</code>
      </p>
      <p>
        目标项目：<code>{receipt.targetProjectId}</code>
      </p>
      {receipt.kind === "project" ? (
        <>
          <p>
            来源：{receipt.sourcePath}（{receipt.sourceProjectId}）
          </p>
          <p>
            根对象：{receipt.sourceObjectId} · 固定版本：
            {receipt.acceptedVersionId}
          </p>
          <p>
            来源摘要：<code>{receipt.sourceDigest}</code>
          </p>
          {receipt.versions.map((version) => (
            <details
              key={JSON.stringify([version.objectId, version.versionId])}
            >
              <summary>
                {version.name} · {version.objectId} / {version.versionId}（
                {version.files.length} 个文件）
              </summary>
              <p>
                目标对象：
                <code>{receipt.identityMap.objects[version.objectId]}</code>
              </p>
              <p>
                目标版本：
                <code>{receipt.identityMap.versions[version.versionId]}</code>
              </p>
              <ul aria-label="冻结文件">
                {version.files.map((file) => (
                  <li key={file.path}>
                    {file.path} · {file.role} · {file.bytes} B ·{" "}
                    <code>{file.sha256}</code>
                  </li>
                ))}
              </ul>
              <ul aria-label="组件映射">
                {version.components.map((component) => (
                  <li key={component.id}>
                    {component.name}（{component.kind}） · {component.id} →{" "}
                    {receipt.identityMap.components[component.id]}
                  </li>
                ))}
              </ul>
              <ul aria-label="固定依赖">
                {version.references.map((reference) => (
                  <li key={JSON.stringify(reference)}>
                    {reference.objectId} / {reference.versionId}
                  </li>
                ))}
              </ul>
            </details>
          ))}
        </>
      ) : (
        <>
          <p>来源路径：{receipt.source.source.paths.join("；")}</p>
          <p>
            快照摘要：<code>{receipt.source.digest}</code>
          </p>
          <ul aria-label="已保存分组">
            {receipt.groups.map((group) => (
              <li key={group.id}>
                {group.name}（{group.paths.length} 项） · 目标身份：
                <code>{receipt.identityMap.groups[group.id]}</code>
              </li>
            ))}
          </ul>
          <ul aria-label="冻结文件">
            {receipt.source.files.map((file) => (
              <li key={file.path}>
                {file.path} · {file.kind} · {file.bytes} B ·{" "}
                <code>{file.sha256}</code>
                <p>
                  分组：
                  {receipt.groups.find((group) =>
                    group.paths.includes(file.path),
                  )?.name ?? "未分组"}
                </p>
                <p>
                  目标身份：<code>{receipt.identityMap.files[file.path]}</code>
                </p>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

export function ObjectImportHistoryPanel({
  projectId,
  prepared,
  openObject,
}: {
  projectId?: string;
  prepared?: ImportHistoryEntry;
  openObject?: (projectId: string, objectId: string) => void;
}) {
  const [history] = useState(() => new ObjectImportHistory(projectId, call));
  const state = useSyncExternalStore(
    history.subscribe,
    history.getSnapshot,
    history.getSnapshot,
  );
  useEffect(() => {
    void history.refresh(false, prepared);
    return () => history.cancel();
  }, [history, prepared]);
  return (
    <section aria-label="导入准备历史">
      <h3>已保存的导入准备</h3>
      <p className="op-muted">
        关闭重开或丢失准备响应后，可在这里找回目标项目的记录；源项目离线也可查看。历史查询不会改动当前表单、复制内容或启动任务。列表按记录标识排序。
      </p>
      <button
        type="button"
        disabled={!projectId || state.loading}
        onClick={() => void history.refresh()}
      >
        {state.loading ? "正在读取历史…" : "刷新准备历史"}
      </button>
      {state.loaded && !state.entries.length && <p>暂无已保存的准备记录。</p>}
      <ul>
        {state.entries.map((entry) => (
          <li key={entry.cursor}>
            <button
              type="button"
              disabled={state.loading}
              aria-pressed={state.selected?.cursor === entry.cursor}
              onClick={() => void history.open(entry)}
            >
              {entry.kind === "files" ? "文件和文件夹" : "Beaver 项目"} · 请求{" "}
              {entry.requestId}
            </button>
            <code>{entry.preparationId}</code>
          </li>
        ))}
      </ul>
      {state.next && (
        <button
          type="button"
          disabled={state.loading}
          onClick={() => void history.refresh(true)}
        >
          加载更多准备记录
        </button>
      )}
      {state.reading && <p role="status">正在读取冻结准备记录…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.receipt && <ImportPreparationDetails receipt={state.receipt} />}
      {state.receipt && (
        <ObjectFileImportPanel
          key={state.receipt.preparationId}
          projectId={state.receipt.targetProjectId}
          preparationId={state.receipt.preparationId}
          sourceKind={state.receipt.kind}
          openObject={openObject}
        />
      )}
    </section>
  );
}

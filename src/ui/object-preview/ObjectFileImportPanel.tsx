import { useEffect, useState, useSyncExternalStore } from "react";
import { call } from "../api";
import { FileImportSession } from "./object-file-import";

export function ObjectFileImportPanel({
  projectId,
  preparationId,
  openObject,
  sourceKind = "files",
}: {
  projectId: string;
  preparationId: string;
  openObject?: (projectId: string, objectId: string) => void;
  sourceKind?: "files" | "project";
}) {
  const [session] = useState(
    () => new FileImportSession(projectId, preparationId, call, sourceKind),
  );
  const [confirmed, setConfirmed] = useState(false);
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    void session.refresh();
    return () => session.cancel();
  }, [session]);
  const operation = state.operation;
  const status = operation?.state;
  return (
    <section
      aria-label={sourceKind === "files" ? "正式文件导入" : "正式项目对象导入"}
    >
      <h3>
        {sourceKind === "files"
          ? "正式导入文件和文件夹"
          : "正式导入对象及固定依赖"}
      </h3>
      {sourceKind === "project" ? (
        <p>
          复制冻结版本及固定依赖闭包，创建新的对象、组件和版本身份并保存来源。
          保留项目内原相对路径和文件内容，目标已有同路径文件时报告冲突，不覆盖。
          同一依赖的多个版本均保留；工作目录使用根对象所选版本和依赖的首个冻结版本。
          闭包外的组织父对象不导入。源项目保持原样；每次最多 512 MiB、1024
          个对象。
        </p>
      ) : (
        <p>
          每个分组登记为一个对象，未分组文件各自登记。文件复制到 imports/
          {preparationId}/root-N/
          下，同一来源根目录内保留相对布局；不改写文件内容或跨根目录引用。源文件保留，目标文件不覆盖。每次最多
          512 MiB、1024 个对象，每个对象最多 1024 个文件。
        </p>
      )}
      <p>
        导入版本标记为“导入待验证”，不能作为已接受引用；本次不运行 Godot
        或批准资源。
      </p>
      {(!status || status === "applying") && (
        <>
          <label>
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)}
            />
            确认按以上冻结准备和目录布局导入
          </label>
          <button
            type="button"
            disabled={!state.loaded || state.busy || !confirmed}
            onClick={() => void session.commit()}
          >
            {status === "applying" ? "继续原导入" : "确认正式导入"}
          </button>
        </>
      )}
      {(status === "applying" || status === "aborting") && (
        <button
          type="button"
          disabled={state.busy}
          onClick={() => void session.abort()}
        >
          {status === "aborting" ? "重试中止清理" : "中止并清理本次写入"}
        </button>
      )}
      <button
        type="button"
        disabled={state.busy}
        onClick={() => void session.refresh()}
      >
        查询导入状态
      </button>
      {state.busy && (
        <p role="status">正在处理；关闭后可从准备历史查询结果。</p>
      )}
      {status === "applying" && (
        <p role="status">内容已冻结，导入尚未完成；可离线继续原请求或中止。</p>
      )}
      {status === "aborting" && (
        <p role="status">
          中止清理未完成，外部修改保持原样。保存冲突文件后移出本次目标路径，再重试清理。
        </p>
      )}
      {status === "aborted" && (
        <p role="status">
          已中止，准备记录仍保留；如需重新导入，请创建新准备。
        </p>
      )}
      {operation?.state === "importedPendingValidation" && (
        <>
          <p role="status">导入完成，待验证。重复请求不会再创建对象。</p>
          <ul>
            {operation.objectIds.map((id) => (
              <li key={id}>
                <code>{id}</code>
                {openObject && (
                  <button
                    type="button"
                    onClick={() => openObject(projectId, id)}
                  >
                    打开导入对象
                  </button>
                )}
              </li>
            ))}
          </ul>
        </>
      )}
      {(state.error || operation?.error) && (
        <p role="alert">{state.error || operation?.error}</p>
      )}
    </section>
  );
}

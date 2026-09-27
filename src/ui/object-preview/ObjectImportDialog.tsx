import { useMemo, useSyncExternalStore } from "react";
import type { Project } from "../../shared/types";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import { useObjectImportDraft } from "./use-object-import-draft";
import { ObjectImportHistoryPanel } from "./ObjectImportHistoryPanel";

export function ObjectImportDialog({
  close,
  notify,
  projectId,
  projects = [],
  openObject,
}: {
  close: () => void;
  notify: (message: string) => void;
  projectId?: string;
  projects?: readonly Pick<Project, "id" | "name" | "path">[];
  openObject?: (projectId: string, objectId: string) => void;
}) {
  const {
    session,
    groupName,
    setGroupName,
    error: draftError,
    restored,
  } = useObjectImportDraft(projectId);
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const selected = session.selectedObject;
  const prepared = useMemo(() => {
    const receipt = state.receipt;
    if (!receipt) return undefined;
    const kind =
      "sourceObjectId" in receipt ? ("project" as const) : ("files" as const);
    return {
      kind,
      preparationId: receipt.preparationId,
      requestId: receipt.requestId,
      cursor:
        (kind === "files"
          ? "file_object_import_preparation:"
          : "object_import_preparation:") + receipt.preparationId,
    };
  }, [state.receipt]);
  const version = session.selectedVersion;
  const contents = version?.manifest?.files ?? [];
  const fileSnapshot = state.fileSnapshot;
  const sources = projects.filter(
    (project) => project.id !== session.targetProjectId,
  );
  const registeredSource = sources.find(
    (project) =>
      state.sourceMode === "project" &&
      project.id === state.sourceProjectId &&
      project.path === state.path,
  );
  const dismiss = () => {
    close();
  };
  return (
    <Dialog
      title="准备导入对象"
      close={dismiss}
      className="op-object-import-dialog"
    >
      <p className="op-muted">
        读取已登记或外部 Beaver
        项目的已接受版本，已打开项目使用已提交快照；也可以读取普通文件和文件夹的只读快照。
      </p>
      <p>
        目标项目：<code>{session.targetProjectId ?? "尚未选择项目"}</code>
      </p>
      <form
        onSubmit={async (event) => {
          event.preventDefault();
          const receipt = await session.prepare();
          if (receipt && session.getSnapshot().receipt === receipt) {
            notify(
              state.sourceMode === "files"
                ? "文件准备回执已保存，可在下方准备历史中打开并确认正式导入"
                : `已准备对象 ${"sourceObjectId" in receipt ? receipt.sourceObjectId : ""} 的版本 ${"acceptedVersionId" in receipt ? receipt.acceptedVersionId : ""}，正式导入尚未提交`,
            );
          }
        }}
      >
        <p className="op-muted">
          {restored
            ? "已恢复本地草稿，请显式重新读取来源后准备导入。"
            : "输入按目标项目保存在本机，关闭后可继续编辑。"}{" "}
          已保存的准备请从下方历史打开。
        </p>
        {draftError && <p role="alert">{draftError}</p>}
        <fieldset className="op-import-folders">
          <legend>来源类型</legend>
          <button
            type="button"
            aria-pressed={state.sourceMode === "project"}
            onClick={() => session.setSourceMode("project")}
          >
            已登记项目
          </button>
          <button
            type="button"
            aria-pressed={state.sourceMode === "external"}
            onClick={() => session.setSourceMode("external")}
          >
            外部 Beaver 项目
          </button>
          <button
            type="button"
            aria-pressed={state.sourceMode === "files"}
            onClick={() => session.setSourceMode("files")}
          >
            文件和文件夹
          </button>
        </fieldset>
        {state.sourceMode !== "files" ? (
          <>
            {state.sourceMode === "project" ? (
              <label className="op-field">
                导入来源
                <select
                  aria-label="导入来源"
                  value={registeredSource?.id ?? ""}
                  onChange={(event) => {
                    const source = sources.find(
                      (project) => project.id === event.target.value,
                    );
                    session.selectSource({
                      path: source?.path ?? "",
                      projectId: source?.id ?? "",
                    });
                  }}
                >
                  <option value="">选择已登记项目</option>
                  {sources.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name} ({project.id})
                    </option>
                  ))}
                </select>
              </label>
            ) : (
              <button
                type="button"
                onClick={() => void session.choosePaths("project")}
                disabled={session.busy}
              >
                选择外部项目文件夹
              </button>
            )}
            <label className="op-field">
              源项目目录
              <input
                aria-label="源项目目录"
                value={state.path}
                readOnly={state.sourceMode === "project"}
                onChange={(event) =>
                  session.setSource("path", event.target.value)
                }
                placeholder="包含 .beaver 存储的项目绝对路径…"
              />
            </label>
            <label className="op-field">
              源项目 ID
              <input
                aria-label="源项目 ID"
                value={state.sourceProjectId}
                readOnly
                placeholder="读取目录后自动识别"
              />
            </label>
            <button
              type="button"
              onClick={() => void session.inspect()}
              disabled={session.busy || !state.path.trim()}
            >
              {state.phase === "inspecting" ? "正在读取…" : "读取源对象"}
            </button>
            {state.objects.length > 0 && (
              <fieldset className="op-import-folders">
                <legend>源对象目录</legend>
                {state.objects.map((object) => (
                  <label key={object.id}>
                    <input
                      type="radio"
                      name="source-object"
                      checked={state.objectId === object.id}
                      onChange={() => session.selectObject(object.id)}
                    />
                    <Icon name="folder" />
                    <span>
                      {object.name} ({object.id})
                    </span>
                  </label>
                ))}
              </fieldset>
            )}
            <div className="op-import-contents">
              {contents.length ? (
                contents.map((file) => (
                  <span key={file.path}>
                    <Icon name="project" />
                    {file.path}
                  </span>
                ))
              ) : (
                <p className="op-muted">
                  {version?.manifest
                    ? "此接受版本没有直属文件。准备时仍会校验其引用的版本和内容。"
                    : "读取源对象并选择可用的接受版本后，显示该版本的冻结文件清单。"}
                </p>
              )}
            </div>
            <label className="op-field">
              固定接受版本
              <select
                value={state.versionId}
                onChange={(event) => session.selectVersion(event.target.value)}
                disabled={!selected?.versions.length}
              >
                {!selected?.versions.length && (
                  <option value="">无接受版本</option>
                )}
                {selected?.versions.map((version) => (
                  <option key={version.versionId} value={version.versionId}>
                    {version.sourceDigest ? "已接受" : "不可准备"} ·{" "}
                    {version.versionId}
                  </option>
                ))}
              </select>
            </label>
            {version?.manifest && (
              <p>版本内对象名称：{version.manifest.name}</p>
            )}
            {version?.blocker && (
              <p role="alert">此版本无法准备：{version.blocker}</p>
            )}
            {state.phase === "ready" && !state.objects.length && (
              <p role="status">源项目没有已登记的对象。</p>
            )}
            {state.sourceProjectId.trim() === session.targetProjectId && (
              <p role="alert">源项目和目标项目必须不同。</p>
            )}
          </>
        ) : (
          <>
            <div className="op-dialog-actions">
              <button
                type="button"
                onClick={() => void session.choosePaths("files")}
                disabled={session.busy}
              >
                选择文件
              </button>
              <button
                type="button"
                onClick={() => void session.choosePaths("directory")}
                disabled={session.busy}
              >
                选择文件夹
              </button>
              <button
                type="button"
                onClick={() => void session.inspectFiles()}
                disabled={session.busy || !state.selectedPaths.length}
              >
                {state.phase === "inspecting" ? "正在读取…" : "读取文件快照"}
              </button>
            </div>
            <p className="op-muted">
              可以多次选择文件和文件夹。路径只会去重和规范化，不会按目录、扩展名或相近名称自动归组。
            </p>
            <ul className="op-import-contents" aria-label="待读取路径">
              {state.selectedPaths.map((path) => (
                <li key={path}>
                  <code>{path}</code>
                  <button
                    type="button"
                    onClick={() => session.removeFilePath(path)}
                  >
                    移除
                  </button>
                </li>
              ))}
            </ul>
            {fileSnapshot && (
              <fieldset className="op-import-folders">
                <legend>文件快照（{fileSnapshot.files.length} 项）</legend>
                {fileSnapshot.files.map((file) => (
                  <label key={file.path}>
                    <span>
                      {file.relativePath} · {file.kind} · {file.bytes} B
                    </span>
                    <select
                      aria-label={`文件分组 ${file.relativePath}`}
                      value={
                        state.fileGroups.find((group) =>
                          group.paths.includes(file.path),
                        )?.id ?? ""
                      }
                      onChange={(event) =>
                        session.assignFileToGroup(file.path, event.target.value)
                      }
                    >
                      <option value="">未分组</option>
                      {state.fileGroups.map((group) => (
                        <option key={group.id} value={group.id}>
                          {group.name}
                        </option>
                      ))}
                    </select>
                  </label>
                ))}
              </fieldset>
            )}
            <div className="op-dialog-actions">
              <input
                aria-label="新分组名称"
                value={groupName}
                onChange={(event) => setGroupName(event.target.value)}
                placeholder="新分组名称"
              />
              <button
                type="button"
                onClick={() => {
                  session.addFileGroup(groupName);
                  setGroupName("");
                }}
              >
                添加分组
              </button>
            </div>
            {state.fileGroups.map((group) => (
              <p key={group.id}>
                {group.name}（{group.paths.length} 项）
                <button
                  type="button"
                  onClick={() => session.removeFileGroup(group.id)}
                >
                  删除分组
                </button>
              </p>
            ))}
            <p className="op-muted">
              普通文件可先预览和人工归组；保存准备后，在下方历史中打开记录并确认正式导入。
            </p>
          </>
        )}
        <div className="op-dialog-note">
          <Icon name="review" />
          <span>
            {state.sourceMode === "files"
              ? "普通文件快照只读，不会写入源目录；准备回执不会复制文件。"
              : "准备会固定所选版本及其依赖，校验冻结内容并保存重试回执。目标对象目录尚不改变；保存后在下方核对固定依赖和目录布局，再确认正式导入。"}
          </span>
        </div>
        <footer>
          <button type="button" onClick={dismiss}>
            关闭
          </button>
          <button
            type="submit"
            className="primary"
            disabled={!session.canPrepare}
          >
            <Icon name="import" />
            {state.phase === "preparing" ? "正在准备…" : "准备导入"}
          </button>
        </footer>
        {state.receipt && (
          <p role="status">
            准备回执：{state.receipt.preparationId}
            。请在下方历史中查看提交状态。
          </p>
        )}
        {state.error && <p role="alert">{state.error}</p>}
      </form>
      <ObjectImportHistoryPanel
        projectId={session.targetProjectId}
        key={session.targetProjectId}
        prepared={prepared}
        openObject={openObject}
      />
    </Dialog>
  );
}

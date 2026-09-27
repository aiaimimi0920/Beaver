import { useState, useSyncExternalStore } from "react";
import { Dialog } from "../components";
import { call } from "../api";
import { browserDraftStorage } from "./object-import-draft";
import { ObjectGenerationSession } from "./object-generation-session";

export function ObjectGenerateDialog({
  projectId,
  close,
  openObject,
  openManufacture,
}: {
  projectId: string;
  close: () => void;
  openObject: (projectId: string, objectId: string) => void;
  openManufacture: (
    projectId: string,
    objectId: string,
    taskId: string,
  ) => void;
}) {
  const [session] = useState(
    () => new ObjectGenerationSession(projectId, call, browserDraftStorage),
  );
  const { saved, busy, error } = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const { fields, pending, receipt } = saved;
  const locked = busy || Boolean(pending) || Boolean(receipt);
  return (
    <Dialog
      title="生成对象"
      close={close}
      className="op-object-generate-dialog"
    >
      <p className="op-muted">
        确认后创建新对象及待调度制作任务；不会立即调用模型或生成文件。草稿按目标项目保存。
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void session.submit();
        }}
      >
        <fieldset disabled={locked}>
          <div className="op-object-form-row">
            <label className="op-field">
              对象名称（选填）
              <input
                maxLength={256}
                value={fields.name}
                onChange={(event) => session.edit({ name: event.target.value })}
              />
            </label>
            <label className="op-field">
              生成内容
              <select
                value={fields.category}
                onChange={(event) =>
                  session.edit({ category: event.target.value })
                }
              >
                {["图像", "音频", "模型", "场景", "翻译", "脚本", "其他"].map(
                  (value) => (
                    <option key={value}>{value}</option>
                  ),
                )}
              </select>
            </label>
          </div>
          <label className="op-field">
            任务描述
            <textarea
              rows={4}
              required
              maxLength={20000}
              value={fields.prompt}
              onChange={(event) => session.edit({ prompt: event.target.value })}
            />
          </label>
          <label className="op-field">
            验收要求（选填）
            <textarea
              rows={2}
              maxLength={10000}
              value={fields.acceptance}
              onChange={(event) =>
                session.edit({ acceptance: event.target.value })
              }
            />
          </label>
        </fieldset>
        {pending && <p>请求：{saved.id}。准备记录保留，重开后需手动重试。</p>}
        {error && <p role="alert">{error}</p>}
        {receipt && (
          <p role="status">
            对象和制作任务已创建，尚未生成或验收。请在项目任务面板中查看和调度。
          </p>
        )}
        <footer>
          <button type="button" onClick={close}>
            关闭并保留草稿
          </button>
          {receipt ? (
            <>
              <button type="button" onClick={() => session.newDraft()}>
                创建另一个对象
              </button>
              <button
                type="button"
                className="primary"
                onClick={() => {
                  const run = receipt.runs[0];
                  if (run)
                    openManufacture(projectId, run.objectId, run.mediumTaskId);
                }}
              >
                前往制造任务
              </button>
              <button
                type="button"
                onClick={() => {
                  const id = receipt.objectIds[0];
                  if (id) openObject(projectId, id);
                }}
              >
                打开对象
              </button>
            </>
          ) : (
            <button
              type="submit"
              className="primary"
              disabled={busy || !fields.prompt.trim()}
            >
              {busy
                ? "正在确认…"
                : pending
                  ? "按原请求重试"
                  : "确认创建对象和任务"}
            </button>
          )}
        </footer>
      </form>
    </Dialog>
  );
}

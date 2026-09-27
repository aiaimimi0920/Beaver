import { useEffect, useState, useSyncExternalStore } from "react";
import type { ObjectPublication } from "./object-publication";
import { ObjectPublicationFeedback } from "./ObjectPublicationFeedback";
import { ObjectPublicationFollowupPanel } from "./ObjectPublicationFollowupPanel";
import type {
  PublicationDecision,
  PublicationPreview,
} from "../../shared/object-publication";

export function ObjectPublicationPanel({
  session,
}: {
  session: ObjectPublication;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const [abortConfirmed, setAbortConfirmed] = useState<string | null>(null);
  useEffect(() => {
    void session.refresh();
    return session.cancel;
  }, [session]);
  return (
    <section aria-label="最终接受与发布">
      <h4>最终接受与发布</h4>
      <p>
        以下为当前发布状态。上方审阅保留生成时证据；重开不会自动发布。发布完成后接受版本、任务状态与队列一起更新。
      </p>
      <button disabled={state.busy} onClick={() => void session.refresh()}>
        刷新发布状态与差异
      </button>
      {state.busy && <p role="status">正在处理发布记录…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.restored && (
        <p role="status">
          已恢复原发布或中止请求；重开只查询状态，不会自动提交。
        </p>
      )}
      {state.storageError && <p role="alert">{state.storageError}</p>}
      {state.recoveryBlocked && (
        <button
          disabled={state.busy}
          onClick={() => {
            session.restore();
            void session.refresh();
          }}
        >
          重试读取原发布请求
        </button>
      )}
      {state.operations.map((op) => (
        <article key={op.request.requestId}>
          <h5>
            {
              {
                applying: "发布未完成，对象占用保留",
                aborting: "中止尚未完成",
                published: "已发布，对象占用已释放",
                aborted: "发布已中止，候选与对象占用保留",
              }[op.state]
            }
          </h5>
          <p>
            请求：{op.request.requestId} · 尝试：{op.request.target.attemptId} ·
            版本：{op.versionId}
          </p>
          <p>接受说明：{op.request.acceptanceNote}</p>
          {op.preview.feedback.map((item) => (
            <ObjectPublicationFeedback
              key={item.requestId}
              session={session}
              feedback={item}
            />
          ))}
          {op.request.feedback.map((item) => (
            <p key={item.requestId}>
              {item.requestId} ·{" "}
              {
                {
                  resolved: "已解决",
                  waived: "豁免",
                  deferred: "排入后续中修",
                }[item.resolution]
              }
              ：{item.note}
            </p>
          ))}
          {op.error && <p role="alert">{op.error}</p>}
          {op.state === "published" && (
            <ObjectPublicationFollowupPanel session={session.followupFor(op)} />
          )}
          {["applying", "aborting"].includes(op.state) && (
            <>
              <label>
                <input
                  type="checkbox"
                  checked={abortConfirmed === op.request.requestId}
                  onChange={(event) =>
                    setAbortConfirmed(
                      event.target.checked ? op.request.requestId : null,
                    )
                  }
                />
                确认中止发布，回退可归属的文件写入；保留外部修改和冻结成果
              </label>
              <button
                disabled={
                  state.busy ||
                  state.recoveryBlocked ||
                  !state.reconciled ||
                  abortConfirmed !== op.request.requestId
                }
                onClick={() =>
                  void session.abort(
                    op.request.requestId,
                    abortConfirmed === op.request.requestId,
                  )
                }
              >
                中止此发布
              </button>
            </>
          )}
        </article>
      ))}
      {state.retry && (
        <button
          disabled={state.busy || state.recoveryBlocked || !state.reconciled}
          onClick={() => void session.retry()}
        >
          重试原发布或中止请求
        </button>
      )}
      {state.preview && !state.retry && (
        <PublicationApproval
          key={state.preview.digest}
          session={session}
          preview={state.preview}
          busy={state.busy || state.recoveryBlocked || !state.reconciled}
        />
      )}
    </section>
  );
}

function PublicationApproval({
  session,
  preview,
  busy,
}: {
  session: ObjectPublication;
  preview: PublicationPreview;
  busy: boolean;
}) {
  const draft = session.approvalFor(preview);
  const saved = useSyncExternalStore(
    draft.subscribe,
    draft.getSnapshot,
    draft.getSnapshot,
  );
  const { note, decisions } = saved.draft;
  const [files, setFiles] = useState(false);
  const [replacement, setReplacement] = useState(false);
  const setDecisions = (next: typeof decisions) =>
    draft.edit({ decisions: next });
  const complete = preview.feedback.every(
    (item) =>
      decisions[item.requestId]?.resolution &&
      decisions[item.requestId]?.note.trim(),
  );
  return (
    <div>
      <p>
        制作基线：{preview.baselineVersionId ?? "空基线"} · 当前接受版本：
        {preview.acceptedVersionId ?? "无"}
      </p>
      <h5>发布后的完整自有文件登记</h5>
      <ul>
        {preview.files.map((file) => (
          <li key={file.path}>
            {file.path} · {file.role}
          </li>
        ))}
      </ul>
      <h5>相对当前接受版本的文件差异</h5>
      <ul>
        {preview.paths.map((file) => (
          <li key={file.path} style={{ overflowWrap: "anywhere" }}>
            {file.path} ·{" "}
            {file.before === file.after
              ? "未变更"
              : !file.before
                ? "新增"
                : !file.after
                  ? "删除"
                  : "修改"}
            <p>
              原 SHA-256：{file.before ?? "无"} / 新 SHA-256：
              {file.after ?? "无"}
            </p>
          </li>
        ))}
      </ul>
      <p>
        接受说明与反馈处置按当前发布摘要保存在本机；刷新或重开可恢复，摘要变化后需重新填写。文件归属及替换确认不会恢复，提交前请重新核对。
      </p>
      {saved.restored && <p role="status">已恢复当前发布草稿，尚未提交。</p>}
      {saved.error && (
        <div role="alert">
          <p>{saved.error}</p>
          <button
            disabled={busy}
            onClick={saved.blocked ? draft.restore : draft.persist}
          >
            {saved.blocked ? "重试读取发布草稿" : "重试保存发布草稿"}
          </button>
        </div>
      )}
      <fieldset disabled={busy || saved.blocked}>
        <legend>人工接受决定</legend>
        <label>
          <input
            type="checkbox"
            checked={files}
            onChange={(event) => setFiles(event.target.checked)}
          />
          确认以上完整文件归属
        </label>
        {preview.replacementRequired && (
          <label>
            <input
              type="checkbox"
              checked={replacement}
              onChange={(event) => setReplacement(event.target.checked)}
            />
            确认使用历史或空基线成果替换当前接受版本
          </label>
        )}
        {preview.feedback.map((item) => {
          const decision = decisions[item.requestId] ?? {
            resolution: "",
            note: "",
          };
          return (
            <div key={item.requestId}>
              <ObjectPublicationFeedback session={session} feedback={item} />
              <label>
                处置
                <select
                  value={decision.resolution}
                  onChange={(event) =>
                    setDecisions({
                      ...decisions,
                      [item.requestId]: {
                        ...decision,
                        resolution: event.target.value as
                          PublicationDecision["resolution"] | "",
                      },
                    })
                  }
                >
                  <option value="">请选择</option>
                  {item.later ? (
                    <option value="deferred">确认排入后续中修</option>
                  ) : (
                    <>
                      <option value="resolved">已解决</option>
                      <option value="waived">明确豁免</option>
                    </>
                  )}
                </select>
              </label>
              <label>
                处置说明
                <textarea
                  value={decision.note}
                  onChange={(event) =>
                    setDecisions({
                      ...decisions,
                      [item.requestId]: {
                        ...decision,
                        note: event.target.value,
                      },
                    })
                  }
                />
              </label>
            </div>
          );
        })}
        <label>
          最终接受说明
          <textarea
            value={note}
            onChange={(event) => draft.edit({ note: event.target.value })}
          />
        </label>
        <button
          disabled={
            !files ||
            !note.trim() ||
            !complete ||
            !!saved.error ||
            (preview.replacementRequired && !replacement)
          }
          onClick={() =>
            draft.persist() &&
            void session.publish({
              acceptanceNote: note,
              confirmFiles: files,
              confirmReplacement: replacement,
              feedback: preview.feedback.flatMap((item) => {
                const decision = decisions[item.requestId];
                return decision?.resolution
                  ? [
                      {
                        requestId: item.requestId,
                        resolution: decision.resolution,
                        note: decision.note,
                      },
                    ]
                  : [];
              }),
            })
          }
        >
          接受并发布对象
        </button>
      </fieldset>
    </div>
  );
}

import { useEffect, useState, useSyncExternalStore } from "react";
import type { ObjectPublication } from "./object-publication";
import { ObjectPublicationFeedback } from "./ObjectPublicationFeedback";
import { ObjectPublicationFollowupPanel } from "./ObjectPublicationFollowupPanel";
import { PublicationApproval } from "./PublicationApproval";

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
              {item.finalRelocation && (
                <span>
                  {" · "}已确认最终编号帧{" "}
                  {item.finalRelocation.targetFrame.frameId}：
                  {item.finalRelocation.regions
                    .map((region) =>
                      region.status === "matched"
                        ? `原区域 ${region.sourceRegion + 1} → 最终区域 ${region.targetRegion + 1}`
                        : `原区域 ${region.sourceRegion + 1} 无对应：${region.note}`,
                    )
                    .join("；")}
                </span>
              )}
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

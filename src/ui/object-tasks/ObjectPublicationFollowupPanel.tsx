import { useEffect, useSyncExternalStore } from "react";
import type { ObjectPublicationFollowup } from "./object-publication-followup";
import { PublicationFramePicker } from "./PublicationFramePicker";

export function ObjectPublicationFollowupPanel({
  session,
}: {
  session: ObjectPublicationFollowup;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  useEffect(() => {
    void session.refresh();
    return session.cancel;
  }, [session]);
  return (
    <section aria-label="发布后改进任务">
      <h5>创建后续中任务</h5>
      <p>
        反馈固定到发布版本 {session.publication.versionId}
        。创建后在任务列表中确认并入队；不会自动执行。
      </p>
      <p>
        可选择此发布版本的已保存编号帧。模型须重新定位历史区域，不自动沿用旧坐标。
      </p>
      <p>草稿保存在当前桌面数据目录中，不跨设备同步。</p>
      {state.restored && <p role="status">已恢复本地草稿或原创建请求。</p>}
      {state.storageError && <p role="alert">{state.storageError}</p>}
      {state.recoveryBlocked && (
        <button onClick={session.restore}>重新读取本地草稿</button>
      )}
      {(["title", "feedback", "acceptance"] as const).map((field) => (
        <label key={field}>
          {
            {
              title: "后续任务标题",
              feedback: "改进要求",
              acceptance: "验收标准",
            }[field]
          }
          <textarea
            value={state.draft[field]}
            disabled={state.busy || state.retry || state.recoveryBlocked}
            onChange={(event) => session.edit({ [field]: event.target.value })}
          />
        </label>
      ))}
      <PublicationFramePicker session={session} />
      <button
        disabled={
          state.busy ||
          state.recoveryBlocked ||
          (!state.retry &&
            [
              state.draft.title,
              state.draft.feedback,
              state.draft.acceptance,
            ].some((value) => !value.trim()))
        }
        onClick={() => void session.create()}
      >
        {state.retry ? "重试原创建请求" : "按此发布版本创建后续任务"}
      </button>
      <button disabled={state.busy} onClick={() => void session.refresh()}>
        刷新后续任务记录
      </button>
      {state.busy && <p role="status">正在处理后续任务…</p>}
      {state.error && <p role="alert">{state.error}</p>}
      {state.receipts.map((receipt) => (
        <article key={receipt.request.requestId}>
          <p>
            已创建：{receipt.request.title} · 中任务 {receipt.mediumTaskId} ·
            细任务 {receipt.fineTaskId}
          </p>
          <p>改进要求：{receipt.request.feedback}</p>
          <p>验收标准：{receipt.request.acceptance}</p>
          {receipt.request.previewFrame && (
            <p>已绑定原始编号帧：{receipt.request.previewFrame.frameId}</p>
          )}
          <p>
            来源尝试：{receipt.source.attemptId} · 固定版本：
            {receipt.request.versionId}
          </p>
        </article>
      ))}
    </section>
  );
}

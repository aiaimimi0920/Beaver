import { useEffect, useState } from "react";
import type {
  AssetAnnotation,
  AssetFeedback,
  AssetReference,
  AssetSubmission,
} from "../../shared/asset-task";
import { call, type Run } from "../api";
import { errorMessage } from "../notification-state";
import {
  archiveDraft,
  clearDraft,
  readDraft,
  writeDraft,
  type FeedbackDraft,
} from "./feedback-draft";
import { ReferenceEditor } from "./ReferenceEditor";

export function FeedbackComposer({
  id,
  reference,
  annotations,
  select,
  mark,
  capture,
  canCapture,
  finished,
  run,
  lock,
  blocked = false,
}: {
  id: string;
  reference: AssetReference | null;
  annotations: AssetAnnotation[];
  select: (reference: AssetReference | null) => void;
  mark: (annotations: AssetAnnotation[]) => void;
  capture: () => Promise<AssetReference>;
  canCapture: boolean;
  finished: boolean;
  run: Run;
  lock: (locked: boolean) => void;
  blocked?: boolean;
}) {
  const [initial] = useState(() => {
    try {
      return { draft: readDraft(localStorage, id), error: "" };
    } catch (error) {
      return {
        draft: null,
        error: `无法读取待提交反馈：${errorMessage(error)}`,
      };
    }
  });
  const [pending, setPending] = useState<FeedbackDraft | null>(initial.draft);
  const [draftError, setDraftError] = useState(initial.error);
  const [cleanupError, setCleanupError] = useState("");
  const [text, setText] = useState("");
  const [timing, setTiming] = useState<AssetSubmission["timing"]>("now");
  const [busy, setBusy] = useState(false);
  const [receipt, setReceipt] = useState<AssetFeedback | null>(null);
  const locked = busy || blocked || !!pending || !!draftError;
  useEffect(() => {
    lock(locked);
  }, [lock, locked]);
  const reset = () => {
    try {
      clearDraft(localStorage, id);
    } catch (error) {
      setCleanupError(
        `后端已确认结果，本地草稿清理失败：${errorMessage(error)}。请清理草稿后再填写下一条。`,
      );
      return;
    }
    setCleanupError("");
    setPending(null);
    setText("");
    select(null);
    mark([]);
  };
  const send = () => {
    if (busy || blocked || draftError || cleanupError) return;
    setBusy(true);
    void run(async () => {
      let draft = pending;
      if (!draft) {
        const fixed = reference ?? (await capture());
        draft = {
          version: 1,
          reference: fixed,
          submission: {
            id,
            feedbackId: crypto.randomUUID(),
            referenceId: fixed.id,
            text: text.trim(),
            timing,
            annotations,
          },
        };
        // Persist the immutable request before IPC, including the ID used after a lost response.
        draft = writeDraft(localStorage, draft);
        setPending(draft);
      }
      const result = await call<AssetFeedback>(
        "assetTask.submit",
        draft.submission,
      );
      setReceipt(result);
      reset();
    }, "反馈已保存，可在处理记录中查看进展。").finally(() => setBusy(false));
  };
  const fixed = pending?.reference ?? reference;
  return (
    <section
      className="asset-feedback-composer"
      aria-label="向当前资产提交修改"
    >
      {draftError && (
        <div role="alert">
          <p>{draftError} 请先从处理记录核对是否已经提交，避免重复修改。</p>
          <button
            disabled={busy || blocked}
            onClick={() => {
              void run(async () => {
                archiveDraft(localStorage, id);
                setDraftError("");
              });
            }}
          >
            已核对处理记录，备份损坏草稿并重新填写
          </button>
        </div>
      )}
      {cleanupError && (
        <p role="alert">
          {cleanupError}
          <button disabled={busy} onClick={reset}>
            清理已确认草稿
          </button>
        </p>
      )}
      {pending && !cleanupError && (
        <p className="asset-notice" role="status">
          提交 {pending.submission.feedbackId}{" "}
          尚待确认。重试保留原文、截图和编号；重开窗口不会自动执行。
        </p>
      )}
      <label className="field">
        <span>修改要求 · 当前绑定任务</span>
        <textarea
          aria-label="资产修改要求"
          value={pending?.submission.text ?? text}
          maxLength={12000}
          rows={4}
          disabled={locked}
          placeholder="例如：鼻子小一点"
          onChange={(event) => setText(event.target.value)}
        />
      </label>
      {fixed ? (
        <ReferenceEditor
          key={fixed.id}
          taskId={id}
          reference={fixed}
          annotations={pending?.submission.annotations ?? annotations}
          change={mark}
          locked={locked}
          clear={() => {
            select(null);
            mark([]);
          }}
        />
      ) : (
        <p className="asset-caption">
          提交时附带当前显示的实际画面。可先定格并标注，或在左侧直接点选三维位置。
        </p>
      )}
      {!fixed && (
        <button
          disabled={locked || !canCapture}
          onClick={() => {
            setBusy(true);
            void run(async () => select(await capture())).finally(() =>
              setBusy(false),
            );
          }}
        >
          定格画面并标注
        </button>
      )}
      <label className="field">
        <span>处理时机</span>
        <select
          aria-label="反馈处理时机"
          disabled={locked}
          value={pending?.submission.timing ?? timing}
          onChange={(event) =>
            setTiming(event.target.value as AssetSubmission["timing"])
          }
        >
          <option value="now">现在调整 · 等待安全切换点</option>
          <option value="afterRound">完成后处理 · 当前制作轮次结束后</option>
        </select>
      </label>
      <p className="asset-caption">
        {finished
          ? "当前任务已交付。新要求将创建关联后续任务，保留独立交付与回退边界。"
          : "修改串行执行。任务正在等待回答时，选项答案仍从“对话与决策”提交。"}
      </p>
      <div className="asset-toolbar">
        <button
          className="primary"
          disabled={
            busy ||
            blocked ||
            !!draftError ||
            !!cleanupError ||
            (!pending && (!text.trim() || (!reference && !canCapture)))
          }
          onClick={send}
        >
          {busy ? "处理中…" : pending ? "重试同一反馈" : "提交修改"}
        </button>
        {pending && !cleanupError && (
          <button
            disabled={busy}
            onClick={() => {
              setBusy(true);
              void run(async () => {
                await call("assetTask.cancel", {
                  id,
                  feedbackId: pending.submission.feedbackId,
                });
                reset();
              }, "该提交已撤回，可以重新编辑。").finally(() => setBusy(false));
            }}
          >
            撤回这次提交
          </button>
        )}
      </div>
      {receipt && (
        <p className="asset-notice" role="status">
          反馈 {receipt.id} 已保存，归入第 {receipt.round} 轮。
          {receipt.taskId !== id && (
            <button
              onClick={() =>
                void run(() => call("assetTask.open", { id: receipt.taskId }))
              }
            >
              打开后续任务制作窗口
            </button>
          )}
        </p>
      )}
    </section>
  );
}

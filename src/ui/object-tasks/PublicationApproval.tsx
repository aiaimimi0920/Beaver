import { useCallback, useState, useSyncExternalStore } from "react";
import type { ObjectPublication } from "./object-publication";
import { ObjectPublicationFeedback } from "./ObjectPublicationFeedback";
import { FinalRelocationEditor } from "./FinalRelocationEditor";
import { matchesFinalRelocationReceipt } from "./final-relocation-draft";
import type { FinalRelocation } from "../../shared/preview-feedback-relocation";
import type {
  PublicationDecision,
  PublicationPreview,
} from "../../shared/object-publication";
export function PublicationApproval({
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
  const [relocations, setRelocations] = useState<
    Record<string, FinalRelocation | undefined>
  >({});
  const onConfirmed = useCallback(
    (requestId: string, confirmation: FinalRelocation | undefined) => {
      setRelocations((previous) => ({
        ...previous,
        [requestId]: confirmation,
      }));
    },
    [],
  );
  const setDecisions = (next: typeof decisions) =>
    draft.edit({ decisions: next });
  const complete = preview.feedback.every(
    (item) =>
      decisions[item.requestId]?.resolution &&
      decisions[item.requestId]?.note.trim() &&
      (!item.relocationRequirement ||
        matchesFinalRelocationReceipt(
          decisions[item.requestId]?.finalRelocation,
          relocations[item.requestId],
        )),
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
        接受说明、反馈处置与逐区映射按当前发布摘要保存在本机；刷新或重开可恢复，摘要变化后需重新填写。文件归属、替换授权和最终区域确认不会恢复，提交前请重新核对。
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
              {item.relocationRequirement ? (
                <FinalRelocationEditor
                  key={
                    preview.digest +
                    item.requestId +
                    item.relocationRequirement.sourceDigest
                  }
                  session={session}
                  preview={preview}
                  feedback={item}
                  draft={decision.finalRelocation}
                  disabled={busy || saved.blocked}
                  onConfirmed={onConfirmed}
                  onChange={(finalRelocation) =>
                    setDecisions({
                      ...decisions,
                      [item.requestId]: { ...decision, finalRelocation },
                    })
                  }
                />
              ) : (
                <ObjectPublicationFeedback session={session} feedback={item} />
              )}
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
                        ...(item.relocationRequirement
                          ? { finalRelocation: relocations[item.requestId] }
                          : {}),
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

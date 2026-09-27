import { useSyncExternalStore } from "react";
import type { ObjectTaskExecution } from "./object-task-execution";
import type { ObjectPublication } from "./object-publication";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";
import { AttemptFramePicker } from "./AttemptFramePicker";
import {
  confirmedRelocation,
  type RelocationDraft,
} from "./feedback-relocation-draft";

export function FeedbackRelocationEditor({
  session,
  publication,
  draft,
  source,
  target,
  onChange,
  onLoaded,
  disabled,
}: {
  session: ObjectTaskExecution;
  publication: ObjectPublication;
  draft?: RelocationDraft;
  source?: SavedPreviewFrame;
  target?: SavedPreviewFrame;
  onChange: (draft: RelocationDraft | undefined) => void;
  onLoaded: (frame: SavedPreviewFrame | undefined) => void;
  disabled: boolean;
}) {
  const { executions } = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const choices = executions.filter(
    ({ attempt }) =>
      attempt.outputCaptured &&
      attempt.target.attemptId !== publication.review.target.attemptId,
  );
  const regions =
    source?.source.target &&
    "attemptId" in source.source.target &&
    source.source.target.attemptId === draft?.sourceAttemptId &&
    source.id === draft.sourceFrame?.frameId &&
    source.runId === draft.sourceFrame.runId
      ? (source.selection?.regions ?? [])
      : [];
  const mapped = regions.map(
    (_, sourceRegion) =>
      draft?.regions[sourceRegion] ?? {
        status: "pending" as const,
        sourceRegion,
      },
  );
  const canConfirm =
    !!draft &&
    !!confirmedRelocation(
      { ...draft, regions: mapped, confirmed: true },
      source,
      target,
    );
  return (
    <fieldset disabled={disabled}>
      <legend>历史区域重新定位</legend>
      <label>
        <input
          type="checkbox"
          checked={!!draft}
          onChange={(event) =>
            onChange(
              event.target.checked
                ? {
                    sourceAttemptId: "",
                    regions: [],
                    confirmed: false,
                  }
                : undefined,
            )
          }
        />
        将同轮次旧尝试区域对应到当前帧
      </label>
      {draft && (
        <>
          <p>
            先为当前候选选择编号帧，再逐区确认历史帧的对应位置；无法对应时填写原因。映射只适用于当前反馈检查点，后续候选或发布版本仍须核对位置。
          </p>
          <label>
            历史来源尝试
            <select
              value={draft.sourceAttemptId}
              onChange={(event) =>
                onChange({
                  sourceAttemptId: event.target.value,
                  regions: [],
                  confirmed: false,
                })
              }
            >
              <option value="">选择旧尝试</option>
              {draft.sourceAttemptId &&
                !choices.some(
                  (item) =>
                    item.attempt.target.attemptId === draft.sourceAttemptId,
                ) && (
                  <option value={draft.sourceAttemptId}>
                    {draft.sourceAttemptId}（保留原引用）
                  </option>
                )}
              {choices.map(({ attempt, definition }) => (
                <option
                  key={attempt.target.attemptId}
                  value={attempt.target.attemptId}
                >
                  {definition.title} · {attempt.target.attemptId}
                </option>
              ))}
            </select>
          </label>
          {draft.sourceAttemptId && (
            <AttemptFramePicker
              session={publication}
              attemptId={draft.sourceAttemptId}
              reference={draft.sourceFrame}
              label="历史来源编号帧"
              disabled={disabled}
              onLoaded={onLoaded}
              onChange={(sourceFrame) =>
                onChange({
                  ...draft,
                  sourceFrame,
                  regions: [],
                  confirmed: false,
                })
              }
            />
          )}
          {regions.map((region, index) => {
            const decision = mapped[index]!;
            const change = (next: RelocationDraft["regions"][number]) =>
              onChange({
                ...draft,
                confirmed: false,
                regions: mapped.map((value, i) => (i === index ? next : value)),
              });
            return (
              <div key={index}>
                <label>
                  历史区域 {index + 1} 对应
                  <select
                    disabled={disabled || !target}
                    value={
                      decision.status === "matched"
                        ? String(decision.targetRegion)
                        : decision.status
                    }
                    onChange={(event) =>
                      change(
                        event.target.value === "absent"
                          ? { status: "absent", sourceRegion: index, note: "" }
                          : event.target.value === "pending"
                            ? { status: "pending", sourceRegion: index }
                            : {
                                status: "matched",
                                sourceRegion: index,
                                targetRegion: Number(event.target.value),
                              },
                      )
                    }
                  >
                    <option value="pending">待确认</option>
                    {(target?.selection?.regions ?? []).map((_, i) => (
                      <option key={i} value={i}>
                        当前区域 {i + 1}
                      </option>
                    ))}
                    <option value="absent">无对应区域</option>
                  </select>
                </label>
                <p>历史意见：{region.prompt || "使用整体修改意见"}</p>
                {decision.status === "absent" && (
                  <label>
                    历史区域 {index + 1} 无对应原因
                    <textarea
                      value={decision.note}
                      maxLength={1000}
                      onChange={(event) =>
                        change({ ...decision, note: event.target.value })
                      }
                    />
                  </label>
                )}
              </div>
            );
          })}
          <label>
            <input
              type="checkbox"
              checked={draft.confirmed && canConfirm}
              disabled={disabled || !canConfirm}
              onChange={(event) =>
                onChange({
                  ...draft,
                  regions: mapped,
                  confirmed: event.target.checked,
                })
              }
            />
            已逐区核对历史来源和当前目标
          </label>
          {(!canConfirm || !draft.confirmed) && (
            <p role="status">
              两张原图须读取成功，每个历史区域均须指定对应位置或填写无对应原因，并确认后才能提交。
            </p>
          )}
        </>
      )}
    </fieldset>
  );
}

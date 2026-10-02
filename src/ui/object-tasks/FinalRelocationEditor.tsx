import { useEffect, useState } from "react";
import type { PublicationPreview } from "../../shared/object-publication";
import type { FinalRelocation } from "../../shared/preview-feedback-relocation";
import type { SavedPreviewFrame } from "../object-preview/saved-preview-frames";
import type { ObjectPublication } from "./object-publication";
import { ObjectPublicationFeedback } from "./ObjectPublicationFeedback";
import { AttemptFramePicker } from "./AttemptFramePicker";
import type { FinalRelocationSource } from "./final-relocation-source";
import {
  confirmedFinalRelocation,
  type FinalRelocationDraft,
} from "./final-relocation-draft";

export function FinalRelocationEditor({
  session,
  preview,
  feedback,
  draft,
  onChange,
  onConfirmed,
  disabled,
}: {
  session: ObjectPublication;
  preview: PublicationPreview;
  feedback: PublicationPreview["feedback"][number];
  draft?: FinalRelocationDraft;
  onChange: (draft: FinalRelocationDraft) => void;
  onConfirmed: (
    requestId: string,
    confirmation: FinalRelocation | undefined,
  ) => void;
  disabled: boolean;
}) {
  const [source, setSource] = useState<FinalRelocationSource>();
  const [target, setTarget] = useState<SavedPreviewFrame>();
  const required = feedback.relocationRequirement!;
  const current: FinalRelocationDraft = draft ?? {
    sourceDigest: required.sourceDigest,
    regions: [],
    confirmed: false,
  };
  const mapped = Array.from(
    { length: required.regionCount },
    (_, sourceRegion) =>
      current.regions[sourceRegion] ?? {
        status: "pending" as const,
        sourceRegion,
      },
  );
  const canConfirm = !!confirmedFinalRelocation(
    preview,
    feedback,
    { ...current, regions: mapped, confirmed: true },
    source,
    target,
  );
  const confirmation = confirmedFinalRelocation(
    preview,
    feedback,
    current,
    source,
    target,
  );
  const confirmationKey = JSON.stringify(confirmation);
  useEffect(() => {
    onConfirmed(feedback.requestId, confirmation);
    return () => onConfirmed(feedback.requestId, undefined);
    // Identity is the exact serialized receipt, not a newly allocated draft object.
  }, [onConfirmed, confirmationKey, feedback.requestId]);
  const changeRegion = (
    index: number,
    next: FinalRelocationDraft["regions"][number],
  ) =>
    onChange({
      ...current,
      confirmed: false,
      regions: mapped.map((value, i) => (i === index ? next : value)),
    });
  return (
    <fieldset disabled={disabled}>
      <legend>最终候选逐区重新确认</legend>
      <p>
        历史反馈检查点的映射不能代替最终确认。读取原始冻结
        PNG（独立图片或存档编号帧）与本次最终输出编号帧，逐区指定对应位置；无对应位置必须说明原因。此处不会启动引擎或自动发布。
      </p>
      <ObjectPublicationFeedback
        session={session}
        feedback={feedback}
        onLoaded={setSource}
        onImageLoaded={setSource}
      />
      <AttemptFramePicker
        session={session}
        attemptId={preview.review.target.attemptId}
        reference={current.targetFrame}
        label="最终候选输出编号帧"
        disabled={disabled}
        onLoaded={setTarget}
        onChange={(targetFrame) =>
          onChange({ ...current, targetFrame, regions: [], confirmed: false })
        }
      />
      {mapped.map((decision, index) => (
        <div key={index}>
          <label>
            原反馈区域 {index + 1} 对应
            <select
              disabled={disabled || !source || !target}
              value={
                decision.status === "matched"
                  ? String(decision.targetRegion)
                  : decision.status
              }
              onChange={(event) =>
                changeRegion(
                  index,
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
                  最终区域 {i + 1}
                </option>
              ))}
              <option value="absent">无对应区域</option>
            </select>
          </label>
          <p>
            原意见：
            {(source &&
              ("kind" in source
                ? source.image.regions
                : source.selection?.regions)?.[index]?.prompt) ||
              "使用整体修改意见"}
          </p>
          {decision.status === "absent" && (
            <label>
              原反馈区域 {index + 1} 无对应原因
              <textarea
                value={decision.note}
                maxLength={1000}
                onChange={(event) =>
                  changeRegion(index, { ...decision, note: event.target.value })
                }
              />
            </label>
          )}
        </div>
      ))}
      <label>
        <input
          type="checkbox"
          checked={current.confirmed && canConfirm}
          disabled={disabled || !canConfirm}
          onChange={(event) =>
            onChange({
              ...current,
              regions: mapped,
              confirmed: event.target.checked,
            })
          }
        />
        已逐区核对原反馈与最终候选输出
      </label>
      {!confirmation && (
        <p role="status">
          两张原 PNG
          必须读取成功，每个原区域均须对应或说明无对应原因，并重新确认后才能接受发布。
        </p>
      )}
    </fieldset>
  );
}

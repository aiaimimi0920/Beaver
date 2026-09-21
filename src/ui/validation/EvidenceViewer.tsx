import { useRef, useState } from "react";
import { assetUrl } from "../api";
import { Field } from "../components";
import { Icon } from "../Icon";
import type { Evidence, Selection, ValidationRun } from "./types";
import type { Region } from "./flow";

function Media({
  runId,
  item,
  videoRef,
  seek,
}: {
  runId: string;
  item: Evidence;
  videoRef?: React.RefObject<HTMLVideoElement | null>;
  seek?: (time: number) => void;
}) {
  const [error, setError] = useState(false);
  const url = assetUrl("validation", `${runId}/${item.id}`);
  return (
    <>
      {item.kind === "video" ? (
        <video
          key={url}
          ref={videoRef}
          controls
          preload="metadata"
          src={url}
          onTimeUpdate={(e) => seek?.(e.currentTarget.currentTime)}
          onError={() => setError(true)}
        />
      ) : (
        <img
          src={url}
          alt={`采集点 ${item.point}`}
          draggable={false}
          onError={() => setError(true)}
        />
      )}
      {error && (
        <p className="validation-error">
          媒体无法读取，请检查证据完整性或重新运行。
        </p>
      )}
    </>
  );
}

export function EvidenceViewer({
  run,
  selection,
  change,
}: {
  run: ValidationRun;
  selection: Selection;
  change: (value: Selection) => void;
}) {
  const item = run.evidence.find((e) => e.id === selection.evidenceId);
  const video = useRef<HTMLVideoElement>(null);
  const start = useRef<[number, number] | null>(null);
  const [cursor, setCursor] = useState(0);
  const [annotate, setAnnotate] = useState(false);
  const [compare, setCompare] = useState(false);
  const baseline = run.baseline;
  const previous = baseline?.run.evidence.find(
    (e) => e.point === item?.point && e.kind === item?.kind,
  );
  function select(evidence: Evidence) {
    change({
      evidenceId: evidence.id,
      ...(evidence.kind === "video"
        ? { range: [evidence.start, evidence.end] as [number, number] }
        : {}),
    });
    setCursor(evidence.start);
  }
  if (!run.evidence.length)
    return (
      <p className="validation-notice">
        尚未产生画面。运行中可从历史列表查看上次结果。
      </p>
    );
  return (
    <section className="validation-evidence">
      <Field label="采集点 / 视频">
        <select
          value={item?.id ?? ""}
          onChange={(e) => {
            const next = run.evidence.find((v) => v.id === e.target.value);
            if (next) select(next);
          }}
        >
          <option value="" disabled>
            选择画面
          </option>
          {run.evidence.map((e) => (
            <option key={e.id} value={e.id}>
              {e.kind === "video" ? "视频" : "截图"} · {e.point} ·{" "}
              {e.start.toFixed(2)} s
            </option>
          ))}
        </select>
      </Field>
      {item && (
        <>
          <div className="validation-toolbar">
            <button aria-pressed={compare} onClick={() => setCompare(!compare)}>
              <Icon name="layers" />
              基准对比
            </button>
            {item.kind === "image" && (
              <button
                aria-pressed={annotate}
                onClick={() => {
                  setAnnotate(!annotate);
                  start.current = null;
                }}
              >
                <Icon name="maximize" />
                框选
              </button>
            )}
            {annotate && item.kind === "image" && (
              <span className="muted">
                拖动画面标记，再点击下方“反馈选中区域”
              </span>
            )}
            {selection.region && (
              <button
                onClick={() => change({ ...selection, region: undefined })}
              >
                清除框选
              </button>
            )}
          </div>
          <div
            className={`validation-media-grid${compare && previous ? " comparing" : ""}`}
          >
            <figure>
              <figcaption>本次画面 · {run.snapshotId.slice(0, 10)}</figcaption>
              <div
                className={
                  annotate
                    ? "validation-canvas annotating"
                    : "validation-canvas"
                }
                onPointerDown={(e) => {
                  if (!annotate || item.kind !== "image") return;
                  const box = e.currentTarget.getBoundingClientRect();
                  start.current = [
                    (e.clientX - box.left) / box.width,
                    (e.clientY - box.top) / box.height,
                  ];
                  e.currentTarget.setPointerCapture(e.pointerId);
                }}
                onPointerUp={(e) => {
                  if (!start.current) return;
                  const box = e.currentTarget.getBoundingClientRect();
                  const end: [number, number] = [
                    Math.max(
                      0,
                      Math.min(1, (e.clientX - box.left) / box.width),
                    ),
                    Math.max(
                      0,
                      Math.min(1, (e.clientY - box.top) / box.height),
                    ),
                  ];
                  const [x, y] = start.current;
                  start.current = null;
                  const region: Region = [
                    Math.min(x, end[0]),
                    Math.min(y, end[1]),
                    Math.abs(x - end[0]),
                    Math.abs(y - end[1]),
                  ];
                  if (region[2] > 0.005 && region[3] > 0.005)
                    change({ ...selection, region });
                }}
                onPointerCancel={() => {
                  start.current = null;
                }}
              >
                <Media
                  key={item.id}
                  runId={run.id}
                  item={item}
                  videoRef={video}
                  seek={setCursor}
                />
                {selection.region && (
                  <span
                    className="validation-region"
                    style={{
                      left: `${selection.region[0] * 100}%`,
                      top: `${selection.region[1] * 100}%`,
                      width: `${selection.region[2] * 100}%`,
                      height: `${selection.region[3] * 100}%`,
                    }}
                  />
                )}
              </div>
            </figure>
            {compare && previous && baseline && (
              <figure>
                <figcaption>
                  用户认可 ·{" "}
                  {new Date(baseline.record.confirmedAt).toLocaleString()} ·{" "}
                  {baseline.record.snapshotId.slice(0, 10)}
                </figcaption>
                <Media
                  key={previous.id}
                  runId={baseline.run.id}
                  item={previous}
                />
                {baseline.integrityError && (
                  <p className="validation-error">{baseline.integrityError}</p>
                )}
              </figure>
            )}
          </div>
          {compare && !previous && (
            <p className="validation-notice">
              {run.baselineError ??
                "没有可对应的用户认可画面；本次完整结果可由用户首次确认。"}
            </p>
          )}
          {item.kind === "video" && (
            <div className="validation-toolbar">
              <span>播放位置 {cursor.toFixed(2)} s</span>
              <button
                onClick={() =>
                  change({
                    ...selection,
                    range: [
                      cursor,
                      Math.max(cursor, selection.range?.[1] ?? item.end),
                    ],
                  })
                }
              >
                设为反馈起点
              </button>
              <button
                onClick={() =>
                  change({
                    ...selection,
                    range: [
                      Math.min(cursor, selection.range?.[0] ?? item.start),
                      cursor,
                    ],
                  })
                }
              >
                设为反馈终点
              </button>
            </div>
          )}
          <div className="validation-keyframes" aria-label="事件关键帧">
            {run.evidence
              .filter((e) => e.kind === "image")
              .map((e) => (
                <button
                  key={e.id}
                  title={e.point}
                  onClick={() => {
                    if (item.kind === "video" && video.current) {
                      video.current.currentTime = e.start;
                      video.current.pause();
                      setCursor(e.start);
                    } else select(e);
                  }}
                >
                  <img
                    loading="lazy"
                    src={assetUrl("validation", `${run.id}/${e.id}`)}
                    alt={e.point}
                  />
                  <span>
                    {e.start.toFixed(2)} s · {e.point}
                  </span>
                </button>
              ))}
          </div>
          <details>
            <summary>本画面的状态与事件</summary>
            <pre>{JSON.stringify(item.state, null, 2)}</pre>
          </details>
        </>
      )}
    </section>
  );
}

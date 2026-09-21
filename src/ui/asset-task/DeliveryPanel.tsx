import { useEffect, useRef, useState } from "react";
import type { AssetTaskState } from "../../shared/asset-task";
import type {
  DeliveryDecision,
  DeliveryState,
} from "../../shared/asset-delivery";
import { call, type Run } from "../api";
import { errorMessage } from "../notification-state";
import { DeliveryFiles } from "./DeliveryFiles";
import { WorkPanel } from "./WorkPanel";
import { FrameworkPanel } from "./FrameworkPanel";
import "./asset-delivery.css";

export function DeliveryPanel({
  id,
  asset,
  status,
  run,
}: {
  id: string;
  asset: AssetTaskState;
  status: string;
  run: Run;
}) {
  const [data, setData] = useState<DeliveryState | null>(null);
  const [selected, setSelected] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = useRef({ signature: "", id: "" });
  const mounted = useRef(false);
  const sequence = useRef(0);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    let disposed = false;
    const read = ++sequence.current;
    call<DeliveryState>("assetTask.deliveryState", { id })
      .then((next) => {
        if (!disposed && read === sequence.current) {
          setData(next);
          setError("");
        }
      })
      .catch((failure: unknown) => {
        if (!disposed && read === sequence.current)
          setError(errorMessage(failure));
      });
    return () => {
      disposed = true;
    };
  }, [id, asset.revision]);
  if (!asset.delivery)
    return (
      <section>
        <p>
          此任务尚未启用分阶段审批。需要逐步交付时，请在制作前让 Codex 用
          beaver_task 的 deliveryPlan 定义阶段。
        </p>
        <FrameworkPanel
          key={id}
          id={id}
          status={status}
          work={asset.work}
          run={run}
        />
      </section>
    );
  if (!data || error)
    return <p role="status">{error || "正在读取候选版本…"}</p>;
  const flow = data.workflow;
  const candidate =
    data.candidates.find((item) => item.id === selected) ?? data.candidates[0];
  const current = asset.stages[flow?.approved.length ?? 0];
  const pending = !!candidate && flow?.pending === candidate.id;
  const mayReview =
    ["awaitingInput", "interrupted", "failed"].includes(status) &&
    data.revision === asset.revision &&
    !busy;
  const reopen =
    candidate && !flow?.pending && flow?.approved.includes(candidate.id);
  const decide = (decision: DeliveryDecision) => {
    if (!candidate || !mayReview) return;
    const payload = {
      id,
      expectedRevision: data.revision,
      candidateId: candidate.id,
      decision,
      note,
    };
    const signature = JSON.stringify(payload);
    if (request.current.signature !== signature)
      request.current = { signature, id: crypto.randomUUID() };
    setBusy(true);
    sequence.current += 1;
    const requestId = request.current.id;
    void run(
      async () => {
        await call("assetTask.deliveryDecide", {
          ...payload,
          requestId,
        });
        if (!mounted.current) return;
        setNote("");
        const read = ++sequence.current;
        const next = await call<DeliveryState>("assetTask.deliveryState", {
          id,
        });
        if (mounted.current && read === sequence.current) {
          setData(next);
          setError("");
        }
      },
      decision === "approve"
        ? "已批准，任务将继续"
        : "已登记修改要求，任务将继续",
    ).finally(() => {
      if (mounted.current) setBusy(false);
    });
  };
  return (
    <section className="delivery-panel">
      <h2>阶段交付</h2>
      <p>
        {flow?.templateId} · 模板版本 {flow?.templateVersion} · 已批准{" "}
        {flow?.approved.length} / {asset.stages.length}
      </p>
      <p role="status">
        {flow?.pending
          ? "等待批准候选文件"
          : current
            ? `当前阶段：${current.name}`
            : "所有阶段已批准，等待任务完成检查与合入"}
      </p>
      <WorkPanel work={data.work} currentStage={current?.id} />
      {candidate ? (
        <>
          <label>
            候选历史
            <select
              value={candidate.id}
              disabled={busy}
              onChange={(event) => {
                setSelected(event.target.value);
                setNote("");
              }}
            >
              {data.candidates.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.stageId} ·{" "}
                  {flow?.pending === item.id
                    ? "待审批"
                    : flow?.approved.includes(item.id)
                      ? "已批准"
                      : "历史版本"}{" "}
                  · {item.createdAt} · {item.id.slice(0, 8)}
                </option>
              ))}
            </select>
          </label>
          <p>{candidate.summary}</p>
          <details>
            <summary>输入与执行身份</summary>
            <p>
              输入候选：{candidate.inputCandidates.join(", ") || "无上游候选"}
            </p>
            <p>候选：{candidate.id}</p>
            <p>阶段：{candidate.stageId}</p>
            <p>
              绑定的执行尝试：
              {candidate.attemptIds?.join(", ") || "未登记子任务"}
            </p>
            <p>
              Thread：{candidate.threadId} / Turn：{candidate.turnId}
            </p>
            <p>Blender 会话：{candidate.sessionId || "未记录"}</p>
          </details>
          <DeliveryFiles key={candidate.id} candidate={candidate} run={run} />
          {(pending || reopen) && (
            <>
              <label>
                修改意见
                <textarea
                  value={note}
                  disabled={busy}
                  maxLength={4000}
                  onChange={(event) => setNote(event.target.value)}
                  placeholder="退回或返工时必须填写"
                />
              </label>
              <div className="asset-toolbar">
                {pending && (
                  <>
                    <button
                      disabled={!mayReview}
                      onClick={() => decide("approve")}
                    >
                      批准并继续
                    </button>
                    <button
                      disabled={!mayReview || !note.trim()}
                      onClick={() => decide("reject")}
                    >
                      退回修改
                    </button>
                  </>
                )}
                {reopen && (
                  <button
                    disabled={!mayReview || !note.trim()}
                    onClick={() => decide("reopen")}
                  >
                    从此阶段返工
                  </button>
                )}
              </div>
              <p>
                批准会核验文件哈希并继续任务。返工会使后续阶段的批准失效，旧候选仍保留；执行中请先中止任务。
              </p>
            </>
          )}
          {data.decisions
            .filter((record) => record.request.candidateId === candidate.id)
            .map((record, index) => (
              <p key={index}>
                用户决定：{record.request.decision} · {record.time}
                <br />
                {record.request.note}
              </p>
            ))}
        </>
      ) : (
        <p>等待 Codex 提交本阶段的实际文件。</p>
      )}
      <FrameworkPanel
        key={id}
        id={id}
        status={status}
        candidate={candidate}
        work={data.work}
        run={run}
      />
    </section>
  );
}

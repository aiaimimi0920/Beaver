import { useCallback, useState } from "react";
import type {
  AssetAnnotation,
  AssetFrame,
  AssetReference,
  Point,
} from "../../shared/asset-task";
import { call, statusNames } from "../api";
import { TaskConversation } from "../TaskConversation";
import { AssetPreview } from "./AssetPreview";
import { AssetProgress } from "./AssetProgress";
import { AssetTitlebar } from "./AssetTitlebar";
import { FeedbackComposer } from "./FeedbackComposer";
import { FeedbackHistory } from "./FeedbackHistory";
import { DeliveryPanel } from "./DeliveryPanel";
import { useAssetState } from "./use-asset-state";
import { useObserver } from "./use-observer";
import "./asset-task-window.css";
import "./asset-preview.css";

const tabs = {
  feedback: "制作与修改",
  conversation: "对话与决策",
  history: "处理记录",
  delivery: "阶段交付",
} as const;

export function AssetTaskWindow({ id }: { id: string }) {
  const { state, error, run } = useAssetState(id);
  const finished =
    state?.task.status === "completed" || state?.task.status === "rolledBack";
  const observer = useObserver(id, finished);
  const [tab, setTab] = useState<keyof typeof tabs>("feedback");
  const [conversationId, setConversationId] = useState(id);
  const [reference, setReference] = useState<AssetReference | null>(null);
  const [annotations, setAnnotations] = useState<AssetAnnotation[]>([]);
  const [locked, setLocked] = useState(false);
  const [selecting, setSelecting] = useState(false);
  const lock = useCallback((value: boolean) => setLocked(value), []);
  const { display, status } = observer;
  const canCapture = finished
    ? !!state?.asset.lastFrame
    : !!display &&
      !!status?.connected &&
      display.frame.sessionId === status.sessionId &&
      display.frame.generation === status.generation;
  const freezeFrame = (frame: AssetFrame) =>
    call<AssetReference>("assetTask.freeze", {
      id,
      frameId: frame.id,
      sessionId: frame.sessionId,
    });
  const capture = async () => {
    if (finished && state?.asset.lastFrame) return state.asset.lastFrame;
    if (!canCapture || !display)
      throw new Error("没有可定格的当前画面，请等待观察连接恢复。");
    return freezeFrame(display.frame);
  };
  const freeze = () => {
    if (locked || selecting) return;
    setSelecting(true);
    void run(async () => {
      setReference(await capture());
      setAnnotations([]);
      setTab("feedback");
    }).finally(() => setSelecting(false));
  };
  const pick = (frame: AssetFrame, point: Point) => {
    if (locked || selecting) return;
    setSelecting(true);
    void run(async () => {
      const fixed = await freezeFrame(frame);
      const selected = await call<AssetReference>("assetTask.pick", {
        id,
        referenceId: fixed.id,
        point,
      });
      setReference(selected);
      setAnnotations([]);
      setTab("feedback");
    }).finally(() => setSelecting(false));
  };
  const openConversation = (target: string) => {
    setConversationId(target);
    setTab("conversation");
  };
  const conversation =
    state?.tasks.find((task) => task.id === conversationId) ?? state?.task;
  return (
    <main className="asset-task-window">
      <AssetTitlebar
        title={
          state
            ? `${state.project.name} · ${state.task.title} · 资产制作`
            : "Beaver · 资产制作"
        }
      />
      {state ? (
        <>
          <header className="asset-task-context">
            <div>
              <strong>{state.task.title}</strong>
              <span>{statusNames[state.task.status]}</span>
              <small>绑定任务：{id}</small>
            </div>
            <div className="asset-toolbar">
              {state.task.parentTaskId && (
                <button
                  onClick={() => openConversation(state.task.parentTaskId!)}
                >
                  查看关联任务对话
                </button>
              )}
              <button onClick={() => openConversation(id)}>当前任务对话</button>
            </div>
          </header>
          {error && (
            <p className="asset-notice" role="status">
              {error} · 显示最后读取的任务记录。
            </p>
          )}
          <div className="asset-task-content">
            <AssetPreview
              id={id}
              observer={observer}
              finished={finished}
              saved={state.asset.lastFrame}
              locked={locked || selecting}
              freeze={freeze}
              pick={pick}
            />
            <aside className="asset-task-side">
              <nav className="asset-task-tabs" aria-label="制作窗口内容">
                {Object.entries(tabs).map(([value, label]) => (
                  <button
                    key={value}
                    aria-pressed={tab === value}
                    onClick={() => setTab(value as keyof typeof tabs)}
                  >
                    {label}
                    {value === "history" &&
                      ` (${state.asset.feedback.filter((feedback) => !["completed", "cancelled", "forwarded"].includes(feedback.status)).length})`}
                  </button>
                ))}
              </nav>
              <div
                hidden={tab !== "feedback"}
                className="asset-task-panel asset-task-scroll"
              >
                <AssetProgress asset={state.asset} />
                {state.asset.delivery ? (
                  <p className="asset-notice">
                    本任务按阶段交付。请在“阶段交付”中查看候选文件、批准或提出修改意见。
                    <button onClick={() => setTab("delivery")}>
                      查看交付{state.asset.delivery.pending ? " · 待审批" : ""}
                    </button>
                  </p>
                ) : (
                  <FeedbackComposer
                    id={id}
                    reference={reference}
                    annotations={annotations}
                    select={setReference}
                    mark={setAnnotations}
                    capture={capture}
                    canCapture={canCapture}
                    finished={finished}
                    run={run}
                    lock={lock}
                    blocked={selecting}
                  />
                )}
              </div>
              <div
                hidden={tab !== "conversation"}
                className="asset-task-panel asset-task-conversation"
              >
                {conversation && (
                  <>
                    {conversation.id !== id && (
                      <p className="asset-notice">
                        正在查看关联任务；左侧观察与修改仍绑定{" "}
                        {state.task.title}。
                        <button onClick={() => openConversation(id)}>
                          返回绑定任务
                        </button>
                      </p>
                    )}
                    <TaskConversation
                      key={conversation.id}
                      task={conversation}
                      tasks={state.tasks}
                      run={run}
                      back={() => {
                        setConversationId(id);
                        setTab("feedback");
                      }}
                      open={openConversation}
                    />
                  </>
                )}
              </div>
              <div
                hidden={tab !== "history"}
                className="asset-task-panel asset-task-scroll"
              >
                <FeedbackHistory id={id} asset={state.asset} run={run} />
              </div>
              {tab === "delivery" && (
                <div className="asset-task-panel asset-task-scroll">
                  <DeliveryPanel
                    key={id}
                    id={id}
                    asset={state.asset}
                    status={state.task.status}
                    run={run}
                  />
                </div>
              )}
            </aside>
          </div>
        </>
      ) : (
        <div className="asset-window-loading">
          <h1>资产制作窗口</h1>
          <p role="status">{error || "正在读取任务…"}</p>
        </div>
      )}
    </main>
  );
}

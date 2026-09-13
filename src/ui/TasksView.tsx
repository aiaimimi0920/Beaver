import { useEffect, useState } from "react";
import type { Task, Reference } from "../shared/types";
import { directions, type TaskDirection } from "../shared/task-board";
import { call, type Run } from "./api";
import { TaskBoard } from "./TaskBoard";
import { TaskConversation } from "./TaskConversation";
import { Field } from "./components";
import { PagedEditor } from "./PagedEditor";

const presets = [
  {
    label: "原创对话游戏",
    direction: "story",
    text: "基于当前项目扩展原创角色、对话和玩法，不复制参考作品的剧情或素材。自行选择上下文，接入 Godot 并验证。具体目标：",
  },
  {
    label: "参考生成图片",
    direction: "visual",
    text: "根据附带参考与反馈生成游戏图片，保留原素材，保持风格并接入游戏。不使用占位内容冒充成功；服务不可用时如实报告。需要生成：",
  },
  {
    label: "统一素材风格",
    direction: "visual",
    text: "检查附带素材，统一色板、线条、光照和比例，保留指定内容并验证游戏引用。需要保留或调整：",
  },
  {
    label: "生成或优化音频",
    direction: "audio",
    text: "根据参考与试听反馈生成或优化音频，处理循环、响度与游戏引用，保留原文件。服务不可用时如实报告。要求：",
  },
  {
    label: "独立审查",
    direction: "review",
    text: "只审查项目，不修改文件。报告运行错误、存读档、素材引用和缺失测试，注明位置、复现步骤与验证结果。",
  },
  {
    label: "准备游戏导出",
    direction: "release",
    text: "准备当前桌面系统的可运行导出预设，检查 Godot 及匹配导出模板，不覆盖已有设置，验证交付资源并如实报告。",
  },
] as const;

export function TasksView({
  projectId,
  tasks,
  refs,
  clearRefs,
  run,
}: {
  projectId: string;
  tasks: Task[];
  refs: Reference[];
  clearRefs: () => void;
  run: Run;
}) {
  const [selected, setSelected] = useState("");
  const [composing, setComposing] = useState(false);
  const [dismissedRefs, setDismissedRefs] = useState(false);
  useEffect(() => setDismissedRefs(false), [refs]);
  const [prompt, setPrompt] = useState("");
  const [direction, setDirection] = useState<TaskDirection>("general");
  const [limits, setLimits] = useState("");
  const [minutes, setMinutes] = useState(0);
  const [review, setReview] = useState(false);
  const [decompose, setDecompose] = useState(true);
  const [autoAccept, setAutoAccept] = useState(true);
  const [askRatio, setAskRatio] = useState<AskRatio | null>(null);
  const [busy, setBusy] = useState(false);
  const task = tasks.find((t) => t.id === selected);
  if (task)
    return (
      <TaskConversation
        key={task.id}
        task={task}
        tasks={tasks}
        run={run}
        back={() => setSelected("")}
        open={setSelected}
      />
    );
  if (composing || (refs.length > 0 && !dismissedRefs))
    return (
      <section className="new-task-screen">
        <header>
          <button
            onClick={() => {
              setComposing(false);
              setDismissedRefs(true);
            }}
          >
            ‹ 看板
          </button>
          <h1>新任务</h1>
        </header>
        <div className="field">
          <span>任务目标</span>
          <PagedEditor label="任务目标" value={prompt} change={setPrompt} />
        </div>
        <div className="new-task-settings">
          <Field label="方向">
            <select
              aria-label="新任务方向"
              value={direction}
              onChange={(e) => setDirection(e.target.value as TaskDirection)}
            >
              {Object.entries(directions).map(([id, info]) => (
                <option key={id} value={id}>
                  {info.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="提示词">
            <select
              aria-label="标准任务提示词"
              value=""
              onChange={(e) => {
                const preset = presets[Number(e.target.value)];
                if (preset) {
                  setPrompt(preset.text);
                  setDirection(preset.direction);
                  setReview(preset.direction === "review");
                }
              }}
            >
              <option value="" disabled>
                选择提示词
              </option>
              {presets.map((preset, i) => (
                <option key={preset.label} value={i}>
                  {preset.label}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <div className="new-task-settings">
          <Field label="停止条件">
            <input
              value={limits}
              onChange={(e) => setLimits(e.target.value)}
              placeholder="完成目标后停止"
            />
          </Field>
          <Field label="自动决策">
            <AutonomySelect
              label="新任务自动决策"
              inherit
              value={askRatio}
              change={setAskRatio}
            />
          </Field>
          <Field label="时限（分钟，0 为不限）">
            <input
              type="number"
              min={0}
              max={1440}
              value={minutes}
              onChange={(e) => setMinutes(Number(e.target.value))}
            />
          </Field>
        </div>
        <label className="check">
          <input
            type="checkbox"
            checked={decompose && !review}
            disabled={review}
            onChange={(e) => setDecompose(e.target.checked)}
          />
          先追问并拆分为多个子任务
        </label>
        {decompose && !review && (
          <label className="check">
            <input
              type="checkbox"
              checked={autoAccept}
              onChange={(e) => setAutoAccept(e.target.checked)}
            />
            子任务成功合入后自动审批（取消后逐项手动认可）
          </label>
        )}
        <label className="check">
          <input
            type="checkbox"
            checked={review}
            onChange={(e) => setReview(e.target.checked)}
          />
          仅审查
        </label>
        <footer>
          <span>{refs.length ? `${refs.length} 份参考` : ""}</span>
          <button
            className="primary"
            disabled={busy || !prompt.trim()}
            onClick={() => {
              setBusy(true);
              void run(async () => {
                const next = await call<Task>("task.create", {
                  projectId,
                  askRatio,
                  prompt,
                  direction,
                  references: refs,
                  stopConditions: limits,
                  maxMinutes: minutes,
                  capability: review ? "review" : "code",
                  decompose: decompose && !review,
                  autoAccept,
                });
                setSelected(next.id);
                setPrompt("");
                setComposing(false);
                clearRefs();
              }).finally(() => setBusy(false));
            }}
          >
            {busy ? "正在创建…" : "发出任务"}
          </button>
        </footer>
      </section>
    );
  return (
    <div className="task-page task-board-page">
      <TaskBoard
        tasks={tasks}
        open={setSelected}
        create={() => setComposing(true)}
      />
    </div>
  );
}
import { AutonomySelect } from "./AutonomySelect";
import type { AskRatio } from "../shared/autonomy";

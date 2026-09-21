import { useState } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import type { DemoObject } from "./mock-objects";
import {
  ManufacturePromptEditor,
  type ManufacturePromptTab,
} from "./ManufacturePromptEditor";
import { ManufactureFeedbackEditor } from "./ManufactureFeedbackEditor";
import { ManufactureStageData } from "./ManufactureStageData";
import {
  useManufactureDraft,
  type RegenerateScope,
} from "./useManufactureDraft";
import "./preview-object-contents.css";
import "./preview-selection.css";
import "./preview-annotations.css";
import "./preview-manufacture-stage.css";
import "./preview-manufacture-editor.css";
import "./preview-manufacture-dialogs.css";

export function ManufactureStageWorkspace({
  object,
  stage,
  title,
  version,
  taskTitle,
  pending,
  stale,
  regenerated,
}: {
  object: DemoObject;
  stage: number;
  title: string;
  version: string;
  taskTitle?: string;
  pending: boolean;
  stale?: string;
  regenerated: (scope: RegenerateScope) => void;
}) {
  const session = useManufactureDraft(
    object,
    version,
    stage,
    title,
    pending,
    taskTitle,
  );
  const [tab, setTab] = useState<ManufacturePromptTab>("creation");
  const [confirmation, setConfirmation] = useState<string | null>(null);
  const [scope, setScope] = useState<RegenerateScope>("stage");
  const key = `${version}:${stage}`;
  const triangleLimit = Number(session.draft.checks.triangleLimit);
  const valid =
    !!session.draft.prompts.creation.trim() &&
    (!session.draft.checks.geometry ||
      (Number.isSafeInteger(triangleLimit) && triangleLimit > 0));

  return (
    <main className="op-manufacture-stage">
      <section className="op-manufacture-prompts" aria-label="阶段提示词">
        <header>
          <div>
            <h2>{title}</h2>
            <small>
              基于 {version} {stale && `· ${stale}`} · UI 预览
            </small>
          </div>
          <div className="op-manufacture-rounds">
            <select
              aria-label="查看生成轮次"
              value={session.viewed}
              disabled={!session.rounds.length}
              onChange={(event) => session.view(Number(event.target.value))}
            >
              {!session.rounds.length && <option value={0}>尚未生成</option>}
              {session.rounds.map((round) => (
                <option key={round.number} value={round.number}>
                  第 {round.number} 轮
                  {round.number === session.rounds.at(-1)?.number
                    ? " · 最新"
                    : " · 历史"}
                </option>
              ))}
            </select>
            <button
              disabled={!session.round}
              onClick={session.restore}
              title="将所选轮次的提示词、检查配置及标注载入当前草稿"
            >
              载入本轮输入
            </button>
          </div>
        </header>
        <ManufacturePromptEditor
          session={session}
          tab={tab}
          selectTab={setTab}
          model={object.objectType === "模型"}
        />
        <ManufactureFeedbackEditor session={session} />
        <div className="op-manufacture-actions">
          <small role="status">
            {session.notice || "编辑阶段要求，或添加补充修改后重新生成"}
          </small>
          <button onClick={session.save}>保存草稿</button>
          <button
            className="primary"
            disabled={!valid}
            onClick={() => {
              setScope("stage");
              setConfirmation(key);
            }}
          >
            <Icon name="refresh" />
            {session.round ? "重新生成" : "生成本阶段"}
          </button>
        </div>
      </section>
      <ManufactureStageData
        key={key}
        object={object}
        version={version}
        stage={stage}
        session={session}
        configure={() => setTab("acceptance")}
      />
      {confirmation === key && (
        <Dialog
          title={session.round ? "重新生成本阶段" : "生成本阶段"}
          close={() => setConfirmation(null)}
          className="op-manufacture-regenerate-dialog"
        >
          <p>
            {object.name} / {title}
          </p>
          <small>
            制作基准 {version} ·{" "}
            {session.round ? `参考第 ${session.viewed} 轮结果` : "首次生成"} ·{" "}
            {session.draft.annotations.length} 个标记
          </small>
          <div className="op-manufacture-scope">
            <label>
              <input
                type="radio"
                name="regenerate-scope"
                checked={scope === "stage"}
                onChange={() => setScope("stage")}
              />
              <span>
                <strong>仅重做当前阶段</strong>
                <small>保留后续结果，标记为待更新，方便逐步调整。</small>
              </span>
            </label>
            <label>
              <input
                type="radio"
                name="regenerate-scope"
                checked={scope === "downstream"}
                onChange={() => setScope("downstream")}
              />
              <span>
                <strong>从当前阶段重做后续</strong>
                <small>当前阶段验收后，按顺序继续后续阶段。</small>
              </span>
            </label>
          </div>
          <p className="op-dialog-note">
            使用当前提示词、验收配置与补充修改创建新轮次。旧轮次保留；本次仅模拟界面变化，不执行真实生成。
          </p>
          <footer>
            <button onClick={() => setConfirmation(null)}>取消</button>
            <button
              className="primary"
              onClick={() => {
                session.generate(scope);
                regenerated(scope);
                setConfirmation(null);
              }}
            >
              确认生成
            </button>
          </footer>
        </Dialog>
      )}
    </main>
  );
}

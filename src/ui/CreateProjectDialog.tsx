import { useState } from "react";
import type { Feature, Project } from "../shared/types";
import {
  blueprintProblems,
  defaultBlueprint,
} from "../shared/project-blueprint";
import { call, type Run } from "./api";
import { BlueprintFields } from "./BlueprintFields";
import { PlannedFeatures } from "./PlannedFeatures";
import { Dialog, Field } from "./components";

export function CreateProjectDialog({
  close,
  created,
  features,
  run,
}: {
  close: () => void;
  created: (project: Project) => void;
  features: Feature[];
  run: Run;
}) {
  const [step, setStep] = useState(0);
  const [name, setName] = useState("");
  const [parent, setParent] = useState("");
  const [template, setTemplate] = useState("nightbar");
  const [blueprint, setBlueprint] = useState(defaultBlueprint);
  const [saving, setSaving] = useState(false);
  const [nprEnabled, setNprEnabled] = useState(false);
  const [nprGodot, setNprGodot] = useState("");
  const problems = blueprintProblems(blueprint);
  const tabs = ["基本信息", "创作方向", "功能规划"];
  return (
    <Dialog
      title="创建游戏项目"
      close={saving ? () => {} : close}
      className="blueprint-dialog"
    >
      <div className="blueprint-tabs" role="tablist" aria-label="创建步骤">
        {tabs.map((tab, i) => (
          <button
            key={tab}
            role="tab"
            id={`create-tab-${i}`}
            aria-selected={step === i}
            aria-controls={`create-panel-${i}`}
            tabIndex={step === i ? 0 : -1}
            onKeyDown={(e) => {
              if (["ArrowLeft", "ArrowRight"].includes(e.key)) {
                e.preventDefault();
                const next = (i + (e.key === "ArrowRight" ? 1 : 2)) % 3;
                setStep(next);
                document.getElementById(`create-tab-${next}`)?.focus();
              }
            }}
            onClick={() => setStep(i)}
          >
            {i + 1}. {tab}
          </button>
        ))}
      </div>
      <div
        className="blueprint-dialog-body"
        role="tabpanel"
        id={`create-panel-${step}`}
        aria-labelledby={`create-tab-${step}`}
      >
        {step === 0 ? (
          <>
            <Field label="项目名称">
              <input
                aria-label="项目名称"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="我的原创游戏"
                maxLength={80}
              />
            </Field>
            <Field label="存放目录">
              <div className="input-action">
                <input
                  aria-label="项目存放目录"
                  value={parent}
                  onChange={(e) => setParent(e.target.value)}
                />
                <button
                  onClick={() =>
                    void run(async () => {
                      const selected = await call<string | null>(
                        "chooseDirectory",
                      );
                      if (selected) setParent(selected);
                    })
                  }
                >
                  选择目录
                </button>
              </div>
            </Field>
            <Field label="游戏模板（仅创建时应用）">
              <select
                value={template}
                onChange={(e) => setTemplate(e.target.value)}
              >
                <option value="nightbar">夜航调饮室 · 原创对话调饮</option>
                <option value="blank">空白 Godot 项目</option>
              </select>
            </Field>
            <p className="muted">
              创建本地工程与规划草案；启用的 NPR 插件会实际安装，不会自动开始 AI
              创作。
            </p>
          </>
        ) : step === 1 ? (
          <BlueprintFields value={blueprint} change={setBlueprint} />
        ) : (
          <>
            {"__TAURI__" in window && (
              <section>
                <label>
                  <input
                    type="checkbox"
                    checked={nprEnabled}
                    disabled={saving}
                    onChange={(e) => setNprEnabled(e.target.checked)}
                  />
                  启用 NPR 人物模块（实际安装）
                </label>
                <p className="muted">
                  包含人物渲染插件、制作规范与 Codex
                  可调用的标准工作流。需要定制 Godot Forward+ 引擎。
                </p>
                {nprEnabled && (
                  <Field label="NPR 定制引擎目录或编辑器路径">
                    <input
                      aria-label="NPR 定制引擎路径"
                      value={nprGodot}
                      disabled={saving}
                      onChange={(e) => setNprGodot(e.target.value)}
                      placeholder="选择定制引擎的 export 目录或编辑器 EXE"
                    />
                  </Field>
                )}
              </section>
            )}
            <PlannedFeatures
              value={blueprint}
              change={setBlueprint}
              features={features}
            />
          </>
        )}
        {!!problems.length && (
          <p className="blueprint-error" role="alert">
            {problems.join("；")}
          </p>
        )}
      </div>
      <footer>
        <button disabled={saving} onClick={close}>
          取消
        </button>
        {step > 0 && (
          <button disabled={saving} onClick={() => setStep(step - 1)}>
            上一步
          </button>
        )}
        {step < 2 ? (
          <button className="primary" onClick={() => setStep(step + 1)}>
            下一步
          </button>
        ) : (
          <button
            className="primary"
            disabled={
              saving ||
              !name.trim() ||
              !parent.trim() ||
              problems.length > 0 ||
              (nprEnabled && !nprGodot.trim())
            }
            onClick={() => {
              setSaving(true);
              void run(async () => {
                const project = await call<Project>("project.create", {
                  parent,
                  name,
                  template,
                  blueprint,
                  ...(nprEnabled ? { npr: { godot: nprGodot.trim() } } : {}),
                });
                created(project);
              }).finally(() => setSaving(false));
            }}
          >
            {saving ? "创建中…" : "创建项目"}
          </button>
        )}
      </footer>
    </Dialog>
  );
}

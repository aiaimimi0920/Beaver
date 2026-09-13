import { useState } from "react";
import type { Project } from "../shared/types";
import {
  blueprintProblems,
  defaultBlueprint,
  type BlueprintDraft,
} from "../shared/project-blueprint";
import { call, type Run } from "./api";
import { Dialog } from "./components";

export function BlueprintActions({
  project,
  draft,
  clearDraft,
  run,
}: {
  project: Project;
  draft?: BlueprintDraft;
  clearDraft: () => void;
  run: Run;
}) {
  const [discard, setDiscard] = useState(false);
  const [saving, setSaving] = useState(false);
  const value =
    draft?.value ?? project.blueprint ?? defaultBlueprint(project.design);
  const problems = blueprintProblems(value);
  const stale = draft && draft.revision !== (project.blueprintRevision ?? 0);
  return (
    <>
      <div className="blueprint-toolbar">
        <span className="prototype-label">交互原型 · 仅保存规划</span>
        <span className="muted" role="status">
          {draft ? "未保存" : project.blueprint ? "规划已保存" : "尚未保存规划"}
        </span>
        <button onClick={() => setDiscard(true)} disabled={!draft || saving}>
          重新载入
        </button>
        <button
          className="primary"
          disabled={
            saving ||
            !!problems.length ||
            !!stale ||
            (!draft && !!project.blueprint)
          }
          onClick={() => {
            setSaving(true);
            void run(async () => {
              await call("project.blueprint.save", {
                id: project.id,
                blueprint: value,
                expectedRevision:
                  draft?.revision ?? project.blueprintRevision ?? 0,
              });
              clearDraft();
            }, "规划已保存，未执行业务操作").finally(() => setSaving(false));
          }}
        >
          保存规划
        </button>
      </div>
      {stale && (
        <p className="blueprint-error" role="alert">
          已保存版本发生变化。草稿保留中，请重新载入后编辑。
        </p>
      )}
      {!!problems.length && (
        <p className="blueprint-error" role="alert">
          {problems.join("；")}
        </p>
      )}
      {discard && (
        <Dialog title="重新载入规划" close={() => setDiscard(false)}>
          <p>
            放弃本项目尚未保存的规划草稿，载入最新保存版本。不会修改游戏文件。
          </p>
          <footer>
            <button onClick={() => setDiscard(false)}>取消</button>
            <button
              className="danger"
              onClick={() => {
                clearDraft();
                setDiscard(false);
              }}
            >
              放弃草稿并载入
            </button>
          </footer>
        </Dialog>
      )}
    </>
  );
}

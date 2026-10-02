import { useEffect, useState } from "react";
import { call, type Run } from "./api";
import { Dialog, Field } from "./components";
import { useNotify } from "./Notifications";
import { errorMessage } from "./notification-state";
import { ReleaseStatus } from "./validation/ReleaseStatus";
import type { ReleaseCheck } from "./validation/types";
import {
  perform,
  useValidationMutation,
  useValidationQuery,
} from "./validation/useValidation";

export function ExportGameDialog({
  projectId,
  initialReleaseId,
  busy,
  run,
  close,
  openRun,
}: {
  projectId: string;
  initialReleaseId?: string;
  busy: boolean;
  run: Run;
  close: () => void;
  openRun: (id: string) => void;
}) {
  const notify = useNotify();
  const [presets, setPresets] = useState<string[]>([]);
  const [preset, setPreset] = useState("");
  const [destination, setDestination] = useState("");
  const [loading, setLoading] = useState(true);
  const [preparingTemplates, setPreparingTemplates] = useState(false);
  const [purpose, setPurpose] = useState<"internal" | "formal">(
    initialReleaseId ? "formal" : "internal",
  );
  const [releaseId, setReleaseId] = useState(initialReleaseId);
  const {
    data: check,
    error,
    refresh,
  } = useValidationQuery<ReleaseCheck>(
    "validation.release.get",
    releaseId ? { projectId, releaseCheckId: releaseId } : null,
    (value) =>
      value.items?.some((item) =>
        ["queued", "preparing", "running"].includes(item.status),
      ) ?? false,
  );
  const { mutate, busy: mutating } = useValidationMutation(projectId, refresh);
  const working = busy || mutating;
  const exportPreset = purpose === "formal" && check ? check.preset : preset;
  useEffect(() => {
    let alive = true;
    void call<string[]>("game.presets", { id: projectId })
      .then((list) => {
        if (!alive) return;
        setPresets(list);
        setPreset(list[0] ?? "");
      })
      .catch((error: unknown) => {
        if (alive) notify({ tone: "error", text: errorMessage(error) });
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [projectId, notify]);
  function prepareTemplates(importArchive: boolean) {
    setPreparingTemplates(true);
    void run(async () => {
      try {
        const result = await call(
          importArchive ? "game.importTemplates" : "game.prepareTemplates",
        );
        if (result)
          notify({ tone: "success", text: "Windows x86_64 导出模板已就绪" });
      } finally {
        setPreparingTemplates(false);
      }
    });
  }
  return (
    <Dialog
      title="导出可运行游戏"
      className="validation-export-dialog"
      close={close}
    >
      <div className="tools-toolbar">
        <button
          disabled={working}
          title="优先校验并复用编辑器旁的本地配套模板；官方稳定版无本地配套时下载并校验模板包（可能超过 1 GB）"
          onClick={() => prepareTemplates(false)}
        >
          {preparingTemplates ? "准备模板中…" : "准备 Windows 模板"}
        </button>
        <button
          disabled={working}
          title="仅导入你信任的、与当前引擎版本匹配的 TPZ；不会覆盖已有模板"
          onClick={() => prepareTemplates(true)}
        >
          导入模板包
        </button>
        {preparingTemplates && (
          <button onClick={() => void run(() => call("game.cancelTemplates"))}>
            停止准备模板
          </button>
        )}
      </div>
      <Field label="导出用途">
        <select
          value={purpose}
          disabled={working}
          onChange={(e) => setPurpose(e.target.value as "internal" | "formal")}
        >
          <option value="internal">内部开发 / 调试验证</option>
          <option value="formal">游戏正式发布</option>
        </select>
      </Field>
      {purpose === "internal" && (
        <p className="validation-notice">
          导出当前项目供内部验证，不要求画面与代码全部通过；产物记录为内部导出，不代表正式验收。
        </p>
      )}
      {!loading && !presets.length && !(purpose === "formal" && check) && (
        <div className="warning-box">
          项目没有导出预设。请给 Codex 发出“配置当前系统的导出预设”任务。
        </div>
      )}
      <Field label="导出预设">
        <select
          value={exportPreset}
          disabled={working || loading || (purpose === "formal" && !!releaseId)}
          onChange={(e) => setPreset(e.target.value)}
        >
          {loading && <option value="">正在读取…</option>}
          {exportPreset && !presets.includes(exportPreset) && (
            <option>{exportPreset}</option>
          )}
          {presets.map((p) => (
            <option key={p}>{p}</option>
          ))}
        </select>
      </Field>
      {purpose === "formal" && (
        <section className="validation-formal-export">
          <p>
            正式发布先固定代码、资源、流程及项目的画面诊断开关，再生成本次新结果。只有该候选的必需检查全部通过才允许导出。
          </p>
          <div className="validation-toolbar">
            <button
              disabled={working || loading || !exportPreset}
              onClick={() =>
                perform(
                  mutate<{ id: string }>("validation.release.start", {
                    preset: exportPreset,
                  }).then((value) => setReleaseId(value.id)),
                )
              }
            >
              {releaseId
                ? "以当前项目建立新候选并重新诊断"
                : "建立发布候选并开始诊断"}
            </button>
            {releaseId && (
              <button
                disabled={working}
                onClick={() => {
                  setPreset(exportPreset);
                  setReleaseId(undefined);
                }}
              >
                选择其他导出预设
              </button>
            )}
            {releaseId && <button onClick={refresh}>刷新诊断</button>}
          </div>
          {error && <p className="validation-error">{error}</p>}
          {releaseId && !check && !error && (
            <p role="status">正在读取固定候选…</p>
          )}
          {check && (
            <ReleaseStatus
              check={check}
              openRun={(id) => {
                close();
                openRun(id);
              }}
            />
          )}
        </section>
      )}
      <Field label="输出父目录（项目外）">
        <div className="input-action">
          <input
            value={destination}
            onChange={(e) => setDestination(e.target.value)}
          />
          <button
            onClick={() =>
              void run(async () => {
                const p = await call<string | null>("chooseDirectory");
                if (p) setDestination(p);
              })
            }
          >
            选择
          </button>
        </div>
      </Field>
      <footer>
        <button onClick={close}>取消</button>
        <button
          disabled={working}
          onClick={() =>
            void run(async () => {
              const directory = await call<string | null>("chooseDirectory");
              if (!directory) return;
              const result = await call<{ files: number }>(
                "game.verifyExport",
                { path: directory },
              );
              notify({
                tone: "success",
                text: `导出包完整性校验通过：${result.files} 个文件；不代表运行或玩法验收。`,
              });
            })
          }
        >
          校验已有导出
        </button>
        <button
          className="primary"
          disabled={
            working ||
            !exportPreset ||
            !destination ||
            (purpose === "formal" && !check?.ready)
          }
          onClick={() =>
            void run(async () => {
              const result = await call<{ path: string }>("game.export", {
                id: projectId,
                preset: exportPreset,
                destination,
                purpose,
                ...(purpose === "formal" && check
                  ? {
                      releaseCheckId: check.id,
                      snapshotId: check.snapshotId,
                      scopeId: check.scopeId,
                    }
                  : {}),
              });
              notify({
                tone: "success",
                text: `${purpose === "formal" ? "正式游戏" : "内部验证"}导出完成：${result.path}`,
              });
              close();
            })
          }
        >
          {purpose === "formal" ? "导出已检查的固定候选" : "生成内部验证程序"}
        </button>
      </footer>
    </Dialog>
  );
}

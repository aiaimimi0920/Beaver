import { useCallback, useEffect, useState } from "react";
import type { Project, Reference, ToolStatus } from "../shared/types";
import { call, type Run, type State } from "./api";
import { Dialog, Field } from "./components";
import { TasksView } from "./TasksView";
import { AssetsView } from "./AssetsView";
import { SettingsView } from "./SettingsView";
import { DesktopShell, type Page } from "./DesktopShell";
import { Icon } from "./Icon";
import { useNotify } from "./Notifications";
import { errorMessage } from "./notification-state";
import { FeaturesView } from "./FeaturesView";
import { CreateProjectDialog } from "./CreateProjectDialog";
import { ProjectBlueprintView } from "./ProjectBlueprintView";
import type { BlueprintDraft } from "../shared/project-blueprint";
import type { OverviewDraft } from "../shared/project-overview";
import { ProjectOverviewView } from "./ProjectOverviewView";
import { StudioPreview } from "./StudioPreview";
import { DocumentsView } from "./DocumentsView";
import { TaskBoard } from "./TaskBoard";

export function App() {
  const notify = useNotify();
  const [state, setState] = useState<State>();
  const [projectId, setProjectId] = useState("");
  const [page, setPage] = useState<Page>("create");
  const [busy, setBusy] = useState(0);
  const [refs, setRefs] = useState<Reference[]>([]);
  const [create, setCreate] = useState(false);
  const [blueprintDrafts, setBlueprintDrafts] = useState<
    Record<string, BlueprintDraft>
  >({});
  const [overviewDrafts, setOverviewDrafts] = useState<
    Record<string, OverviewDraft>
  >({});
  const [overviewRisk, setOverviewRisk] = useState<{
    name: string;
    fields: string[];
  }>();
  const [exporting, setExporting] = useState(false);
  const [preparingTemplates, setPreparingTemplates] = useState(false);
  const [presets, setPresets] = useState<string[]>([]);
  const [preset, setPreset] = useState("");
  const [destination, setDestination] = useState("");
  const [missingTools, setMissingTools] = useState<ToolStatus[]>([]);
  const [setup, setSetup] = useState(false);
  const refresh = useCallback(async () => {
    const next = await call<State>("state");
    setState(next);
    setProjectId((id) =>
      next.projects.some((p) => p.id === id)
        ? id
        : (next.projects[0]?.id ?? ""),
    );
  }, []);
  const run: Run = useCallback(
    async (work, message) => {
      setBusy((n) => n + 1);
      try {
        await work();
        await refresh();
        if (message) notify({ tone: "success", text: message });
      } catch (e) {
        notify({ tone: "error", text: errorMessage(e) });
      } finally {
        setBusy((n) => n - 1);
      }
    },
    [refresh, notify],
  );
  useEffect(() => {
    const onError = (error: unknown) =>
      notify({ tone: "error", text: errorMessage(error) });
    void refresh().catch(onError);
    return window.beaver.subscribe(() => void refresh().catch(onError));
  }, [refresh, notify]);
  useEffect(() => setRefs([]), [projectId]);
  useEffect(() => {
    let active = true;
    void call<ToolStatus[]>("tools.detect")
      .then((result) => {
        if (!active) return;
        const missing = result.filter(
          (tool) => !tool.available && ["codex", "godot"].includes(tool.name),
        );
        setMissingTools(missing);
        // Preview workspaces remain usable before the local toolchain is ready.
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);
  const project = state?.projects.find((p) => p.id === projectId);
  const tasks = state?.tasks.filter((t) => t.projectId === projectId) ?? [];
  const running =
    state?.tasks.filter((t) => t.status === "running").length ?? 0;
  const closeCreate = useCallback(() => setCreate(false), []);
  const closeExport = useCallback(() => setExporting(false), []);
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
    <>
      <DesktopShell
        page={page}
        navigate={setPage}
        projects={state?.projects ?? []}
        projectId={projectId}
        selectProject={setProjectId}
        createProject={() => setCreate(true)}
        openProject={() =>
          void run(async () => {
            const dir = await call<string | null>("chooseDirectory");
            if (dir) {
              const p = await call<Project>("project.import", { path: dir });
              setProjectId(p.id);
              setPage("overview");
            }
          })
        }
        refresh={() => void run(refresh)}
        busy={busy > 0}
        running={running}
        ready={!!state}
        environment={() => setPage("environment")}
        actions={
          project &&
          page !== "settings" &&
          page !== "overview" &&
          page !== "project" && (
            <div className="header-actions">
              <button
                onClick={() =>
                  void run(() => call("game.play", { id: projectId }))
                }
              >
                <Icon name="play" /> 试玩
              </button>
              <button
                onClick={() =>
                  void run(async () => {
                    const list = await call<string[]>("game.presets", {
                      id: projectId,
                    });
                    setPresets(list);
                    setPreset(list[0] ?? "");
                    setExporting(true);
                  })
                }
              >
                导出
              </button>
              <button
                aria-label="打开项目文件夹"
                title="打开项目文件夹"
                onClick={() =>
                  void run(() => call("project.reveal", { id: projectId }))
                }
              >
                <Icon name="folder" />
              </button>
            </div>
          )
        }
      >
        <div className="page-content">
          {project && ["overview", "project", "features"].includes(page) && (
            <nav className="game-section-tabs" aria-label="游戏设置">
              <button
                className={page === "overview" ? "active" : ""}
                onClick={() => setPage("overview")}
              >
                总览
              </button>
              <button
                className={page === "project" ? "active" : ""}
                onClick={() => setPage("project")}
              >
                创作重点
              </button>
              <button
                className={page === "features" ? "active" : ""}
                onClick={() => setPage("features")}
              >
                功能包
              </button>
            </nav>
          )}
          {!state ? (
            <div className="startup" role="status">
              正在打开本地工作室…
            </div>
          ) : page === "settings" || page === "environment" ? (
            <SettingsView
              key={page}
              initial={state.settings}
              run={run}
              environment={page === "environment"}
            />
          ) : !project && page === "create" ? (
            <TaskBoard
              tasks={[]}
              open={() => {}}
              create={() => setCreate(true)}
              createLabel="创建项目"
            />
          ) : !project && ["docs", "library"].includes(page) ? (
            <StudioPreview
              key="demo"
              projectKey="demo"
              page={page as "create" | "docs" | "library"}
              navigate={setPage}
            />
          ) : !project ? (
            <section className="welcome">
              <h2>未打开项目</h2>
              <div className="welcome-actions">
                <button className="primary" onClick={() => setCreate(true)}>
                  创建项目
                </button>
              </div>
            </section>
          ) : page === "docs" ? (
            <DocumentsView
              key={projectId}
              projectId={projectId}
              run={run}
              addReference={(ref) => {
                setRefs([ref]);
                setPage("create");
              }}
            />
          ) : page === "overview" ? (
            <ProjectOverviewView
              key={project.id}
              project={project}
              busy={busy > 0}
              play={() => void run(() => call("game.play", { id: projectId }))}
              exportGame={() =>
                void run(async () => {
                  const list = await call<string[]>("game.presets", {
                    id: projectId,
                  });
                  setPresets(list);
                  setPreset(list[0] ?? "");
                  setExporting(true);
                })
              }
              draft={overviewDrafts[project.id]}
              changeDraft={(draft) =>
                setOverviewDrafts((old) => ({ ...old, [project.id]: draft }))
              }
              clearDraft={() =>
                setOverviewDrafts((old) => {
                  const next = { ...old };
                  delete next[project.id];
                  return next;
                })
              }
              saved={(name, fields) => setOverviewRisk({ name, fields })}
              run={run}
            />
          ) : page === "project" ? (
            <ProjectBlueprintView
              key={project.id}
              project={project}
              draft={blueprintDrafts[project.id]}
              changeDraft={(draft) =>
                setBlueprintDrafts((old) => ({ ...old, [project.id]: draft }))
              }
              clearDraft={() =>
                setBlueprintDrafts((old) => {
                  const next = { ...old };
                  delete next[project.id];
                  return next;
                })
              }
              features={state.features}
              openOverview={() => setPage("overview")}
              run={run}
            />
          ) : page === "tasks" || page === "create" ? (
            <TasksView
              key={projectId}
              projectId={projectId}
              tasks={tasks}
              refs={refs}
              clearRefs={() => setRefs([])}
              run={run}
            />
          ) : page === "assets" || page === "library" ? (
            <AssetsView
              projectId={projectId}
              addReferences={(r) => {
                setRefs(r);
                setPage("tasks");
              }}
              run={run}
            />
          ) : (
            <FeaturesView
              key={project.id}
              features={state.features}
              project={project}
              draft={blueprintDrafts[project.id]}
              changeDraft={(draft) =>
                setBlueprintDrafts((old) => ({ ...old, [project.id]: draft }))
              }
              clearDraft={() =>
                setBlueprintDrafts((old) => {
                  const next = { ...old };
                  delete next[project.id];
                  return next;
                })
              }
              run={run}
              queued={() => setPage("tasks")}
            />
          )}
        </div>
      </DesktopShell>
      {create && (
        <CreateProjectDialog
          close={closeCreate}
          features={state?.features ?? []}
          run={run}
          created={(project) => {
            setProjectId(project.id);
            setCreate(false);
            setPage("overview");
          }}
        />
      )}
      {overviewRisk && (
        <Dialog
          title="项目设定已更改，存在风险"
          className="overview-risk-dialog"
          close={() => setOverviewRisk(undefined)}
        >
          <div className="overview-risk" role="alert">
            <strong>基础设定变更可能导致游戏崩坏。</strong>
            <p>
              “{overviewRisk.name}”已修改：{overviewRisk.fields.join("、")}。
            </p>
            <p>
              现有玩法、素材、存档或网络逻辑可能与新设定不兼容。继续开发前，请检查影响并验证游戏。
            </p>
          </div>
          <p className="muted">
            本次只保存 Beaver 项目设定，不会同步修改游戏文件、目录名或自动启动
            AI 修复。
          </p>
          <footer>
            <button
              className="primary"
              onClick={() => setOverviewRisk(undefined)}
            >
              我已知晓
            </button>
          </footer>
        </Dialog>
      )}
      {exporting && (
        <Dialog title="导出可运行游戏" close={closeExport}>
          <div className="tools-toolbar">
            <button
              disabled={busy > 0}
              title="从 Godot 官方下载并校验当前稳定版的模板包（可能超过 1 GB），安装 Windows x86_64 模板"
              onClick={() => prepareTemplates(false)}
            >
              {preparingTemplates ? "准备模板中…" : "下载 Windows 模板"}
            </button>
            <button
              disabled={busy > 0}
              title="仅导入你信任的、与当前引擎版本匹配的 TPZ；不会覆盖已有模板"
              onClick={() => prepareTemplates(true)}
            >
              导入模板包
            </button>
            {preparingTemplates && (
              <button
                onClick={() => void run(() => call("game.cancelTemplates"))}
              >
                停止准备模板
              </button>
            )}
          </div>
          {!presets.length && (
            <div className="warning-box">
              项目没有导出预设。请给 Codex 发出“配置当前系统的导出预设”任务。
            </div>
          )}
          <Field label="导出预设">
            <select value={preset} onChange={(e) => setPreset(e.target.value)}>
              {presets.map((p) => (
                <option key={p}>{p}</option>
              ))}
            </select>
          </Field>
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
            <button onClick={closeExport}>取消</button>
            <button
              disabled={busy > 0}
              onClick={() =>
                void run(async () => {
                  const directory = await call<string | null>(
                    "chooseDirectory",
                  );
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
              disabled={!preset || !destination || busy > 0}
              onClick={() =>
                void run(async () => {
                  const result = await call<{ path: string }>("game.export", {
                    id: projectId,
                    preset,
                    destination,
                  });
                  notify({ tone: "success", text: `导出完成：${result.path}` });
                  setExporting(false);
                })
              }
            >
              生成游戏程序
            </button>
          </footer>
        </Dialog>
      )}
      {setup && (
        <Dialog title="未找到必要工具" close={() => setSetup(false)}>
          <p>安装工具，或在设置中选择已有程序。</p>
          {missingTools.map((tool) => (
            <div className="feature-row" key={tool.name}>
              <strong>{tool.name}</strong>
              <button
                disabled={busy > 0}
                onClick={() =>
                  void run(async () => {
                    await call("tools.install", { name: tool.name });
                    setMissingTools(
                      (await call<ToolStatus[]>("tools.detect")).filter(
                        (t) =>
                          !t.available && ["codex", "godot"].includes(t.name),
                      ),
                    );
                  })
                }
              >
                安装 {tool.name}
              </button>
            </div>
          ))}
          <footer>
            <button onClick={() => setSetup(false)}>稍后准备</button>
            <button
              className="primary"
              onClick={() => {
                setSetup(false);
                setPage("settings");
              }}
            >
              打开设置
            </button>
          </footer>
        </Dialog>
      )}
    </>
  );
}

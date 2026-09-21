import { useCallback, useEffect, useState } from "react";
import type { Project, Reference, ToolStatus } from "../shared/types";
import { call, type Run, type State } from "./api";
import { Dialog } from "./components";
import { TasksView } from "./TasksView";
import { SettingsView } from "./SettingsView";
import { DesktopShell, type Page } from "./DesktopShell";
import { Icon } from "./Icon";
import { useNotify } from "./Notifications";
import { errorMessage } from "./notification-state";
import { FeaturesView } from "./FeaturesView";
import { CreateProjectDialog } from "./CreateProjectDialog";
import { ProjectRegistrationDialog } from "./ProjectRegistrationDialog";
import { MigrationDialog } from "./MigrationDialog";
import { ProjectBlueprintView } from "./ProjectBlueprintView";
import type { BlueprintDraft } from "../shared/project-blueprint";
import type { OverviewDraft } from "../shared/project-overview";
import { ProjectOverviewView } from "./ProjectOverviewView";
import { StudioPreview } from "./StudioPreview";
import { DocumentsView } from "./DocumentsView";
import { TaskBoard } from "./TaskBoard";
import { ExportGameDialog } from "./ExportGameDialog";
import { ValidationView } from "./validation/ValidationView";
import { ObjectFrameworkWorkspace } from "./object-preview/ObjectFrameworkWorkspace";
import "./object-preview/preview-shell.css";
import "./object-preview/preview-embedded.css";
import "./object-preview/preview-object-toolbar.css";
import "./object-preview/object-framework-workspace.css";

export function App() {
  const notify = useNotify();
  const [state, setState] = useState<State>();
  const [projectId, setProjectId] = useState("");
  const [page, setPage] = useState<Page>("objects");
  const [busy, setBusy] = useState(0);
  const [refs, setRefs] = useState<Reference[]>([]);
  const [create, setCreate] = useState(false);
  const [registration, setRegistration] = useState<Project>();
  const [migration, setMigration] = useState(false);
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
  const [exporting, setExporting] = useState<{ releaseId?: string }>();
  const [taskFocus, setTaskFocus] = useState("");
  const [validationFocus, setValidationFocus] = useState<{
    taskId?: string;
    runId?: string;
  }>({});
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
  useEffect(() => {
    setRefs([]);
    setTaskFocus("");
    setValidationFocus({});
    setExporting(undefined);
  }, [projectId]);
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
  function openValidation(focus: { taskId?: string; runId?: string }) {
    setValidationFocus(focus);
    setPage("validation");
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
        manageProject={
          "__TAURI__" in window ? () => setRegistration(project) : undefined
        }
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
          page === "settings" && "__TAURI__" in window ? (
            <button onClick={() => setMigration(true)}>数据迁移</button>
          ) : (
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
                <button onClick={() => setExporting({})}>导出</button>
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
          )
        }
      >
        <div
          className={`page-content${["objects", "manufacture"].includes(page) ? " object-preview-content" : ""}${page === "validation" && project ? " validation-workspace-content" : ""}`}
        >
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
          {["objects", "manufacture"].includes(page) ? (
            <ObjectFrameworkWorkspace
              key={project?.id ?? "no-project"}
              project={project}
              loading={!state}
              page={page === "manufacture" ? "manufacture" : "objects"}
              createProject={() => setCreate(true)}
            />
          ) : !state ? (
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
          ) : !project && page === "docs" ? (
            <StudioPreview
              key="demo"
              projectKey="demo"
              page={page}
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
              exportGame={() => setExporting({})}
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
          ) : page === "validation" ? (
            <ValidationView
              key={`${projectId}:${validationFocus.taskId ?? ""}:${validationFocus.runId ?? ""}`}
              projectId={projectId}
              tasks={tasks}
              initialTaskId={validationFocus.taskId}
              initialRunId={validationFocus.runId}
              openTask={(id) => {
                setTaskFocus(id);
                setPage("create");
              }}
              exportGame={(releaseId) => setExporting({ releaseId })}
            />
          ) : page === "tasks" || page === "create" ? (
            <TasksView
              key={`${projectId}:${taskFocus}`}
              projectId={projectId}
              initialTaskId={taskFocus}
              openValidation={(taskId) => openValidation({ taskId })}
              tasks={tasks}
              refs={refs}
              clearRefs={() => setRefs([])}
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
      {migration && <MigrationDialog close={() => setMigration(false)} />}
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
      {registration && (
        <ProjectRegistrationDialog
          project={registration}
          close={() => setRegistration(undefined)}
          run={run}
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
      {exporting && project && (
        <ExportGameDialog
          key={`${projectId}:${exporting.releaseId ?? "new"}`}
          projectId={projectId}
          initialReleaseId={exporting.releaseId}
          busy={busy > 0}
          run={run}
          close={() => setExporting(undefined)}
          openRun={(runId) => openValidation({ runId })}
        />
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

import { useState } from "react";
import type { Project } from "../shared/types";
import {
  audiences,
  gameSizes,
  themeChoices,
} from "../shared/blueprint-catalog";
import { styles } from "../shared/game-design";
import {
  blueprintProblems,
  defaultBlueprint,
} from "../shared/project-blueprint";
import {
  changedOverviewFields,
  overviewBlueprint,
  overviewSchema,
  projectOverview,
  type OverviewDraft,
  type ProjectOverview,
} from "../shared/project-overview";
import {
  OverviewChoices,
  allGenres,
  choiceLabel,
  genreIcon,
  themeIcon,
  type OverviewChoice,
} from "./OverviewChoices";
import { Icon } from "./Icon";
import { call, type Run } from "./api";
import { Dialog, Field } from "./components";

export function ProjectOverviewView({
  project,
  draft,
  changeDraft,
  clearDraft,
  saved,
  run,
  play,
  exportGame,
  busy,
}: {
  project: Project;
  draft?: OverviewDraft;
  changeDraft: (draft: OverviewDraft) => void;
  clearDraft: () => void;
  saved: (name: string, fields: string[]) => void;
  run: Run;
  play: () => void;
  exportGame: () => void;
  busy: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [discard, setDiscard] = useState(false);
  const [saving, setSaving] = useState(false);
  const [choice, setChoice] = useState<OverviewChoice>();
  const current = projectOverview(project);
  const value = draft?.value ?? current;
  const base = project.blueprint ?? defaultBlueprint(project.design);
  const plan = overviewBlueprint(value, base);
  const parsed = overviewSchema.safeParse(value);
  const changes = changedOverviewFields(
    current,
    parsed.success ? parsed.data : value,
  );
  const problems = [
    ...new Set([
      ...(parsed.success
        ? []
        : parsed.error.issues.map((issue) => issue.message)),
      ...blueprintProblems(plan),
    ]),
  ];
  const stale = !!draft && draft.revision !== (project.blueprintRevision ?? 0);
  const edit = (next: ProjectOverview) =>
    changeDraft({
      value: next,
      revision: draft?.revision ?? project.blueprintRevision ?? 0,
    });
  const lock = () => {
    clearDraft();
    setEditing(false);
    setDiscard(false);
  };
  const shown = editing ? value : current;
  const configure = (kind: OverviewChoice, label: string) => (
    <button
      className="overview-configure"
      aria-label={label}
      title={editing ? label : "开启编辑设定后修改"}
      disabled={!editing || saving}
      onClick={() => setChoice(kind)}
    >
      <Icon name="settings" />
    </button>
  );
  return (
    <section className="overview-page" aria-label="游戏总览">
      <div className="overview-name">
        {editing ? (
          <Field label="游戏名称">
            <input
              aria-label="游戏名称"
              value={value.name}
              maxLength={80}
              disabled={saving}
              onChange={(event) => edit({ ...value, name: event.target.value })}
            />
          </Field>
        ) : (
          <h1 title={current.name}>{current.name}</h1>
        )}
      </div>
      <div className="overview-reserved">
        <div className="game-launch-actions">
          <button className="primary" disabled={busy} onClick={play}>
            <Icon name="play" />
            试玩游戏
          </button>
          <button disabled={busy} onClick={exportGame}>
            导出游戏
          </button>
        </div>
      </div>
      <aside className="overview-card" aria-label="游戏设定卡">
        <div className="overview-toolbar">
          <span className="muted" role="status">
            {editing ? "编辑中" : "设定已锁定"}
            {draft ? " · 草稿未保存" : ""}
          </span>
          <label className="overview-edit-switch">
            编辑设定
            <input
              type="checkbox"
              role="switch"
              aria-label="编辑游戏设定"
              checked={editing}
              disabled={saving}
              onChange={(event) => {
                if (event.target.checked) setEditing(true);
                else if (draft && (changes.length || stale)) setDiscard(true);
                else lock();
              }}
            />
            <span className="overview-switch-track" aria-hidden="true" />
          </label>
        </div>
        <div className="overview-card-feature">
          <span className="overview-card-art">
            <Icon name={genreIcon(shown.genres[0]!)} />
          </span>
          <div className="overview-card-copy">
            <strong>{choiceLabel(allGenres, shown.genres[0]!)}</strong>
            <span>
              主类别
              {shown.genres[1]
                ? ` · 副类别 ${choiceLabel(allGenres, shown.genres[1])}`
                : " · 无副类别"}
            </span>
          </div>
          {configure("genres", "修改类别")}
        </div>
        <div className="overview-card-feature">
          <span className="overview-card-art">
            <Icon
              name={
                shown.theme.mode === "custom"
                  ? "palette"
                  : themeIcon(shown.theme.value)
              }
            />
          </span>
          <div className="overview-card-copy">
            <strong>
              {shown.theme.mode === "custom"
                ? shown.theme.value
                : choiceLabel(themeChoices, shown.theme.value)}
            </strong>
            <span>主题</span>
          </div>
          {configure("theme", "修改主题")}
        </div>
        <dl className="overview-card-details">
          <div>
            <dt>游戏评级</dt>
            <dd>{choiceLabel(audiences, shown.audience)}</dd>
            {configure("audience", "修改评级")}
          </div>
          <div>
            <dt>游戏大小</dt>
            <dd>{choiceLabel(gameSizes, shown.size)}</dd>
            {configure("size", "修改游戏大小")}
          </div>
          <div>
            <dt>在线多人游戏</dt>
            <dd>
              <span
                className={`online-indicator${shown.online.enabled ? " enabled" : ""}`}
              >
                {shown.online.enabled ? "开启" : "关闭"}
              </span>
            </dd>
            {configure("online", "修改在线多人设定")}
          </div>
          <div>
            <dt>表现风格</dt>
            <dd>{choiceLabel(styles, shown.style)}</dd>
            {configure("style", "修改表现风格")}
          </div>
        </dl>
        {shown.online.enabled && (
          <p className="overview-online-summary">
            {shown.online.playersPerSession} 人 / 局 · 峰值{" "}
            {shown.online.peakCcu} 人<br />
            {shown.online.topology === "dedicated"
              ? "专用服务器"
              : "玩家主机 + 中继"}{" "}
            · {shown.online.serverCount} 实例 · {shown.online.region}
          </p>
        )}
        {editing && (
          <>
            <p className="overview-risk" role="note">
              修改基础设定可能与现有游戏冲突，请谨慎保存。
            </p>
            {stale && (
              <p className="blueprint-error" role="alert">
                已保存版本发生变化，草稿保留中。请重新载入后编辑。
              </p>
            )}
            {!!problems.length && (
              <p className="blueprint-error" role="alert">
                {problems.join("；")}
              </p>
            )}
            <div className="overview-savebar">
              <button
                disabled={!draft || saving}
                onClick={() => setDiscard(true)}
              >
                重新载入
              </button>
              <button
                className="primary"
                disabled={
                  saving || stale || !!problems.length || !changes.length
                }
                onClick={() => {
                  setSaving(true);
                  void run(async () => {
                    const updated = await call<Project>(
                      "project.overview.save",
                      {
                        id: project.id,
                        overview: value,
                        expectedRevision:
                          draft?.revision ?? project.blueprintRevision ?? 0,
                        allowRiskyChanges: true,
                      },
                    );
                    clearDraft();
                    setEditing(false);
                    saved(updated.name, changes);
                  }).finally(() => setSaving(false));
                }}
              >
                保存设定
              </button>
            </div>
          </>
        )}
      </aside>
      {choice && (
        <OverviewChoices
          kind={choice}
          value={value}
          apply={edit}
          close={() => setChoice(undefined)}
        />
      )}
      {discard && (
        <Dialog title="放弃设定草稿？" close={() => setDiscard(false)}>
          <p>
            尚未保存的修改将被丢弃，载入已保存设定并重新锁定。游戏文件不会改变。
          </p>
          <footer>
            <button onClick={() => setDiscard(false)}>继续编辑</button>
            <button className="danger" onClick={lock}>
              放弃修改并锁定
            </button>
          </footer>
        </Dialog>
      )}
    </section>
  );
}

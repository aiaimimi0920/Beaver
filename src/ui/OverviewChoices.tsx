import { useState } from "react";
import {
  audiences,
  gameSizes,
  genreChoices,
  themeChoices,
} from "../shared/blueprint-catalog";
import { styles } from "../shared/game-design";
import {
  blueprintProblems,
  defaultBlueprint,
  legacyGenreChoices,
  onlinePlanSchema,
} from "../shared/project-blueprint";
import {
  overviewBlueprint,
  type ProjectOverview,
} from "../shared/project-overview";
import { Dialog, Field } from "./components";
import { Icon, type IconName } from "./Icon";

export type OverviewChoice =
  "genres" | "theme" | "audience" | "size" | "style" | "online";
export const choiceTitles: Record<OverviewChoice, string> = {
  genres: "选择类别",
  theme: "选择主题",
  audience: "选择评级",
  size: "选择游戏大小",
  style: "选择表现风格",
  online: "在线多人设定",
};
const groupIcons: Record<string, IconName> = {
  动作: "sword",
  探索: "compass",
  叙事: "book",
  角色扮演: "review",
  休闲: "puzzle",
  模拟经营: "city",
  载具: "wheel",
  策略: "crown",
};
export function genreIcon(id: string): IconName {
  if (["fps", "tps", "shoot", "artillery", "shoot-em-up"].includes(id))
    return "target";
  if (["farming", "animals", "living-organism"].includes(id)) return "leaf";
  if (["mmorpg", "open-world", "space"].includes(id)) return "globe";
  if (id === "rythm") return "music";
  if (["bishojo", "eroge", "otome"].includes(id)) return "heart";
  return (
    groupIcons[genreChoices.find((g) => g.id === id)?.group ?? ""] ?? "puzzle"
  );
}
export function themeIcon(id: string): IconName {
  if (
    [
      "cyberpunk",
      "hack",
      "gamedev",
      "science-fiction",
      "industrialization",
      "steampunk",
    ].includes(id)
  )
    return "chip";
  if (["ghost", "horror", "vampire", "zombie", "postapocalyptic"].includes(id))
    return "moon";
  if (["pet", "evolution", "prehistory", "hunt", "kaiju"].includes(id))
    return "leaf";
  if (["erotic", "medical", "comedy"].includes(id)) return "heart";
  if (
    [
      "history",
      "mythology",
      "medieval",
      "religious",
      "fantasy",
      "dragon",
      "arabian-nights",
    ].includes(id)
  )
    return "book";
  if (["military", "ninja", "pirates", "western"].includes(id)) return "sword";
  if (["spy", "organized-crime"].includes(id)) return "target";
  if (["ocean", "desert", "atlantis", "archeology", "time-travel"].includes(id))
    return "compass";
  if (id === "urban") return "city";
  if (id === "music") return "music";
  if (["transportation", "football", "soccer"].includes(id)) return "wheel";
  return "palette";
}
export const choiceLabel = (
  choices: readonly { id: string; name: string }[],
  id: string,
) => choices.find((c) => c.id === id)?.name ?? id;
export const allGenres = [...genreChoices, ...legacyGenreChoices];
interface Choice {
  id: string;
  name: string;
  icon: IconName;
  meaning?: string;
}

export function OverviewChoices({
  kind,
  value,
  apply,
  close,
}: {
  kind: OverviewChoice;
  value: ProjectOverview;
  apply: (value: ProjectOverview) => void;
  close: () => void;
}) {
  const [local, setLocal] = useState(() => structuredClone(value));
  const [slot, setSlot] = useState<0 | 1>(0);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState("catalog");
  const online = (patch: Partial<ProjectOverview["online"]>) =>
    setLocal((old) => ({ ...old, online: { ...old.online, ...patch } }));
  const choices: Choice[] =
    kind === "genres"
      ? [
          ...genreChoices.map((g) => ({ ...g, icon: genreIcon(g.id) })),
          ...legacyGenreChoices
            .filter((g) => value.genres.includes(g.id))
            .map((g) => ({ ...g, icon: genreIcon(g.id) })),
          ...(slot === 1
            ? [{ id: "", name: "无副类别", icon: "close" as const }]
            : []),
        ]
      : kind === "theme"
        ? [
            ...themeChoices.map((t) => ({ ...t, icon: themeIcon(t.id) })),
            { id: "custom", name: "自定义主题", icon: "add" },
          ]
        : kind === "audience"
          ? audiences.map((a) => ({ ...a, icon: "review" }))
          : kind === "size"
            ? gameSizes.map((s) => ({ ...s, icon: "layers" }))
            : kind === "style"
              ? styles.map((s) => ({
                  ...s,
                  icon:
                    s.id === "pixel"
                      ? "overview"
                      : s.id === "text"
                        ? "book"
                        : s.id.includes("3d") || s.id === "low-poly"
                          ? "features"
                          : "palette",
                }))
              : [];
  const filtered = choices.filter((c) =>
    c.name.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()),
  );
  if (sort === "name")
    filtered.sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
  const selectedId =
    kind === "genres"
      ? (local.genres[slot] ?? "")
      : kind === "theme"
        ? local.theme.mode === "custom"
          ? "custom"
          : local.theme.value
        : kind === "online"
          ? ""
          : local[kind];
  const selected = choices.find((c) => c.id === selectedId);
  const problems = blueprintProblems(
    overviewBlueprint(local, defaultBlueprint()),
  );
  const choose = (id: string) => {
    if (kind === "genres") {
      const genres =
        slot === 0
          ? [
              id,
              ...(local.genres[1]
                ? [local.genres[1] === id ? local.genres[0]! : local.genres[1]]
                : []),
            ]
          : id
            ? [local.genres[0]!, id]
            : [local.genres[0]!];
      setLocal({ ...local, genres });
    } else if (kind === "theme")
      setLocal({
        ...local,
        theme:
          id === "custom"
            ? {
                mode: "custom",
                value: local.theme.mode === "custom" ? local.theme.value : "",
              }
            : { mode: "preset", value: id },
      });
    else if (kind === "audience")
      setLocal({ ...local, audience: id as ProjectOverview["audience"] });
    else if (kind === "size")
      setLocal({ ...local, size: id as ProjectOverview["size"] });
    else if (kind === "style")
      setLocal({ ...local, style: id as ProjectOverview["style"] });
  };
  return (
    <Dialog
      title={choiceTitles[kind]}
      className={`overview-choice-dialog${kind === "online" ? " online-choice-dialog" : ""}`}
      close={close}
    >
      {kind === "genres" && (
        <div className="genre-slots" aria-label="类别位置">
          {([0, 1] as const).map((index) => (
            <button
              key={index}
              aria-pressed={slot === index}
              onClick={() => setSlot(index)}
            >
              <span>{index === 0 ? "主类别" : "副类别"}</span>
              <strong>
                {local.genres[index]
                  ? choiceLabel(allGenres, local.genres[index]!)
                  : "无"}
              </strong>
            </button>
          ))}
        </div>
      )}
      {(kind === "genres" || kind === "theme") && (
        <div className="choice-search">
          <input
            type="search"
            aria-label="搜索卡牌"
            placeholder="搜索名称"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <select
            aria-label="卡牌排序"
            value={sort}
            onChange={(e) => setSort(e.target.value)}
          >
            <option value="catalog">目录顺序</option>
            <option value="name">名称</option>
          </select>
          <span className="muted">{filtered.length} 项</span>
        </div>
      )}
      <div className="overview-choice-body">
        {kind === "online" ? (
          <>
            <label className="check online-switch">
              <input
                type="checkbox"
                checked={local.online.enabled}
                onChange={(e) => online({ enabled: e.target.checked })}
              />
              在线多人游戏
            </label>
            {(local.online.enabled ||
              !onlinePlanSchema.safeParse(local.online).success) && (
              <div className="network-plan">
                <div className="form-row">
                  <Field label="联机架构">
                    <select
                      value={local.online.topology}
                      onChange={(e) =>
                        online({
                          topology: e.target
                            .value as ProjectOverview["online"]["topology"],
                        })
                      }
                    >
                      <option value="dedicated">专用权威服务器</option>
                      <option value="host-relay">玩家主机 + 中继</option>
                    </select>
                  </Field>
                  <Field label="目标服务区域">
                    <input
                      value={local.online.region}
                      maxLength={120}
                      onChange={(e) => online({ region: e.target.value })}
                    />
                  </Field>
                </div>
                <div className="form-row">
                  <Field label="每局 / 房间人数">
                    <input
                      type="number"
                      min={2}
                      max={10000}
                      value={
                        Number.isFinite(local.online.playersPerSession)
                          ? local.online.playersPerSession
                          : ""
                      }
                      onChange={(e) =>
                        online({ playersPerSession: e.target.valueAsNumber })
                      }
                    />
                  </Field>
                  <Field label="峰值同时在线人数">
                    <input
                      type="number"
                      min={2}
                      max={10000000}
                      value={
                        Number.isFinite(local.online.peakCcu)
                          ? local.online.peakCcu
                          : ""
                      }
                      onChange={(e) =>
                        online({ peakCcu: e.target.valueAsNumber })
                      }
                    />
                  </Field>
                  <Field
                    label={
                      local.online.topology === "dedicated"
                        ? "初期服务器实例数"
                        : "初期中继实例数"
                    }
                  >
                    <input
                      type="number"
                      min={1}
                      max={100000}
                      value={
                        Number.isFinite(local.online.serverCount)
                          ? local.online.serverCount
                          : ""
                      }
                      onChange={(e) =>
                        online({ serverCount: e.target.valueAsNumber })
                      }
                    />
                  </Field>
                </div>
              </div>
            )}
            <p className="muted">容量规划，不代表已部署资源或压测结果。</p>
          </>
        ) : (
          <div
            className={`choice-grid${kind !== "genres" && kind !== "theme" ? " choice-grid-short" : ""}`}
            aria-label="选择卡牌"
          >
            {filtered.map((c) => {
              const badge =
                kind === "genres"
                  ? local.genres[0] === c.id
                    ? "主类别"
                    : local.genres[1] === c.id
                      ? "副类别"
                      : ""
                  : selectedId === c.id
                    ? "已选择"
                    : "";
              return (
                <button
                  key={c.id}
                  type="button"
                  className={`choice-card${badge ? " assigned" : ""}`}
                  aria-label={c.name}
                  aria-pressed={selectedId === c.id}
                  disabled={
                    kind === "genres" && slot === 1 && c.id === local.genres[0]
                  }
                  onClick={() => choose(c.id)}
                >
                  <span className="choice-art">
                    <Icon name={c.icon} />
                    {badge && <small>{badge}</small>}
                  </span>
                  <span className="choice-name">{c.name}</span>
                </button>
              );
            })}
            {!filtered.length && <p className="choice-empty">没有匹配的选项</p>}
          </div>
        )}
      </div>
      {kind === "theme" && local.theme.mode === "custom" && (
        <Field label="自定义主题">
          <input
            value={local.theme.value}
            maxLength={120}
            placeholder="输入你的原创主题"
            onChange={(e) =>
              setLocal({
                ...local,
                theme: { mode: "custom", value: e.target.value },
              })
            }
          />
        </Field>
      )}
      {selected?.meaning && (
        <p className="choice-meaning">{selected.meaning}</p>
      )}
      {!!problems.length && (
        <p className="blueprint-error" role="alert">
          {problems.join("；")}
        </p>
      )}
      <footer>
        <span className="muted">应用到草稿，尚未保存</span>
        <button onClick={close}>取消</button>
        <button
          className="primary"
          disabled={!!problems.length}
          onClick={() => {
            apply(local);
            close();
          }}
        >
          应用
        </button>
      </footer>
    </Dialog>
  );
}

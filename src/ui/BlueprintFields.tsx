import { styles } from "../shared/game-design";
import {
  audiences,
  gameSizes,
  genreChoices,
  themeChoices,
} from "../shared/blueprint-catalog";
import {
  legacyGenreChoices,
  onlinePlanSchema,
  type ProjectBlueprint,
} from "../shared/project-blueprint";
import { Field } from "./components";

const groups = [...new Set(genreChoices.map((g) => g.group))];
const genreOptions = (exclude?: string) =>
  groups.map((group) => (
    <optgroup key={group} label={group}>
      {genreChoices
        .filter((g) => g.group === group && g.id !== exclude)
        .map((g) => (
          <option key={g.id} value={g.id}>
            {g.name}
          </option>
        ))}
    </optgroup>
  ));

export function BlueprintFields({
  value,
  change,
}: {
  value: ProjectBlueprint;
  change: (value: ProjectBlueprint) => void;
}) {
  const online = (patch: Partial<ProjectBlueprint["online"]>) =>
    change({ ...value, online: { ...value.online, ...patch } });
  return (
    <div className="blueprint-fields">
      <div className="form-row">
        <Field label="主类别">
          <select
            aria-label="主类别"
            value={value.genres[0]}
            onChange={(e) =>
              change({
                ...value,
                genres: [
                  e.target.value,
                  ...value.genres
                    .slice(1)
                    .filter((id) => id !== e.target.value),
                ],
              })
            }
          >
            {genreOptions()}
            {legacyGenreChoices
              .filter((g) => value.genres.includes(g.id))
              .map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
          </select>
        </Field>
        <Field label="副类别">
          <select
            aria-label="副类别"
            value={value.genres[1] ?? ""}
            onChange={(e) =>
              change({
                ...value,
                genres: e.target.value
                  ? [value.genres[0]!, e.target.value]
                  : [value.genres[0]!],
              })
            }
          >
            <option value="">无</option>
            {genreOptions(value.genres[0])}
            {legacyGenreChoices
              .filter((g) => value.genres.slice(1).includes(g.id))
              .map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
          </select>
        </Field>
      </div>
      <div className="form-row">
        <Field label="游戏主题">
          <select
            aria-label="游戏主题"
            value={value.theme.mode === "custom" ? "custom" : value.theme.value}
            onChange={(e) =>
              change({
                ...value,
                theme:
                  e.target.value === "custom"
                    ? { mode: "custom", value: "" }
                    : { mode: "preset", value: e.target.value },
              })
            }
          >
            {themeChoices.map((t) => (
              <option key={t.id} value={t.id}>
                {t.name}
              </option>
            ))}
            <option value="custom">自定义主题…</option>
          </select>
        </Field>
        <Field label="表现风格">
          <select
            aria-label="表现风格"
            value={value.style}
            onChange={(e) =>
              change({
                ...value,
                style: e.target.value as ProjectBlueprint["style"],
              })
            }
          >
            {styles.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {value.theme.mode === "custom" && (
        <Field label="自定义主题">
          <input
            aria-label="自定义主题"
            value={value.theme.value}
            maxLength={120}
            placeholder="例如：雨夜车站里的失物招领"
            onChange={(e) =>
              change({
                ...value,
                theme: { mode: "custom", value: e.target.value },
              })
            }
          />
        </Field>
      )}
      <div className="form-row">
        <Field label="目标评级">
          <select
            aria-label="目标评级"
            value={value.audience}
            onChange={(e) =>
              change({
                ...value,
                audience: e.target.value as ProjectBlueprint["audience"],
              })
            }
          >
            {audiences.map((a) => (
              <option key={a.id} value={a.id}>
                {a.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="游戏规模">
          <select
            aria-label="游戏规模"
            value={value.size}
            onChange={(e) =>
              change({
                ...value,
                size: e.target.value as ProjectBlueprint["size"],
              })
            }
          >
            {gameSizes.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <details className="blueprint-help">
        <summary>评级与规模的含义</summary>
        <p>{audiences.find((a) => a.id === value.audience)?.meaning}</p>
        <p>{gameSizes.find((s) => s.id === value.size)?.meaning}</p>
        <p>
          评级是创作内容尺度目标，不是正式分级认证；规模不是游戏时长或质量承诺。当前只保存规划，不自动限制或生成素材。
        </p>
      </details>
      <label className="check online-switch">
        <input
          type="checkbox"
          checked={value.online.enabled}
          onChange={(e) => online({ enabled: e.target.checked })}
        />
        在线多人游戏
      </label>
      {(value.online.enabled ||
        !onlinePlanSchema.safeParse(value.online).success) && (
        <div className="network-plan">
          {!value.online.enabled && (
            <p className="muted">联机已关闭，请修正保留的容量草案后保存。</p>
          )}
          <div className="form-row">
            <Field label="联机架构">
              <select
                value={value.online.topology}
                onChange={(e) =>
                  online({
                    topology: e.target
                      .value as ProjectBlueprint["online"]["topology"],
                  })
                }
              >
                <option value="dedicated">专用权威服务器</option>
                <option value="host-relay">玩家主机 + 中继</option>
              </select>
            </Field>
            <Field label="目标服务区域">
              <input
                value={value.online.region}
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
                  Number.isFinite(value.online.playersPerSession)
                    ? value.online.playersPerSession
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
                  Number.isFinite(value.online.peakCcu)
                    ? value.online.peakCcu
                    : ""
                }
                onChange={(e) => online({ peakCcu: e.target.valueAsNumber })}
              />
            </Field>
            <Field
              label={
                value.online.topology === "dedicated"
                  ? "初期服务器实例数"
                  : "初期中继实例数"
              }
            >
              <input
                type="number"
                min={1}
                max={100000}
                value={
                  Number.isFinite(value.online.serverCount)
                    ? value.online.serverCount
                    : ""
                }
                onChange={(e) =>
                  online({ serverCount: e.target.valueAsNumber })
                }
              />
            </Field>
          </div>
          <p className="muted">
            容量规划，不是已部署资源或压测结果；单局人数与全服同时在线人数不同。
          </p>
        </div>
      )}
    </div>
  );
}

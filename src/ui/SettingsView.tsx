import { useEffect, useState } from "react";
import { capabilities, type Settings, type ToolStatus } from "../shared/types";
import { call, type Run } from "./api";
import { Field } from "./components";
import { Icon, type IconName } from "./Icon";
import type { ToolSetupState } from "../shared/tool-setup";
const names = {
  code: "Codex 编程",
  review: "独立代码审查",
  image: "图像生成 / 编辑",
  speech: "语音生成",
  music: "音乐 / 音效",
  translation: "翻译",
};
const icons: Record<(typeof capabilities)[number], IconName> = {
  code: "code",
  review: "review",
  image: "assets",
  speech: "speech",
  music: "music",
  translation: "translation",
};
export function SettingsView({
  initial,
  run,
  environment = false,
}: {
  initial: Settings;
  run: Run;
  environment?: boolean;
}) {
  const [settings, setSettings] = useState<Settings>(() =>
    structuredClone(initial),
  );
  const [keys, setKeys] = useState<Record<string, string>>({});
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [detecting, setDetecting] = useState(true);
  const [setup, setSetup] = useState<ToolSetupState>();
  const toolBusy = detecting || setup?.status === "running";
  useEffect(() => {
    let live = true;
    const refresh = () => {
      void call<ToolSetupState>("tools.setupStatus")
        .then((state) => {
          if (live) setSetup(state);
        })
        .catch(() => {});
    };
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, []);
  function applyTools(result: ToolStatus[]) {
    setTools(result);
    setSettings((old) => ({
      ...old,
      tools: {
        ...old.tools,
        ...Object.fromEntries(
          result
            .filter((tool) => tool.available)
            .map((tool) => [tool.name, tool.path]),
        ),
      },
    }));
  }
  function prepare(name?: ToolStatus["name"]) {
    setDetecting(true);
    void run(async () => {
      try {
        const result = await call<ToolSetupState>(
          name ? "tools.install" : "tools.setup",
          name ? { name } : { tools: settings.tools },
        );
        if (result.status !== "completed")
          throw new Error(result.error ?? "环境准备未完成");
      } finally {
        setSetup(await call<ToolSetupState>("tools.setupStatus"));
        applyTools(await call<ToolStatus[]>("tools.detect"));
      }
    }, "工具路径已验证并保存").finally(() => setDetecting(false));
  }
  useEffect(() => {
    let active = true;
    void call<ToolStatus[]>("tools.detect")
      .then((result) => {
        if (!active) return;
        setTools(result);
        setSettings((s) => ({
          ...s,
          tools: {
            ...s.tools,
            ...Object.fromEntries(
              result
                .filter((t) => t.available && !s.tools[t.name])
                .map((t) => [t.name, t.path]),
            ),
          },
        }));
      })
      .catch(() => {})
      .finally(() => {
        if (active) setDetecting(false);
      });
    return () => {
      active = false;
    };
  }, []);
  return (
    <div className={`settings-page${environment ? " environment-page" : ""}`}>
      {environment && (
        <header className="settings-toolbar">
          <h1>创作环境</h1>
        </header>
      )}
      {!environment && (
        <>
          <header className="settings-toolbar">
            <h1>设置</h1>
            <div className="segmented" aria-label="AI 服务来源">
              {(
                [
                  ["local", "自配 API"],
                  ["cloud", "平台 AI"],
                ] as const
              ).map(([value, label]) => (
                <button
                  className={settings.mode === value ? "selected" : ""}
                  aria-pressed={settings.mode === value}
                  key={value}
                  onClick={() => setSettings({ ...settings, mode: value })}
                >
                  {label}
                </button>
              ))}
            </div>
            <button
              className="primary"
              disabled={toolBusy}
              onClick={() =>
                void run(async () => {
                  setSettings(
                    await call<Settings>("settings.save", { settings, keys }),
                  );
                  setKeys({});
                }, "设置已保存")
              }
            >
              保存设置
            </button>
          </header>
          <section className="service-settings">
            {settings.mode === "local" && (
              <div className="settings-import">
                <button
                  title="保存当前设置，仅补齐同一服务的编程、审查和翻译空白项；不复制登录令牌或个人 MCP"
                  disabled={toolBusy}
                  onClick={() =>
                    void run(async () => {
                      const result = await call<{
                        settings: Settings;
                        filled: string[];
                      }>("settings.importLocalCodex", { settings, keys });
                      setSettings(result.settings);
                      setKeys({});
                    }, "本机 API 设置已补齐，已有配置保持不变")
                  }
                >
                  从本机 Codex 补齐
                </button>
              </div>
            )}
            {settings.mode === "cloud" && (
              <>
                <div className="warning-box">
                  需配置平台网关与账号令牌；当前版本不提供充值和计费后台。
                </div>
                <div className="form-row">
                  <Field label="平台 API Base URL">
                    <input
                      value={settings.cloud.baseUrl}
                      onChange={(e) =>
                        setSettings({
                          ...settings,
                          cloud: { ...settings.cloud, baseUrl: e.target.value },
                        })
                      }
                      placeholder="https://平台地址/v1"
                    />
                  </Field>
                  <Field label="平台账号令牌">
                    <input
                      type="password"
                      value={keys.cloud ?? ""}
                      placeholder={
                        settings.cloud.hasKey ? "已保存；留空保留" : "尚未配置"
                      }
                      onChange={(e) =>
                        setKeys({ ...keys, cloud: e.target.value })
                      }
                    />
                    {settings.cloud.hasKey && (
                      <button
                        onClick={() =>
                          void run(async () => {
                            await call("settings.clearKey", { slot: "cloud" });
                            setSettings((s) => ({
                              ...s,
                              cloud: { ...s.cloud, hasKey: false },
                            }));
                            setKeys((k) => ({ ...k, cloud: "" }));
                          })
                        }
                      >
                        清除平台令牌
                      </button>
                    )}
                  </Field>
                </div>
              </>
            )}
            <div className="providers">
              {capabilities.map((cap) => (
                <details
                  className="provider settings-group"
                  name="settings-section"
                  key={cap}
                >
                  <summary>
                    <Icon name={icons[cap]} />
                    <span>{names[cap]}</span>
                  </summary>
                  <div className="settings-group-body">
                    <div className="form-row">
                      {settings.mode === "local" && (
                        <Field label="API Base URL">
                          <input
                            value={settings.local[cap].baseUrl}
                            placeholder="https://服务地址/v1"
                            onChange={(e) =>
                              setSettings({
                                ...settings,
                                local: {
                                  ...settings.local,
                                  [cap]: {
                                    ...settings.local[cap],
                                    baseUrl: e.target.value,
                                  },
                                },
                              })
                            }
                          />
                        </Field>
                      )}
                      <Field label="模型标识">
                        <input
                          value={
                            settings.mode === "cloud"
                              ? settings.cloudModels[cap]
                              : settings.local[cap].model
                          }
                          onChange={(e) =>
                            setSettings(
                              settings.mode === "cloud"
                                ? {
                                    ...settings,
                                    cloudModels: {
                                      ...settings.cloudModels,
                                      [cap]: e.target.value,
                                    },
                                  }
                                : {
                                    ...settings,
                                    local: {
                                      ...settings.local,
                                      [cap]: {
                                        ...settings.local[cap],
                                        model: e.target.value,
                                      },
                                    },
                                  },
                            )
                          }
                        />
                      </Field>
                      {settings.mode === "local" && (
                        <Field label="API Key">
                          <input
                            type="password"
                            value={keys[cap] ?? ""}
                            placeholder={
                              settings.local[cap].hasKey
                                ? "已保存；留空保留"
                                : "未配置；免密本机服务可留空"
                            }
                            onChange={(e) =>
                              setKeys({ ...keys, [cap]: e.target.value })
                            }
                          />
                          {settings.local[cap].hasKey && (
                            <button
                              onClick={() =>
                                void run(async () => {
                                  await call("settings.clearKey", {
                                    slot: cap,
                                  });
                                  setSettings((s) => ({
                                    ...s,
                                    local: {
                                      ...s.local,
                                      [cap]: { ...s.local[cap], hasKey: false },
                                    },
                                  }));
                                  setKeys((k) => ({ ...k, [cap]: "" }));
                                })
                              }
                            >
                              清除此 Key
                            </button>
                          )}
                        </Field>
                      )}
                    </div>
                    {["image", "speech", "music"].includes(cap) && (
                      <Field label="媒体 API 路径">
                        <input
                          value={settings.local[cap].route}
                          onChange={(e) =>
                            setSettings({
                              ...settings,
                              local: {
                                ...settings.local,
                                [cap]: {
                                  ...settings.local[cap],
                                  route: e.target.value,
                                },
                              },
                            })
                          }
                          placeholder={
                            cap === "music"
                              ? "显式填写服务支持的音乐生成路径"
                              : ""
                          }
                        />
                      </Field>
                    )}
                    <small
                      className="muted"
                      title="完整接口约定见随附文档 AI_SERVICES.md"
                    >
                      {["code", "review", "translation"].includes(cap)
                        ? "Responses API"
                        : cap === "music"
                          ? "自定义接口：model/prompt → 音频或 data[].b64_json / url"
                          : "OpenAI 兼容接口"}
                    </small>
                  </div>
                </details>
              ))}
            </div>
          </section>
        </>
      )}
      <details
        className="settings-group"
        name="settings-section"
        open={environment || undefined}
      >
        <summary>
          <Icon name="tools" />
          <span>本机工具</span>
          <small>
            {detecting
              ? "检测中…"
              : `${tools.filter((t) => t.available).length} / 4 就绪`}
          </small>
        </summary>
        <div className="settings-group-body">
          <header className="tools-toolbar">
            <button
              className="primary"
              disabled={toolBusy}
              onClick={() => prepare()}
            >
              一键准备工具
            </button>
            {setup?.status === "running" && (
              <button onClick={() => void run(() => call("tools.cancelSetup"))}>
                停止准备
              </button>
            )}
            <button
              disabled={toolBusy}
              onClick={() => {
                setDetecting(true);
                void run(async () => {
                  const result = await call<ToolStatus[]>("tools.detect", {
                    tools: settings.tools,
                  });
                  setTools(result);
                  setSettings((s) => ({
                    ...s,
                    tools: {
                      ...s.tools,
                      ...Object.fromEntries(
                        result
                          .filter((t) => t.available)
                          .map((t) => [t.name, t.path]),
                      ),
                    },
                  }));
                }).finally(() => setDetecting(false));
              }}
            >
              {detecting ? "检测中…" : "重新检测"}
            </button>
          </header>
          {setup && setup.status !== "idle" && (
            <div className="tool-setup-status" role="status">
              {setup.steps.map((step) => (
                <span key={step.name}>
                  {step.name} ·{" "}
                  {
                    {
                      waiting: "等待",
                      checking: "检测中",
                      installing: "安装中",
                      ready: "就绪",
                      failed: "失败",
                      cancelled: "已停止",
                    }[step.status]
                  }
                </span>
              ))}
              {setup.error && (
                <span className="warning-box">{setup.error}</span>
              )}
            </div>
          )}
          {(["codex", "godot", "blender", "node"] as const).map((name) => (
            <div className="tool-row" key={name}>
              <Field label={name}>
                <input
                  aria-label={`${name} 路径`}
                  value={settings.tools[name]}
                  placeholder="自动检测"
                  disabled={toolBusy}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      tools: { ...settings.tools, [name]: e.target.value },
                    })
                  }
                />
              </Field>
              <button
                disabled={toolBusy}
                onClick={() =>
                  void run(async () => {
                    const file = await call<string | null>("chooseTool");
                    if (file)
                      setSettings({
                        ...settings,
                        tools: { ...settings.tools, [name]: file },
                      });
                  })
                }
              >
                选择
              </button>
              <button
                disabled={
                  toolBusy || tools.find((t) => t.name === name)?.available
                }
                onClick={() => prepare(name)}
              >
                安装
              </button>
              <small
                className={
                  tools.find((t) => t.name === name)?.available
                    ? "positive"
                    : "muted"
                }
              >
                {tools.find((t) => t.name === name)?.version}
              </small>
            </div>
          ))}
          <p className="muted">
            安装可能需要系统权限；Godot 导出模板需与引擎版本匹配。
          </p>
        </div>
      </details>
      {!environment && (
        <details className="settings-group" name="settings-section">
          <summary>
            <Icon name="settings" />
            <span>执行与扩展</span>
          </summary>
          <div className="settings-group-body">
            <Field label="并发任务上限">
              <input
                type="number"
                min={1}
                max={6}
                value={settings.maxParallel}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    maxParallel: Number(e.target.value),
                  })
                }
              />
            </Field>
            <Field label="全局自动决策">
              <AutonomySelect
                label="全局自动决策"
                value={settings.askRatio ?? 100}
                change={(value) =>
                  setSettings({ ...settings, askRatio: value ?? 100 })
                }
              />
            </Field>
            <p className="muted">
              按决策重要性筛选；任务设置优先。已展示的问题保留回答，切换档位不会静默提交。
            </p>
            <label className="check">
              <input
                type="checkbox"
                checked={settings.mcp.godot}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    mcp: { ...settings.mcp, godot: e.target.checked },
                  })
                }
              />
              Godot MCP（首次使用需下载）
            </label>
            <label className="check">
              <input
                type="checkbox"
                checked={settings.mcp.blender}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    mcp: { ...settings.mcp, blender: e.target.checked },
                  })
                }
              />
              Blender MCP（需 uvx 与 Blender add-on server）
            </label>
            <p className="muted">
              使用独立 Codex
              配置。自主任务可操作本机，工作副本不是系统沙箱，仅用于可信项目。
            </p>
          </div>
        </details>
      )}
    </div>
  );
}
import { AutonomySelect } from "./AutonomySelect";

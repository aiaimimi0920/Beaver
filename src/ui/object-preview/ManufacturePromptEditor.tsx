import { Icon } from "../Icon";
import type {
  ManufactureSession,
  ManufacturePrompts,
} from "./useManufactureDraft";

export type ManufacturePromptTab = keyof ManufacturePrompts;
const tabs = [
  { key: "creation", label: "创建提示词", icon: "sparkles" },
  { key: "acceptance", label: "自动验收", icon: "review" },
  { key: "output", label: "输出格式", icon: "layers" },
] as const;

export function ManufacturePromptEditor({
  session,
  tab,
  selectTab,
  model,
}: {
  session: ManufactureSession;
  tab: ManufacturePromptTab;
  selectTab: (tab: ManufacturePromptTab) => void;
  model: boolean;
}) {
  const { prompts, checks } = session.draft;
  const changeCheck = (patch: Partial<typeof checks>) =>
    session.edit({ checks: { ...checks, ...patch } });
  return (
    <div className="op-manufacture-editor">
      <div className="op-manufacture-tabs" role="tablist" aria-label="阶段要求">
        {tabs.map((item) => (
          <button
            key={item.key}
            role="tab"
            id={`manufacture-tab-${item.key}`}
            aria-selected={tab === item.key}
            aria-controls="manufacture-prompt-panel"
            onClick={() => selectTab(item.key)}
          >
            <Icon name={item.icon} />
            {item.label}
          </button>
        ))}
      </div>
      <div
        className="op-manufacture-editor-body"
        id="manufacture-prompt-panel"
        role="tabpanel"
        aria-labelledby={`manufacture-tab-${tab}`}
      >
        <label
          className={`op-manufacture-prompt${tab === "acceptance" ? " is-acceptance" : ""}`}
        >
          <span>
            {tab === "acceptance"
              ? "自动验收提示词"
              : tab === "output"
                ? "输出格式提示词"
                : "阶段创建要求"}
          </span>
          <textarea
            value={prompts[tab]}
            spellCheck={false}
            onChange={(event) =>
              session.edit({
                prompts: { ...prompts, [tab]: event.target.value },
              })
            }
          />
        </label>
        {tab === "acceptance" && (
          <div className="op-manufacture-checks">
            <header>
              <strong>验收流程</strong>
              <small>工具产出数据，AI 结合数据判断</small>
            </header>
            {model && (
              <div className="op-manufacture-check">
                <label>
                  <input
                    type="checkbox"
                    checked={checks.geometry}
                    onChange={(event) =>
                      changeCheck({ geometry: event.target.checked })
                    }
                  />
                  <Icon name="code" />
                  <span>
                    <strong>面数统计</strong>
                    <small>确定性脚本 · Blender · 输出统计报告</small>
                  </span>
                </label>
                <label className="op-manufacture-limit">
                  三角面上限
                  <input
                    type="number"
                    min="1"
                    step="1"
                    aria-label="三角面上限"
                    disabled={!checks.geometry}
                    value={checks.triangleLimit}
                    onChange={(event) =>
                      changeCheck({ triangleLimit: event.target.value })
                    }
                  />
                </label>
              </div>
            )}
            <div className="op-manufacture-check">
              <label>
                <input
                  type="checkbox"
                  checked={checks.views}
                  onChange={(event) =>
                    changeCheck({ views: event.target.checked })
                  }
                />
                <Icon name="assets" />
                <span>
                  <strong>{model ? "固定生成三视图" : "固定生成效果图"}</strong>
                  <small>
                    {model
                      ? "工具模板 · 正面 / 侧面 / 背面 · 固定相机与灯光"
                      : "工具模板 · 整体 / 局部 / 对照"}
                  </small>
                </span>
              </label>
              <small>PNG · 固定视角</small>
            </div>
            <div className="op-manufacture-check">
              <label>
                <input
                  type="checkbox"
                  checked={checks.visual}
                  onChange={(event) =>
                    changeCheck({ visual: event.target.checked })
                  }
                />
                <Icon name="sparkles" />
                <span>
                  <strong>视觉与需求核对</strong>
                  <small>AI 判断 · 使用上方验收提示词与工具数据</small>
                </span>
              </label>
              <small>结论 + 依据</small>
            </div>
            <small>工具项为配置演示，尚未执行脚本或调用 AI。</small>
          </div>
        )}
      </div>
    </div>
  );
}

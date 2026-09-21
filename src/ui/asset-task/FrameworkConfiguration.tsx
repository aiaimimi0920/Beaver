import { useState } from "react";
import type {
  FrameworkAction,
  FrameworkConfiguration as Configuration,
} from "../../shared/framework";

export function FrameworkConfiguration({
  configuration,
  disabled,
  action,
}: {
  configuration: Configuration;
  disabled: boolean;
  action: FrameworkAction;
}) {
  const [text, setText] = useState(JSON.stringify(configuration, null, 2));
  const [error, setError] = useState("");
  return (
    <details>
      <summary>插件与检查器配置 · 修订 {configuration.revision}</summary>
      <p>
        配置任务专用适配器；固定可执行文件、脚本
        SHA-256、插件和宿主版本。适配器读取 BEAVER_CONTEXT，并输出 BEAVER_RESULT
        JSON。更改规则会使旧检查失效。
      </p>
      <label>
        框架配置 JSON
        <textarea
          aria-label="框架配置 JSON"
          rows={12}
          value={text}
          disabled={disabled}
          onChange={(e) => setText(e.target.value)}
        />
      </label>
      {error && <p role="alert">{error}</p>}
      <button
        disabled={disabled}
        onClick={() => {
          try {
            const parsed: unknown = JSON.parse(text);
            setError("");
            void action({ operation: "configure", configuration: parsed });
          } catch {
            setError("请输入有效 JSON；字段和版本由 Beaver 校验。");
          }
        }}
      >
        保存配置
      </button>
    </details>
  );
}

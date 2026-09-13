import { useState } from "react";
import { Field } from "../components";
import type { Mutate, ValidationSettings as Settings } from "./types";
import { perform } from "./useValidation";

export function ValidationSettings({
  settings,
  mutate,
  busy,
}: {
  settings: Settings;
  mutate: Mutate;
  busy: boolean;
}) {
  const [ffmpeg, setFfmpeg] = useState(settings.ffmpeg);
  function save(visualRequired: boolean, encoder = settings.ffmpeg) {
    perform(
      mutate("validation.settings.save", {
        expectedRevision: settings.revision,
        settings: { ...settings, visualRequired, ffmpeg: encoder },
      }),
    );
  }
  return (
    <section className="validation-settings">
      <label className="check">
        <input
          type="checkbox"
          checked={settings.visualRequired}
          disabled={busy}
          onChange={(e) => save(e.target.checked)}
        />
        正式发布前强制运行一次画面诊断
      </label>
      <small>
        普通任务由代码验收和安全合入决定完成；画面可以稍后查看。内部验证导出不受正式发布绿灯限制。
      </small>
      <details>
        <summary>录像设置</summary>
        <Field label="FFmpeg 可执行文件（留空使用 PATH）">
          <input
            value={ffmpeg}
            onChange={(e) => setFfmpeg(e.target.value)}
            placeholder="ffmpeg"
          />
        </Field>
        <button
          disabled={busy || ffmpeg === settings.ffmpeg}
          onClick={() => save(settings.visualRequired, ffmpeg)}
        >
          保存录像设置
        </button>
      </details>
    </section>
  );
}

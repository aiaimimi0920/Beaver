import { useState, useSyncExternalStore } from "react";
import { ObjectScenePreview } from "../object-preview/ObjectScenePreview";
import type { SceneTarget } from "../object-preview/object-scene-preview";
import { FrozenMedia } from "../FrozenMedia";
import type { ObjectAttemptFile } from "./object-attempt-file";

export function ObjectAttemptFilePreview({
  session,
}: {
  session: ObjectAttemptFile;
}) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  if (!state.request) return null;
  const content = state.response?.content;
  return (
    <section aria-label="冻结文件正文">
      <h4>
        {state.request.checkpoint === "input" ? "输入" : "输出"}：
        {state.request.path}
      </h4>
      <p style={{ overflowWrap: "anywhere" }}>
        SHA-256：{state.request.sha256}
      </p>
      <button onClick={session.cancel}>关闭预览</button>
      {/\.(tscn|scn|blend)$/.test(state.request.path) && (
        <AttemptScene
          key={JSON.stringify(state.request)}
          target={state.request}
        />
      )}
      {state.loading && <p role="status">正在读取冻结文件…</p>}
      {state.error && <p role="alert">无法读取冻结文件：{state.error}</p>}
      {state.response && <p>{state.response.byteCount} 字节</p>}
      {(content?.kind === "image" || content?.kind === "audio") && (
        <FrozenMedia
          key={JSON.stringify(state.request)}
          content={content}
          name={state.request.path}
        />
      )}
      {content?.kind === "binary" && (
        <p>二进制或非 UTF-8 文本，不支持正文预览。</p>
      )}
      {content?.kind === "tooLarge" && (
        <p>超过 1 MiB 正文预览上限；本次预览未校验文件完整性。</p>
      )}
      {content?.kind === "text" && (
        <pre
          style={{
            whiteSpace: "pre-wrap",
            overflowWrap: "anywhere",
            maxHeight: "32rem",
            overflow: "auto",
          }}
        >
          {content.text}
        </pre>
      )}
    </section>
  );
}

function AttemptScene({ target }: { target: SceneTarget }) {
  const [open, setOpen] = useState(false);
  return open ? (
    <ObjectScenePreview target={target} />
  ) : (
    <button type="button" onClick={() => setOpen(true)}>
      {target.path.endsWith(".blend") ? "Blender 三维预览" : "Godot 场景预览"}
    </button>
  );
}

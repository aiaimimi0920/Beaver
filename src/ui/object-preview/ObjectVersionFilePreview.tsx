import { useEffect, useState, useSyncExternalStore } from "react";
import { call } from "../api";
import { FrozenMedia } from "../FrozenMedia";
import { ObjectScenePreview } from "./ObjectScenePreview";
import {
  readFrozenObjectVersion,
  type ObjectCatalogRecord,
  type ObjectCatalogVersion,
} from "../../shared/object-catalog";
import {
  ObjectVersionFile,
  type VersionFileResponse,
} from "./object-version-file";

export function ObjectVersionFilePreview({
  object,
  version,
}: {
  object: ObjectCatalogRecord;
  version: ObjectCatalogVersion;
}) {
  const [session] = useState(() => new ObjectVersionFile(call));
  const [scene, setScene] = useState<string | null>(null);
  useEffect(() => () => session.cancel(), [session]);
  const manifest = readFrozenObjectVersion(object, version);
  if (!manifest) return <p role="alert">版本清单无效，无法预览。</p>;
  return (
    <section aria-label="版本文件预览">
      <p>冻结版本：{version.versionId}；从版本存储读取，不读取当前项目文件。</p>
      {manifest.files.length ? (
        <ul>
          {manifest.files.map((file) => (
            <li key={file.path}>
              <button
                type="button"
                onClick={() =>
                  void session.open({
                    projectId: object.projectId,
                    objectId: object.id,
                    versionId: version.versionId,
                    path: file.path,
                    sha256: file.sha256,
                  })
                }
              >
                {file.path}
              </button>{" "}
              <small>
                {file.role} · {file.bytes} 字节
              </small>
              {/\.(tscn|scn|blend)$/.test(file.path) && (
                <button type="button" onClick={() => setScene(file.path)}>
                  {file.path.endsWith(".blend")
                    ? "Blender 三维预览"
                    : "Godot 场景预览"}
                </button>
              )}
            </li>
          ))}
        </ul>
      ) : (
        <p>该版本没有文件。</p>
      )}
      <VersionFileView session={session} />
      {scene &&
        manifest.files
          .filter((file) => file.path === scene)
          .map((file) => (
            <ObjectScenePreview
              key={`${version.versionId}:${file.path}`}
              target={{
                projectId: object.projectId,
                objectId: object.id,
                versionId: version.versionId,
                path: file.path,
                sha256: file.sha256,
              }}
            />
          ))}
    </section>
  );
}

export function VersionFileView({ session }: { session: ObjectVersionFile }) {
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  if (!state.request) return null;
  return (
    <section aria-label="冻结版本文件内容">
      <h4>{state.request.path}</h4>
      <p style={{ overflowWrap: "anywhere" }}>
        {state.request.versionId} · SHA-256：{state.request.sha256}
      </p>
      <button type="button" onClick={session.cancel}>
        关闭预览
      </button>
      {state.loading && <p role="status">正在校验并读取版本文件…</p>}
      {state.error && <p role="alert">无法预览：{state.error}</p>}
      {state.response && (
        <VersionFileContent
          key={JSON.stringify(state.request)}
          response={state.response}
        />
      )}
    </section>
  );
}

function VersionFileContent({ response }: { response: VersionFileResponse }) {
  const { content } = response;
  if (content.kind === "tooLarge")
    return <p>超过 8 MiB 预览上限；本次未校验内容摘要。</p>;
  if (content.kind === "unsupported")
    return (
      <p>
        内容摘要已校验；当前文件格式不支持内联预览。Godot 场景和 Blender
        文件可使用上方场景预览入口。
      </p>
    );
  if (content.kind === "text")
    return (
      <>
        <p>内容摘要已校验；仅显示文本，不执行脚本或渲染场景。</p>
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
      </>
    );
  return <FrozenMedia content={content} name={response.request.path} />;
}

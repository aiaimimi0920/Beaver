import { useEffect, useState, useSyncExternalStore } from "react";
import { assetUrl, call } from "../api";
import { LiveScenePreview } from "./LiveScenePreview";
import { SavedPreviewFrames } from "./SavedPreviewFrames";
import {
  ObjectScenePreview as Session,
  type SceneTarget,
  type PreviewResolution,
} from "./object-scene-preview";

export function ObjectScenePreview({ target }: { target: SceneTarget }) {
  const blender = target.path.endsWith(".blend");
  const engine = blender ? "Blender" : "Godot";
  const [interactive, setInteractive] = useState(false);
  const [savedRevision, setSavedRevision] = useState(0);
  const [session] = useState(() => new Session(target, call));
  const state = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot,
  );
  const run = state.result?.run;
  const active = !!run && ["queued", "running"].includes(run.status);
  useEffect(() => {
    void session.refresh();
    const refresh = () => {
      if (!document.hidden) void session.refresh();
    };
    document.addEventListener("visibilitychange", refresh);
    return () => {
      document.removeEventListener("visibilitychange", refresh);
      session.close();
    };
  }, [session]);
  useEffect(() => {
    if (!active) return;
    const timer = setInterval(() => {
      if (!document.hidden) void session.refresh();
    }, 1000);
    return () => clearInterval(timer);
  }, [active, session]);
  const image = run?.evidence.find((item) => item.kind === "image");
  return (
    <section aria-label={engine + " 冻结场景预览"}>
      <h4>
        {engine} 场景：{target.path}
      </h4>
      <p>
        {blender
          ? "使用冻结 .blend 和依赖生成 Blender 实体预览（Workbench），显示几何与材质基础色；不呈现最终节点材质和灯光。保留场景相机，无相机时按网格边界生成观察相机。禁用脚本自动执行；缺失或快照外依赖会报错。完成后可打开交互相机并保存编号图像标注。"
          : "运行冻结场景及快照内依赖，按所选分辨率采集图像。完成后可打开独立交互相机。可信脚本会执行，工作副本隔离不提供操作系统沙箱。"}
      </p>
      <label>
        下次渲染分辨率
        <select
          value={state.resolution}
          disabled={state.busy || active || state.retryPending}
          onChange={(event) =>
            session.setResolution(event.target.value as PreviewResolution)
          }
        >
          <option value="540p">960 × 540</option>
          <option value="720p">1280 × 720</option>
          <option value="1080p">1920 × 1080</option>
        </select>
      </label>
      {state.retryPending && <p>请求结果尚未确认；重试保留原分辨率。</p>}
      <button
        type="button"
        disabled={state.busy || active}
        onClick={() => void session.render()}
      >
        {state.error ? "重试渲染请求" : "渲染冻结场景"}
      </button>
      {active && (
        <button
          type="button"
          disabled={state.busy}
          onClick={() => void session.cancel()}
        >
          取消渲染
        </button>
      )}
      <button
        type="button"
        disabled={state.busy}
        onClick={() => void session.refresh()}
      >
        刷新结果
      </button>
      {state.error && <p role="alert">{state.error}</p>}
      {run?.status === "completed" &&
        !state.result?.integrityError &&
        (interactive ? (
          <LiveScenePreview
            key={"live:" + run.id}
            projectId={target.projectId}
            runId={run.id}
            snapshotId={run.snapshotId}
            engine={engine}
            onSaved={() => setSavedRevision((value) => value + 1)}
            onClose={() => setInteractive(false)}
          />
        ) : (
          <button type="button" onClick={() => setInteractive(true)}>
            打开交互预览
          </button>
        ))}
      {run && (
        <>
          <p role="status">
            {run.status} · {run.phase}
          </p>
          {run.error && <p role="alert">{run.error}</p>}
          {state.result?.integrityError && (
            <p role="alert">{state.result.integrityError}</p>
          )}
          {image && !state.result?.integrityError && (
            <img
              key={image.id}
              src={assetUrl("validation", `${run.id}/${image.id}`)}
              alt={`${engine} 冻结场景 ${target.path}`}
              style={{ maxWidth: "100%" }}
            />
          )}
          <p style={{ overflowWrap: "anywhere" }}>
            {"attemptId" in target
              ? `运行 ${target.runId} · 尝试 ${target.attemptId} · ${target.checkpoint === "input" ? "输入" : "输出"}`
              : `版本 ${target.versionId}`}{" "}
            · 场景 SHA-256 {target.sha256}
            <br />
            已保存尺寸{" "}
            {state.result?.resolution
              ? `${state.result.resolution.width} × ${state.result.resolution.height}`
              : "未知"}{" "}
            · 快照 {run.snapshotId} · {engine} {run.engineVersion || "等待启动"}{" "}
            · {run.runnerVersion}
            <br />
            采集时间 {run.finishedAt || "尚未完成"} · 图像 SHA-256{" "}
            {image?.sha256 || "尚未采集"}
          </p>
          <p>
            {blender
              ? "使用冻结 Blender 场景，仅接受快照内依赖的渲染结果。"
              : state.result?.projectConfig === "frozen"
                ? "使用冻结项目配置。"
                : "快照未包含 project.godot，使用最小预览配置；不读取当前项目配置。"}
            此图像仅用于预览，不计入验收。离开页面不会取消后台渲染；重新打开可恢复结果。
          </p>
        </>
      )}
      {run?.status === "completed" && (
        <SavedPreviewFrames
          key={"saved:" + run.id}
          target={target}
          runId={run.id}
          snapshotId={run.snapshotId}
          refresh={savedRevision}
        />
      )}
    </section>
  );
}

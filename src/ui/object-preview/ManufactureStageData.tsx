import { useState, type CSSProperties } from "react";
import { Dialog } from "../components";
import { Icon } from "../Icon";
import type { DemoObject } from "./mock-objects";
import { objectCategories } from "./object-categories";
import { SelectablePreview } from "./SelectablePreview";
import type { PreviewTarget } from "./preview-target";
import {
  manufactureArtwork,
  manufactureImageSize,
  manufactureTargets,
} from "./manufacture-preview";
import type { ManufactureSession } from "./useManufactureDraft";
import { ManufactureFeedbackEditor } from "./ManufactureFeedbackEditor";

export function ManufactureStageData({
  object,
  version,
  stage,
  session,
  configure,
}: {
  object: DemoObject;
  version: string;
  stage: number;
  session: ManufactureSession;
  configure: () => void;
}) {
  const [inspect, setInspect] = useState<PreviewTarget | null>(null);
  const [selected, setSelected] = useState("");
  const targets = manufactureTargets(object, version, stage, session.viewed);
  const checks = session.round?.draft.checks;
  const activeInspect =
    inspect &&
    [...targets.evidence, ...targets.output].find(
      (target) => target.key === inspect.key,
    );
  const preview = (target: PreviewTarget) => (
    <SelectablePreview
      key={target.key}
      target={target}
      mode={session.mode}
      selected={selected === target.key}
      annotating={!!session.mode}
      annotations={session.draft.annotations}
      renderArtwork={manufactureArtwork}
      imageSize={manufactureImageSize(target)}
      className="op-manufacture-preview"
      onSelect={(value) => {
        setSelected(value.target.key);
        session.add(value);
      }}
      open={() => setInspect(target)}
    />
  );
  const card = (target: PreviewTarget) => {
    const category = objectCategories.find(
      (item) => item.name === target.part?.objectType,
    );
    return (
      <figure
        className="op-manufacture-data-card"
        key={target.key}
        style={{ "--data-color": category?.color } as CSSProperties}
      >
        {preview(target)}
        <figcaption>
          <span>
            <strong title={target.part?.name}>{target.part?.name}</strong>
            <small>{target.part?.format}</small>
          </span>
          <button
            title="放大查看与标记"
            aria-label={`放大${target.part?.name}`}
            onClick={() => setInspect(target)}
          >
            <Icon name="maximize" />
          </button>
        </figcaption>
      </figure>
    );
  };
  return (
    <div className="op-manufacture-data">
      <section aria-label="验收数据">
        <header>
          <h2>
            验收数据 <small>· Mock</small>
          </h2>
          <button onClick={configure}>
            <Icon name="settings" />
            配置验收
          </button>
        </header>
        {!session.round ? (
          <div className="op-manufacture-data-empty">生成后显示验收数据</div>
        ) : (
          <div className="op-manufacture-evidence-body">
            {checks?.views ? (
              <div className="op-manufacture-evidence-grid">
                {targets.evidence.map(card)}
              </div>
            ) : (
              <p className="op-muted">本轮未启用固定效果图</p>
            )}
            <div className="op-manufacture-report">
              <Icon name="code" />
              <span>
                <strong>工具报告</strong>
                <small>
                  {checks?.geometry
                    ? `面数 / 三角面统计 · 三角面上限 ${checks.triangleLimit}`
                    : "未启用面数统计"}
                </small>
              </span>
              <small>待接入</small>
            </div>
            <div className="op-manufacture-report">
              <Icon name="review" />
              <span>
                <strong>AI 验收意见</strong>
                <small>
                  {checks?.visual
                    ? "结合本轮提示词、报告与画面给出结论"
                    : "本轮未启用 AI 判断"}
                </small>
              </span>
              <small>待接入</small>
            </div>
          </div>
        )}
      </section>
      <section aria-label="输出数据">
        <header>
          <h2>
            输出数据{" "}
            <small>
              · {session.round ? `第 ${session.viewed} 轮` : "尚未生成"}
            </small>
          </h2>
          <small>选中工具后标记</small>
        </header>
        {!session.round ? (
          <div className="op-manufacture-data-empty">生成后显示输出资源</div>
        ) : (
          <div className="op-manufacture-output-grid">
            {targets.output.map(card)}
          </div>
        )}
      </section>
      {activeInspect && (
        <Dialog
          title={`查看与标记 · ${activeInspect.part?.name}`}
          close={() => setInspect(null)}
          className="op-manufacture-inspect-dialog"
        >
          <p className="op-manufacture-inspect-source">
            {object.id} · {activeInspect.version} ·
            演示画面，截图与模型射线待接入
          </p>
          <div className="op-manufacture-inspect-layout">
            <div
              className="op-manufacture-inspect-canvas"
              style={
                {
                  "--preview-ratio":
                    manufactureImageSize(activeInspect)[0] /
                    manufactureImageSize(activeInspect)[1],
                } as CSSProperties
              }
            >
              {preview(activeInspect)}
            </div>
            <ManufactureFeedbackEditor session={session} />
          </div>
          <footer>
            <small>标记已同步到本阶段的补充修改</small>
            <button className="primary" onClick={() => setInspect(null)}>
              完成标记
            </button>
          </footer>
        </Dialog>
      )}
    </div>
  );
}

import { Icon } from "../Icon";
import { IterationAnnotations } from "./IterationAnnotations";
import {
  manufactureArtwork,
  manufactureImageSize,
} from "./manufacture-preview";
import type { ManufactureSession } from "./useManufactureDraft";

export function ManufactureFeedbackEditor({
  session,
}: {
  session: ManufactureSession;
}) {
  const { draft } = session;
  return (
    <section className="op-manufacture-feedback" aria-label="补充修改">
      <header>
        <strong>
          补充修改{" "}
          {draft.annotations.length > 0 && (
            <small>· {draft.annotations.length} 个标记</small>
          )}
        </strong>
        <div
          className="op-annotation-tools"
          role="group"
          aria-label="制造画面标注工具"
        >
          <button
            disabled={!session.round}
            aria-pressed={session.mode === "point"}
            onClick={() => session.toggleMode("point")}
          >
            <Icon name="target" />
            点选
          </button>
          <button
            disabled={!session.round}
            aria-pressed={session.mode === "box"}
            onClick={() => session.toggleMode("box")}
          >
            <Icon name="maximize" />
            框选
          </button>
        </div>
      </header>
      <textarea
        aria-label="本轮补充修改提示词"
        rows={2}
        value={draft.instruction}
        placeholder="补充这次要改什么，例如：把序号 1、2 的颜色改为黄色，其他部分保持不变。"
        onChange={(event) => session.edit({ instruction: event.target.value })}
      />
      {session.mode && (
        <small>
          在下方验收图或输出画面上
          {session.mode === "point" ? "点击位置" : "拖动框选"}；可放大后标记。
        </small>
      )}
      <IterationAnnotations
        gallery
        annotations={draft.annotations}
        renderArtwork={manufactureArtwork}
        thumbnailStyle={(target) => ({
          aspectRatio: manufactureImageSize(target).join(" / "),
        })}
        edit={(number, prompt) =>
          session.edit({
            annotations: draft.annotations.map((item) =>
              item.number === number ? { ...item, prompt } : item,
            ),
          })
        }
        remove={(number) =>
          session.edit({
            annotations: draft.annotations.filter(
              (item) => item.number !== number,
            ),
          })
        }
      />
    </section>
  );
}

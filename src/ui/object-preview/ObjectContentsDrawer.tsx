import { useId, useState } from "react";
import { Icon } from "../Icon";
import { objectById, type DemoObject } from "./mock-objects";
import { SelectablePreview } from "./SelectablePreview";
import { referencePreview } from "./object-preview-targets";
import {
  childPreview,
  type PreviewAnnotation,
  type PreviewFeedback,
  type PreviewMode,
  type PreviewTarget,
} from "./preview-target";
import "./preview-object-contents.css";

export function ObjectContentsDrawer({
  object,
  select,
  mode,
  feedback,
  onSelect,
  annotations,
  annotating,
}: {
  object: DemoObject;
  select: (id: string) => void;
  mode: PreviewMode | null;
  feedback: PreviewFeedback | null;
  onSelect: (feedback: PreviewFeedback) => void;
  annotations?: PreviewAnnotation[];
  annotating?: boolean;
}) {
  const [expanded, setExpanded] = useState(false);
  const contentsId = useId();
  const card = (target: PreviewTarget, open?: () => void) => {
    const selected = feedback?.target.key === target.key;
    return (
      <SelectablePreview
        target={selected ? feedback.target : target}
        mode={mode}
        selected={selected}
        selection={selected ? feedback.selection : undefined}
        annotations={annotations}
        annotating={annotating}
        onSelect={onSelect}
        open={open}
        className="op-content-card"
      />
    );
  };
  const summary = [
    ...object.components.map((part) => part.name),
    ...object.references.map(
      (reference) => objectById(reference.objectId).name,
    ),
  ].join("、");
  return (
    <section
      className={`op-contents-drawer${expanded ? " is-expanded" : ""}`}
      aria-label="选中对象内容"
    >
      <header className="op-contents-summary">
        <button
          type="button"
          className="op-contents-toggle"
          aria-label={expanded ? "收起对象内容" : "展开对象内容"}
          title={expanded ? "收起对象内容" : "展开对象内容"}
          aria-expanded={expanded}
          aria-controls={contentsId}
          onClick={() => setExpanded((value) => !value)}
        >
          <Icon name="chevronDown" />
        </button>
        <strong title={object.name}>{object.name}</strong>
        <span className="op-contents-type">{object.objectType}</span>
        <span className="op-contents-count">
          {object.components.length} 项内容
          {object.references.length > 0 &&
            ` · ${object.references.length} 个引用`}
        </span>
        <span className="op-contents-overview" title={summary}>
          {summary || "暂无内容"}
        </span>
      </header>
      <div
        id={contentsId}
        className="op-contents-body"
        hidden={!expanded}
        role="region"
        aria-label={`${object.name}包含的内容`}
        tabIndex={0}
      >
        <ul className="op-contents-grid">
          {object.components.map((part) => {
            const target = childPreview(object, part);
            return (
              <li className="op-content-item" key={target.key}>
                {card(target)}
              </li>
            );
          })}
          {object.references.map((reference) => {
            const referenced = objectById(reference.objectId);
            const target = referencePreview(object, reference);
            return (
              <li className="op-content-item" key={target.key}>
                {card(target, () => select(referenced.id))}
              </li>
            );
          })}
        </ul>
        {!summary && <p className="op-muted">此对象暂无内容</p>}
      </div>
    </section>
  );
}

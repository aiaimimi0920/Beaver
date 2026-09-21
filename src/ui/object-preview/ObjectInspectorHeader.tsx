import { Icon } from "../Icon";
import type { DemoObject } from "./mock-objects";
import type { DemoTask } from "./mock-tasks";
import { Status } from "./PreviewControls";
import type { ObjectPreviewState } from "./useObjectPreview";

export function ObjectInspectorHeader({
  object,
  task,
  preview,
}: {
  object: DemoObject;
  task: DemoTask | null;
  preview: ObjectPreviewState;
}) {
  const version = preview.viewedObject.version;
  const accepted = version === object.accepted;
  const status = accepted ? "已验收" : object.status;
  const progress = accepted ? 100 : task?.progress;
  const versions = [
    ...new Set(
      [object.version, object.accepted].filter(
        (value): value is string => !!value,
      ),
    ),
  ];
  return (
    <>
      <header className="op-inspector-heading">
        <span className="op-mono">{object.id}</span>
        <div className="op-inspector-version-status">
          <label className="op-inspector-select op-inspector-version">
            <select
              aria-label="显示的对象版本"
              title="切换显示版本，不影响正在进行的任务"
              value={version}
              onChange={(event) => preview.selectVersion(event.target.value)}
            >
              {versions.map((value) => (
                <option key={value}>{value}</option>
              ))}
            </select>
            <Icon name="chevronDown" />
          </label>
          <div className="op-inspector-version-progress">
            <div>
              <Status value={status} />
              {progress !== undefined && <span>{progress}%</span>}
            </div>
            {progress !== undefined && (
              <progress
                aria-label={`${version} 制作进度`}
                max={100}
                value={progress}
              />
            )}
          </div>
        </div>
      </header>
      <div className="op-inspector-title-row">
        <h2 title={object.name}>{object.name}</h2>
        <label className="op-inspector-select op-inspector-child">
          <select
            aria-label="操作的子对象"
            title={
              preview.child
                ? preview.choices.find((value) => value.value === preview.child)
                    ?.label
                : "未选择子对象：操作整个对象"
            }
            value={preview.child}
            onChange={(event) => preview.selectChild(event.target.value)}
          >
            {preview.choices.map((value) => (
              <option
                key={value.value}
                value={value.value}
                aria-label={value.label}
              >
                {value.value ? value.label : ""}
              </option>
            ))}
          </select>
          <Icon name="chevronDown" />
        </label>
      </div>
    </>
  );
}

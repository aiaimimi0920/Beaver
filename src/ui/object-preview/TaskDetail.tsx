import { Dialog } from "../components";
import { Icon } from "../Icon";
import { objectById } from "./mock-objects";
import { demoTasks, taskById } from "./mock-tasks";
import { Status, type DemoAction } from "./PreviewControls";
import { IterationAnnotations } from "./IterationAnnotations";

export function TaskDetail({
  id,
  close,
  openTask,
  openObject,
  openCreation,
  action,
}: {
  id: string;
  close: () => void;
  openTask: (id: string) => void;
  openObject: (id: string) => void;
  openCreation: (id: string, stage?: number) => void;
  action: DemoAction;
}) {
  const task = taskById(id),
    children = demoTasks.filter((child) => child.parentId === task.id);
  return (
    <Dialog title={task.title} close={close} className="op-task-dialog">
      <div className="op-task-dialog-meta">
        <span className="op-level">{task.level}</span>
        <span className="op-mono">{task.id}</span>
        <Status value={task.status} />
      </div>
      <p>{task.detail}</p>
      <IterationAnnotations annotations={task.annotations ?? []} />
      <div className="op-task-detail-links">
        {task.parentId && (
          <button onClick={() => openTask(task.parentId!)}>
            <Icon name="tasks" />
            上级任务 · {task.parentId} ↗
          </button>
        )}
        {task.objectId && (
          <button onClick={() => openObject(task.objectId!)}>
            <Icon name="features" />
            {objectById(task.objectId).name} ↗
          </button>
        )}
      </div>
      <h3>{children.length ? "子任务" : "输入与交付"}</h3>
      {children.length ? (
        <div className="op-subtask-list">
          {children.map((child) => (
            <button key={child.id} onClick={() => openTask(child.id)}>
              <span>
                <small>
                  {child.id} · {child.level}
                </small>
                {child.title}
              </span>
              <Status value={child.status} />
            </button>
          ))}
        </div>
      ) : (
        <div className="op-detail-io">
          <div>
            <span>输入</span>
            <strong>
              {task.status === "排队中"
                ? "开始时读取最新已验收版本"
                : "当前轮次规格与上游阶段交付"}
            </strong>
          </div>
          <div>
            <span>交付</span>
            <strong>
              {task.status === "排队中" || task.status === "等待依赖"
                ? "尚未开始，无新交付"
                : task.status === "执行失败"
                  ? "保留当前尝试，等待修正导入配置"
                  : "对象预览、文件清单与阶段检查"}
            </strong>
          </div>
        </div>
      )}
      <p className="op-dialog-note">
        测试数据 · 执行完成、阶段验收与对象最终验收分别记录。
      </p>
      <footer>
        <button
          onClick={() => {
            close();
            action(
              children.length ? "调整任务拆分" : "调整任务要求",
              `${task.id} · ${task.title}`,
            );
          }}
        >
          调整任务
        </button>
        <button
          onClick={() => {
            close();
            action("添加子任务", `${task.id} · 手动补充任务`);
          }}
        >
          <Icon name="add" />
          添加子任务
        </button>
        {task.objectId && (
          <button
            className="primary"
            onClick={() => openCreation(task.objectId!, task.stage)}
          >
            进入对象创作 ↗
          </button>
        )}
      </footer>
    </Dialog>
  );
}

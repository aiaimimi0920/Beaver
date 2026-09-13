import { randomUUID } from "node:crypto";
import type { Change, Project, Task } from "../shared/types";
import { Files, fileHash, safePath } from "./files";
import { Store } from "./store";

export interface FileOperation {
  id: string;
  projectId: string;
  taskId: string;
  kind: "merge" | "rollback";
  state: "applying" | "aborting" | "complete" | "aborted";
  changes: Change[];
  taskAfter: Task;
}

// Durable intent + content hashes let startup distinguish untouched, applied and externally edited files.
export class Journal {
  constructor(
    private store: Store,
    private files: Files,
  ) {}
  async recover(): Promise<void> {
    for (const op of this.store.list<FileOperation>("operation").reverse())
      if (op.state === "applying" || op.state === "aborting") {
        try {
          await this.recoverOne(op);
        } catch (error) {
          const task = this.store.get<Task>("task", op.taskId);
          if (task) {
            task.status = "conflict";
            task.error = `文件恢复尚未完成，项目禁止继续写入。恢复项目路径/权限后重启 Beaver：${String(error)}`;
            this.store.put("task", task.id, task);
            this.store.event(task.id, "recovery", task.error);
          }
        }
      }
  }
  private project(op: FileOperation): Project {
    const project = this.store.get<Project>("project", op.projectId);
    if (!project) throw new Error("文件恢复日志指向不存在的项目");
    return project;
  }
  private commit(op: FileOperation): void {
    this.store.transaction(() => {
      op.taskAfter.direction =
        this.store.get<Task>("task", op.taskId)?.direction ??
        op.taskAfter.direction;
      this.store.put("task", op.taskId, op.taskAfter);
      if (op.kind === "rollback" && op.taskAfter.feature) {
        const key = `${op.projectId}:${op.taskAfter.feature.id}`;
        const adopted = this.store.get<{ taskId: string }>("feature", key);
        if (adopted?.taskId === op.taskId) {
          const previous = op.taskAfter.feature.previous;
          if (previous && !op.taskAfter.retainedFiles?.length)
            this.store.put("feature", key, previous);
          else this.store.remove("feature", key);
        }
      }
      op.state = "complete";
      this.store.put("operation", op.id, op);
    });
  }
  private async abort(op: FileOperation): Promise<void> {
    op.state = "aborting";
    this.store.put("operation", op.id, op);
    const project = this.project(op);
    const reverse: Change[] = [];
    const conflicts: string[] = [];
    for (const change of op.changes) {
      const current = await fileHash(await safePath(project.path, change.path));
      if (current === change.after)
        reverse.push({
          path: change.path,
          before: change.after,
          after: change.before,
        });
      else if (current !== change.before) conflicts.push(change.path);
    }
    await this.files.apply(project.path, reverse.reverse());
    const task = this.store.get<Task>("task", op.taskId)!;
    task.status = op.kind === "rollback" ? "completed" : "conflict";
    task.conflicts = conflicts;
    task.error =
      "上次文件操作未完成，已撤销可确认的写入；外部修改保持不变。请检查后重试或发起对话整合。";
    op.state = "aborted";
    this.store.transaction(() => {
      this.store.put("task", task.id, task);
      this.store.put("operation", op.id, op);
      this.store.event(task.id, "recovery", task.error!);
    });
  }
  async recoverOne(op: FileOperation): Promise<void> {
    if (op.state === "aborting") return this.abort(op);
    const project = this.project(op);
    const remaining: Change[] = [];
    for (const change of op.changes) {
      const current = await fileHash(await safePath(project.path, change.path));
      if (current === change.after) continue;
      if (current !== change.before) return this.abort(op);
      remaining.push(change);
    }
    await this.files.apply(project.path, remaining);
    this.commit(op);
    this.store.event(op.taskId, "recovery", "已恢复上次中断的文件操作。");
  }
  async apply(
    project: Project,
    changes: Change[],
    taskAfter: Task,
    kind: FileOperation["kind"],
  ): Promise<void> {
    const conflicts = await this.files.conflicts(project.path, changes);
    if (conflicts.length)
      throw new Error(`文件冲突，未覆盖：${conflicts.join(", ")}`);
    const op: FileOperation = {
      id: randomUUID(),
      projectId: project.id,
      taskId: taskAfter.id,
      kind,
      state: "applying",
      changes,
      taskAfter,
    };
    this.store.put("operation", op.id, op);
    try {
      await this.files.apply(project.path, changes);
      this.commit(op);
    } catch (error) {
      await this.abort(op);
      throw error;
    }
  }
}

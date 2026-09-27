import { z } from "zod";
import type { ObjectTaskProposal } from "../../shared/object-tasks";
import type { ObjectTaskWorkspace } from "./object-task-workspace";

const resultSchema = z
  .object({
    title: z
      .string()
      .trim()
      .min(1)
      .refine(
        (title) =>
          Array.from(title).length <= 48 &&
          !/[\u0000-\u001f\u007f-\u009f]/u.test(title),
        "标题格式无效",
      ),
  })
  .strict();
type State = {
  pending: boolean;
  candidate: string | null;
  error: string | null;
};
const initial: State = { pending: false, candidate: null, error: null };

export class ObjectTaskTitleController {
  private state = initial;
  private generation = 0;
  private original: ObjectTaskProposal | null = null;
  private listeners = new Set<() => void>();
  constructor(
    private workspace: ObjectTaskWorkspace,
    private taskId: string,
  ) {}
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(state: State) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }
  dismiss = () => {
    this.generation++;
    this.original = null;
    this.set(initial);
  };
  private current() {
    const state = this.workspace.getSnapshot();
    if (
      state.kind !== "ready" ||
      state.operation !== "idle" ||
      state.committedRequestId
    )
      throw new Error("草稿当前不可编辑，请稍后重试");
    const task = state.plan.tasks.find((task) => task.id === this.taskId);
    if (!task) throw new Error("任务已不存在，请重新选择任务");
    return task;
  }
  private assertUnchanged(task: ObjectTaskProposal) {
    if (
      !this.original ||
      task.title !== this.original.title ||
      task.prompt !== this.original.prompt ||
      task.acceptance !== this.original.acceptance
    )
      throw new Error("任务内容已变化，请重新请求标题建议");
  }
  request = async () => {
    if (this.state.pending) return;
    const generation = ++this.generation;
    this.set({ pending: true, candidate: null, error: null });
    try {
      const task = this.current();
      this.original = structuredClone(task);
      const result = resultSchema.parse(
        await this.workspace.suggestTitle(task.prompt, task.acceptance),
      );
      if (generation !== this.generation) return;
      this.assertUnchanged(this.current());
      this.set({ pending: false, candidate: result.title, error: null });
    } catch (error: unknown) {
      if (generation !== this.generation) return;
      this.set({
        pending: false,
        candidate: null,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };
  accept = () => {
    const title = this.state.candidate;
    if (!title || this.state.pending) return;
    try {
      this.assertUnchanged(this.current());
      this.workspace.updatePlan((plan) => ({
        ...plan,
        tasks: plan.tasks.map((task) =>
          task.id === this.taskId ? { ...task, title } : task,
        ),
      }));
      this.dismiss();
    } catch (error: unknown) {
      this.set({
        pending: false,
        candidate: null,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  };
}

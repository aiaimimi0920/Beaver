import {
  objectExecutionSchema,
  type ObjectExecution,
} from "../../shared/object-attempts";
import { revisionTarget } from "../../shared/object-task-revision-plan";
import type { ObjectTaskSnapshot } from "../../shared/object-tasks";

interface State {
  sourceTaskId: string;
  entries: ObjectExecution[];
  loading: boolean;
  loaded: boolean;
  error: string;
}

export class ObjectTaskPromptHistory {
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = {
    sourceTaskId: "",
    entries: [],
    loading: false,
    loaded: false,
    error: "",
  };

  constructor(
    private readonly projectId: string,
    private readonly taskId: string,
    private readonly snapshot: () => ObjectTaskSnapshot,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
  ) {}

  sources() {
    const snapshot = this.snapshot();
    const target = revisionTarget(snapshot, this.taskId);
    if (target.granularity !== "fine") return [];
    return snapshot.tasks.filter(
      (task) =>
        task.granularity === "medium" &&
        task.objectId === target.objectId &&
        task.runId !== target.runId &&
        task.status !== "planned",
    );
  }

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
    this.set({ loading: false });
  };

  load = async (sourceTaskId: string): Promise<boolean> => {
    const generation = ++this.generation;
    this.set({
      sourceTaskId,
      entries: [],
      loading: true,
      loaded: false,
      error: "",
    });
    try {
      const source = this.sources().find((task) => task.id === sourceTaskId);
      if (!source) throw new Error("请选择同一对象的历史迭代");
      const snapshot = this.snapshot();
      const response = await this.api("objectTask.attempts", {
        projectId: this.projectId,
        runId: source.runId,
      });
      if (generation !== this.generation) return false;
      const entries = objectExecutionSchema.array().parse(response);
      const ids = new Set<string>();
      for (const { attempt } of entries) {
        const target = attempt.target;
        const fine = snapshot.tasks.find(
          (task) => task.id === target.fineTaskId,
        );
        if (
          attempt.projectId !== this.projectId ||
          target.taskId !== source.id ||
          target.objectId !== source.objectId ||
          target.runId !== source.runId ||
          !fine ||
          fine.granularity !== "fine" ||
          fine.parentTaskId !== source.id ||
          fine.objectId !== source.objectId ||
          fine.runId !== source.runId ||
          ids.has(target.attemptId)
        ) {
          throw new Error("历史尝试与所选项目、对象或迭代不一致");
        }
        ids.add(target.attemptId);
      }
      this.set({ entries, loading: false, loaded: true });
      return true;
    } catch (error: unknown) {
      if (generation !== this.generation) return false;
      this.set({
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      });
      return false;
    }
  };
}

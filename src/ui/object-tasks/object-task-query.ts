import { parseObjectTaskSnapshot } from "../../shared/object-task-snapshot";
import type { ObjectTaskSnapshot } from "../../shared/object-tasks";

export type ObjectTaskQueryState =
  | { kind: "loading" }
  | { kind: "ready"; snapshot: ObjectTaskSnapshot }
  | { kind: "error"; message: string };
type Api = (method: string, input: unknown) => Promise<unknown>;

export class ObjectTaskQuery {
  private state: ObjectTaskQueryState = { kind: "loading" };
  private generation = 0;
  private listeners = new Set<() => void>();

  constructor(
    readonly projectId: string,
    private readonly api: Api,
  ) {}

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private set(state: ObjectTaskQueryState) {
    this.state = state;
    for (const listener of this.listeners) listener();
  }
  cancel = () => {
    this.generation++;
  };

  refresh = async (): Promise<void> => {
    const generation = ++this.generation;
    this.set({ kind: "loading" });
    try {
      const result = await this.api("objectTask.snapshot", {
        projectId: this.projectId,
      });
      if (generation !== this.generation) return;
      this.set({
        kind: "ready",
        snapshot: parseObjectTaskSnapshot(result, this.projectId),
      });
    } catch (error: unknown) {
      if (generation !== this.generation) return;
      this.set({
        kind: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };
}

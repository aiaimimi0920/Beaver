import {
  parseObjectTaskRevisionHistory,
  type ObjectTaskDefinitionRevision,
} from "../../shared/object-task-revisions";

interface HistoryState {
  entries: readonly ObjectTaskDefinitionRevision[];
  loading: boolean;
  loaded: boolean;
  error: string;
}

export class ObjectTaskRevisionHistoryStore {
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: HistoryState = {
    entries: [],
    loading: false,
    loaded: false,
    error: "",
  };

  constructor(
    private readonly projectId: string,
    private readonly taskId: string,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
  ) {}

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private set(patch: Partial<HistoryState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  cancel = () => {
    this.generation++;
    this.set({ loading: false });
  };

  confirm(receipt: ObjectTaskDefinitionRevision): void {
    // A pre-mutation read must not erase a confirmed receipt when it arrives late.
    this.generation++;
    const entries = this.state.entries.filter(
      (entry) => entry.requestId !== receipt.requestId,
    );
    entries.push(receipt);
    entries.sort((left, right) => left.taskRevision - right.taskRevision);
    this.set({ entries, loading: false, error: "" });
  }

  load = async (): Promise<boolean> => {
    if (this.state.loading) return false;
    const generation = ++this.generation;
    this.set({ loading: true, error: "" });
    try {
      const response = await this.api("objectTask.revisions", {
        projectId: this.projectId,
        taskId: this.taskId,
      });
      if (generation !== this.generation) return false;
      const entries = parseObjectTaskRevisionHistory(
        response,
        this.projectId,
        this.taskId,
        this.state.entries,
      );
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

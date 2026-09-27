import {
  parseFileImportOperation,
  type FileImportOperation,
} from "../../shared/object-file-import";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  loaded: boolean;
  busy: boolean;
  operation: FileImportOperation | null;
  error: string;
}

// Identity is the saved preparation, including after closing or losing a response.
export class FileImportSession {
  private state: State = {
    loaded: false,
    busy: false,
    operation: null,
    error: "",
  };
  private generation = 0;
  private listeners = new Set<() => void>();
  constructor(
    readonly projectId: string,
    readonly preparationId: string,
    private readonly call: Api,
    private readonly sourceKind: "files" | "project" = "files",
  ) {}
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private update(change: Partial<State>) {
    this.state = { ...this.state, ...change };
    this.listeners.forEach((listener) => listener());
  }
  cancel() {
    this.generation++;
    this.update({ busy: false });
  }
  refresh() {
    return this.request(
      this.sourceKind === "files"
        ? "object.fileImportOperation"
        : "object.importOperation",
    );
  }
  commit() {
    if (
      !this.state.loaded ||
      (this.state.operation && this.state.operation.state !== "applying")
    )
      return;
    return this.request(
      this.sourceKind === "files"
        ? "object.commitFileImport"
        : "object.commitImport",
    );
  }
  abort() {
    if (
      !this.state.loaded ||
      !this.state.operation ||
      !["applying", "aborting"].includes(this.state.operation.state)
    )
      return;
    return this.request(
      this.sourceKind === "files"
        ? "object.abortFileImport"
        : "object.abortImport",
    );
  }
  private async request(method: string) {
    if (this.state.busy) return;
    const generation = ++this.generation;
    this.update({ busy: true, error: "" });
    try {
      const raw = await this.call(method, {
        projectId: this.projectId,
        preparationId: this.preparationId,
      });
      if (generation !== this.generation) return;
      const operation =
        raw === null &&
        ["object.fileImportOperation", "object.importOperation"].includes(
          method,
        )
          ? null
          : parseFileImportOperation(raw, this.projectId, this.preparationId);
      this.update({ loaded: true, busy: false, operation });
    } catch (error) {
      if (generation === this.generation)
        this.update({
          busy: false,
          error: `${error instanceof Error ? error.message : String(error)}。可查询状态或按原准备记录重试。`,
        });
    }
  }
}

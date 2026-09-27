import {
  parseImportHistory,
  parseImportHistoryReceipt,
  type ImportHistoryEntry,
  type ImportHistoryReceipt,
} from "../../shared/object-import-history";

type Api = (method: string, input: unknown) => Promise<unknown>;
interface State {
  entries: ImportHistoryEntry[];
  next: string | null;
  loaded: boolean;
  loading: boolean;
  reading: boolean;
  selected: ImportHistoryEntry | null;
  receipt: ImportHistoryReceipt | null;
  error: string;
}
const message = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

export class ObjectImportHistory {
  private state: State = {
    entries: [],
    next: null,
    loaded: false,
    loading: false,
    reading: false,
    selected: null,
    receipt: null,
    error: "",
  };
  private listGeneration = 0;
  private readGeneration = 0;
  private listeners = new Set<() => void>();
  constructor(
    readonly projectId: string | undefined,
    private readonly call: Api,
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
    this.listGeneration++;
    this.readGeneration++;
    this.update({ loading: false, reading: false });
  }
  async refresh(more = false, prepared?: ImportHistoryEntry) {
    if (!this.projectId || (more && (this.state.loading || !this.state.next)))
      return;
    const generation = ++this.listGeneration;
    const after = more ? this.state.next! : undefined;
    if (!more) this.readGeneration++;
    this.update({
      loading: true,
      error: "",
      ...(!more ? { reading: false, selected: null, receipt: null } : {}),
    });
    try {
      const raw = await this.call("object.importPreparations", {
        projectId: this.projectId,
        ...(after ? { after } : {}),
      });
      if (generation !== this.listGeneration) return;
      const page = parseImportHistory(raw, this.projectId, after);
      this.update({
        entries: more ? [...this.state.entries, ...page.entries] : page.entries,
        next: page.next,
        loaded: true,
        loading: false,
      });
      if (prepared && !more) await this.read(prepared);
    } catch (error) {
      if (generation === this.listGeneration)
        this.update({ loading: false, error: message(error) });
    }
  }
  async open(entry: ImportHistoryEntry) {
    if (
      !this.projectId ||
      this.state.loading ||
      !this.state.entries.includes(entry)
    )
      return;
    await this.read(entry);
  }
  private async read(entry: ImportHistoryEntry) {
    if (!this.projectId) return;
    const generation = ++this.readGeneration;
    this.update({ selected: entry, receipt: null, reading: true, error: "" });
    try {
      const files = entry.kind === "files";
      const raw = await this.call(
        files
          ? "object.getFileImportPreparation"
          : "object.getImportPreparation",
        {
          [files ? "targetProjectId" : "projectId"]: this.projectId,
          preparationId: entry.preparationId,
        },
      );
      if (generation !== this.readGeneration) return;
      if (raw === null)
        throw new Error("准备记录已不存在，请刷新历史；当前草稿不受影响");
      const receipt = parseImportHistoryReceipt(raw, this.projectId, entry);
      this.update({ receipt, reading: false });
    } catch (error) {
      if (generation === this.readGeneration)
        this.update({ reading: false, error: message(error) });
    }
  }
}

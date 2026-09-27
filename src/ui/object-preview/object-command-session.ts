import type {
  ObjectCatalogRecord,
  ObjectRegistrationDraft,
} from "../../shared/object-catalog";
import {
  parseObjectCommand,
  parseObjectCommandResult,
  type ObjectCommand,
  type ObjectCommandResult,
} from "../../shared/object-command";

export type ObjectCommandTarget =
  | { kind: "register"; projectId: string }
  | { kind: "update" | "capture"; object: ObjectCatalogRecord };
type Api = (method: string, input?: unknown) => Promise<unknown>;
interface State {
  phase: "editing" | "submitting" | "failed" | "succeeded";
  locked: boolean;
  draft: ObjectRegistrationDraft;
  error: string;
  result: ObjectCommandResult | null;
}

export class ObjectCommandSession {
  readonly projectId: string;
  readonly kind: ObjectCommandTarget["kind"];
  private readonly original?: ObjectCatalogRecord;
  private command?: ObjectCommand;
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State;

  constructor(
    target: ObjectCommandTarget,
    private readonly api: Api,
    private readonly requestId: () => string = () => crypto.randomUUID(),
  ) {
    this.kind = target.kind;
    this.projectId =
      target.kind === "register" ? target.projectId : target.object.projectId;
    this.original =
      target.kind === "register" ? undefined : structuredClone(target.object);
    this.state = {
      phase: "editing",
      locked: false,
      draft: this.original
        ? structuredClone({
            name: this.original.name,
            category: this.original.category,
            tags: this.original.tags,
            thumbnailPath: this.original.thumbnailPath,
            parentObjectId: this.original.parentObjectId,
            components: this.original.components,
            files: this.original.files,
            references: this.original.references,
          })
        : {
            name: "",
            category: "其他",
            tags: [],
            thumbnailPath: null,
            parentObjectId: null,
            components: [],
            files: [],
            references: [],
          },
      error: "",
      result: null,
    };
  }

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private set(patch: Partial<State>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }
  edit = (patch: Partial<ObjectRegistrationDraft>) => {
    if (this.state.locked || this.kind === "capture") return;
    this.set({
      draft: structuredClone({ ...this.state.draft, ...patch }),
      error: "",
    });
  };
  cancel = () => {
    this.generation++;
    if (this.state.phase === "submitting")
      this.set({ phase: "failed", error: "已停止等待，原请求仍可能完成" });
  };

  async submit(): Promise<ObjectCommandResult | null> {
    if (this.state.phase === "succeeded") return this.state.result;
    if (this.state.phase === "submitting") return null;
    const generation = ++this.generation;
    try {
      if (!this.command) {
        const input = {
          projectId: this.projectId,
          requestId: this.requestId(),
        };
        const baseline = this.original && {
          objectId: this.original.id,
          expectedRevision: this.original.revision,
        };
        this.command = parseObjectCommand(
          this.kind === "register"
            ? {
                method: "object.register",
                input: { ...input, ...this.state.draft },
              }
            : this.kind === "update"
              ? {
                  method: "object.updateRegistration",
                  input: { ...input, ...baseline, ...this.state.draft },
                }
              : {
                  method: "object.captureVersion",
                  input: { ...input, ...baseline },
                },
        );
      }
      // A failed transport can follow a commit; keep both payload and request ID for retries.
      this.set({ phase: "submitting", locked: true, error: "" });
      const response = await this.api(this.command.method, this.command.input);
      if (generation !== this.generation) return null;
      const result = parseObjectCommandResult(
        response,
        this.command,
        this.original,
      );
      this.set({ phase: "succeeded", result });
      return result;
    } catch (error: unknown) {
      if (generation !== this.generation) return null;
      this.set({
        phase: "failed",
        error: error instanceof Error ? error.message : String(error),
      });
      return null;
    }
  }
}

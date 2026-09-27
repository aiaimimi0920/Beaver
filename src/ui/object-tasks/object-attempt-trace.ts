import { z } from "zod";

const identity = z.strictObject({
  projectId: z.string(),
  runId: z.string(),
  attemptId: z.string(),
});
const responseSchema = z
  .strictObject({
    request: identity,
    entries: z
      .array(
        z.strictObject({
          sequence: z.number().int().positive(),
          operation: z.enum([
            "commandExecution",
            "fileChange",
            "mcpToolCall",
            "dynamicToolCall",
            "webSearch",
            "turn",
            "provider",
          ]),
          phase: z.enum(["started", "completed", "error"]),
          status: z.enum([
            "completed",
            "failed",
            "declined",
            "inProgress",
            "unknown",
            "interrupted",
            "retrying",
          ]),
        }),
      )
      .max(256),
    truncated: z.boolean(),
  })
  .refine((trace) =>
    trace.entries.every((entry, index) => entry.sequence === index + 1),
  );
type Response = z.infer<typeof responseSchema>;
interface State {
  response: Response | null;
  loading: boolean;
  error: string;
}

export class ObjectAttemptTrace {
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = { response: null, loading: false, error: "" };
  constructor(
    private readonly request: z.infer<typeof identity>,
    private readonly api: (method: string, input: unknown) => Promise<unknown>,
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
  cancel = () => {
    this.generation++;
    this.set({ response: null, loading: false, error: "" });
  };
  async refresh() {
    const generation = ++this.generation;
    this.set({ response: null, loading: true, error: "" });
    try {
      const response = responseSchema.parse(
        await this.api("objectTask.attemptTrace", this.request),
      );
      if (generation !== this.generation) return;
      if (
        response.request.projectId !== this.request.projectId ||
        response.request.runId !== this.request.runId ||
        response.request.attemptId !== this.request.attemptId
      )
        throw new Error("操作轨迹与所选执行不匹配");
      this.set({ response, loading: false, error: "" });
    } catch (error) {
      if (generation === this.generation)
        this.set({ response: null, loading: false, error: String(error) });
    }
  }
}

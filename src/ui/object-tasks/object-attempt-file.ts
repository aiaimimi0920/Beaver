import { z } from "zod";
import { frozenMediaSchemas } from "../../shared/frozen-media";

const requestSchema = z.strictObject({
  projectId: z.string(),
  runId: z.string(),
  attemptId: z.string(),
  checkpoint: z.enum(["input", "output"]),
  path: z.string(),
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
});
const responseSchema = z.strictObject({
  request: requestSchema,
  byteCount: z.number().int().nonnegative(),
  content: z.discriminatedUnion("kind", [
    z.strictObject({
      kind: z.literal("text"),
      text: z.string().max(1024 * 1024),
    }),
    ...frozenMediaSchemas(1024 * 1024),
    z.strictObject({ kind: z.literal("binary") }),
    z.strictObject({ kind: z.literal("tooLarge") }),
  ]),
});
type Request = z.infer<typeof requestSchema>;
type Response = z.infer<typeof responseSchema>;
interface State {
  request: Request | null;
  response: Response | null;
  loading: boolean;
  error: string;
}

export class ObjectAttemptFile {
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = {
    request: null,
    response: null,
    loading: false,
    error: "",
  };
  constructor(
    private readonly identity: Pick<
      Request,
      "projectId" | "runId" | "attemptId"
    >,
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
    this.set({ request: null, response: null, loading: false, error: "" });
  };
  async open(checkpoint: Request["checkpoint"], path: string, sha256: string) {
    const generation = ++this.generation;
    const request = { ...this.identity, checkpoint, path, sha256 };
    this.set({ request, response: null, loading: true, error: "" });
    try {
      const response = responseSchema.parse(
        await this.api("objectTask.attemptFile", request),
      );
      if (generation !== this.generation) return;
      if (
        !(Object.keys(request) as (keyof Request)[]).every(
          (key) => response.request[key] === request[key],
        )
      )
        throw new Error("冻结文件响应与所选文件不匹配");
      this.set({ request, response, loading: false, error: "" });
    } catch (error) {
      if (generation !== this.generation) return;
      this.set({
        request,
        response: null,
        loading: false,
        error: String(error),
      });
    }
  }
}

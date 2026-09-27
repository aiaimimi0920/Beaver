import { z } from "zod";
import { frozenMediaSchemas } from "../../shared/frozen-media";
import { objectIdSchema } from "../../shared/object-catalog";

const requestSchema = z.strictObject({
  projectId: objectIdSchema,
  objectId: objectIdSchema,
  versionId: objectIdSchema,
  path: z.string().min(1),
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
});
const responseSchema = z.strictObject({
  request: requestSchema,
  byteCount: z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
  content: z.discriminatedUnion("kind", [
    z.strictObject({
      kind: z.literal("text"),
      text: z.string().max(8 * 1024 * 1024),
    }),
    ...frozenMediaSchemas(8 * 1024 * 1024),
    z.strictObject({ kind: z.literal("unsupported") }),
    z.strictObject({ kind: z.literal("tooLarge") }),
  ]),
});
export type VersionFileRequest = z.infer<typeof requestSchema>;
export type VersionFileResponse = z.infer<typeof responseSchema>;
interface State {
  request: VersionFileRequest | null;
  response: VersionFileResponse | null;
  loading: boolean;
  error: string;
}

export class ObjectVersionFile {
  private generation = 0;
  private listeners = new Set<() => void>();
  private state: State = {
    request: null,
    response: null,
    loading: false,
    error: "",
  };
  constructor(
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
  async open(input: VersionFileRequest) {
    const generation = ++this.generation;
    const request = { ...input };
    this.set({ request, response: null, loading: true, error: "" });
    try {
      const value = await this.api(
        "object.versionFile",
        requestSchema.parse(request),
      );
      if (generation !== this.generation) return;
      const response = responseSchema.parse(value);
      if (
        !(Object.keys(request) as (keyof VersionFileRequest)[]).every(
          (key) => response.request[key] === request[key],
        )
      )
        throw new Error("版本文件响应与所选项目、对象、版本或文件不匹配");
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

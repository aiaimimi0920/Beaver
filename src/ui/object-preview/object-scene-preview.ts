import { z } from "zod";

const versionTargetSchema = z
  .object({
    projectId: z.string(),
    objectId: z.string(),
    versionId: z.string(),
    path: z.string(),
    sha256: z.string(),
  })
  .strict();
export const targetSchema = z.union([
  versionTargetSchema,
  z
    .object({
      projectId: z.string(),
      runId: z.string(),
      attemptId: z.string(),
      checkpoint: z.enum(["input", "output"]),
      path: z.string(),
      sha256: z.string(),
    })
    .strict(),
]);
export type PreviewResolution = "540p" | "720p" | "1080p";
export type SceneTarget = z.infer<typeof targetSchema>;
const resultSchema = z.object({
  target: targetSchema,
  resolution: z
    .object({
      width: z.number().int().positive(),
      height: z.number().int().positive(),
    })
    .optional(),
  sourceDigest: z.string(),
  projectConfig: z.string(),
  integrityError: z.string().nullable(),
  run: z.object({
    id: z.string(),
    projectId: z.string(),
    kind: z.literal("objectPreview"),
    status: z.string(),
    phase: z.string(),
    error: z.string().nullable(),
    engineVersion: z.string(),
    runnerVersion: z.string(),
    snapshotId: z.string(),
    createdAt: z.string(),
    finishedAt: z.string().nullable(),
    evidence: z.array(
      z.object({ id: z.string(), kind: z.string(), sha256: z.string() }),
    ),
  }),
});
export type SceneResult = z.infer<typeof resultSchema>;
type Call = (method: string, input: unknown) => Promise<unknown>;
export class ObjectScenePreview {
  private state: {
    result: SceneResult | null;
    busy: boolean;
    error: string;
    resolution: PreviewResolution;
    retryPending: boolean;
  } = {
    resolution: "540p",
    retryPending: false,
    result: null,
    busy: false,
    error: "",
  };
  private listeners = new Set<() => void>();
  private generation = 0;
  private reading = false;
  private refreshAgain = false;
  private closed = false;
  private requestId: string | null = null;
  constructor(
    readonly target: SceneTarget,
    private call: Call,
  ) {}
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private update(value: Partial<typeof this.state>) {
    if (this.closed) return;
    this.state = { ...this.state, ...value };
    this.listeners.forEach((listener) => listener());
  }
  private get method() {
    return "attemptId" in this.target
      ? "object.attemptScenePreview"
      : "object.scenePreview";
  }
  refresh = async () => {
    if (this.closed || this.state.busy) return;
    if (this.reading) {
      this.refreshAgain = true;
      return;
    }
    this.reading = true;
    const generation = this.generation;
    try {
      const raw = await this.call(this.method + ".get", this.target);
      const result = raw === null ? null : resultSchema.parse(raw);
      if (
        result &&
        (Object.keys(result.target).length !==
          Object.keys(this.target).length ||
          Object.entries(this.target).some(
            ([key, value]) => Reflect.get(result.target, key) !== value,
          ))
      )
        throw new Error("预览结果属于其他版本或尝试");
      if (result && result.run.projectId !== this.target.projectId)
        throw new Error("预览结果属于其他项目");
      if (generation === this.generation) this.update({ result, error: "" });
    } catch (error) {
      if (generation === this.generation) this.update({ error: String(error) });
    } finally {
      this.reading = false;
      if (this.refreshAgain) {
        this.refreshAgain = false;
        void this.refresh();
      }
    }
  };
  setResolution = (resolution: PreviewResolution) => {
    if (
      this.closed ||
      this.state.busy ||
      this.requestId ||
      ["queued", "running"].includes(this.state.result?.run.status ?? "")
    )
      return;
    this.update({ resolution });
  };
  render = async () => {
    if (this.closed || this.state.busy) return;
    this.generation++;
    this.requestId ??= crypto.randomUUID();
    this.update({ busy: true, error: "" });
    try {
      await this.call(this.method + ".run", {
        projectId: this.target.projectId,
        target: this.target,
        requestId: this.requestId,
        resolution: this.state.resolution,
      });
      this.requestId = null;
      this.update({ busy: false, retryPending: false });
      await this.refresh();
    } catch (error) {
      this.update({
        busy: false,
        retryPending: this.requestId !== null,
        error: String(error),
      });
    }
  };
  cancel = async () => {
    const run = this.state.result?.run;
    if (!run || this.closed || this.state.busy) return;
    this.generation++;
    this.update({ busy: true, error: "" });
    try {
      await this.call("validation.run.cancel", {
        projectId: this.target.projectId,
        runId: run.id,
        requestId: crypto.randomUUID(),
      });
      this.update({ busy: false });
      await this.refresh();
    } catch (error) {
      this.update({ busy: false, error: String(error) });
    }
  };
  close = () => {
    this.closed = true;
    this.generation++;
    this.listeners.clear();
  };
}

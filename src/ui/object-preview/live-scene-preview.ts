import { z } from "zod";
import { requestPreviewPick } from "./preview-picking";
import { beforeCloseDeadline, confirmPreviewClosed } from "./preview-close";
import {
  previewPickSchema,
  type PreviewRectangle,
} from "../../shared/preview-pick";
import {
  previewSelectionSchema,
  type PreviewSelection,
} from "../../shared/preview-selection";

const vector = z.array(z.number().finite());
export const frameSchema = z.object({
  sessionId: z.string(),
  sequence: z.number().int().positive(),
  revision: z.number().int().nonnegative(),
  frozen: z.boolean().default(false),
  width: z.number().int().positive(),
  height: z.number().int().positive(),
  sha256: z.string().regex(/^[a-f0-9]{64}$/),
  dataUrl: z.string().startsWith("data:image/png;base64,"),
  camera: z.object({
    transform: z.array(vector.length(3)).length(4),
    projection: z.array(vector.length(4)).length(4),
    near: z.number(),
    far: z.number(),
    mode: z.number(),
  }),
  engine: z.string(),
  picking: z
    .object({
      capability: z.literal("frozen-static-mesh-ray"),
      triangles: z.number().int().min(0).max(100000),
      skipped: z.number().int().nonnegative(),
    })
    .partial()
    .optional(),
  picks: z.array(previewPickSchema).max(32).optional(),
});
const responseSchema = z.object({
  projectId: z.string(),
  runId: z.string(),
  snapshotId: z.string(),
  sessionId: z.string(),
  status: z.enum(["starting", "ready", "closed"]),
  error: z.string().nullable(),
  frame: frameSchema.nullable(),
});
export type LiveFrame = z.infer<typeof frameSchema>;
export const resolutions = {
  "540p": { width: 960, height: 540 },
  "720p": { width: 1280, height: 720 },
  "1080p": { width: 1920, height: 1080 },
} as const;
export type CameraView = {
  yaw: number;
  pitch: number;
  panX: number;
  panY: number;
  zoom: number;
};
export const initialCamera = (): CameraView => ({
  yaw: 0,
  pitch: 0,
  panX: 0,
  panY: 0,
  zoom: 0,
});
type Call = (method: string, input: unknown) => Promise<unknown>;

export class LiveScenePreview {
  private state: {
    status: string;
    error: string;
    frame: LiveFrame | null;
    revision: number;
    frozen: boolean;
    capturePending: boolean;
    pickPending: boolean;
    closeStatus: "idle" | "pending" | "failed" | "confirmed";
    closeError: string;
    resolution: keyof typeof resolutions | null;
  } = {
    status: "idle",
    error: "",
    frame: null,
    revision: 0,
    frozen: false,
    capturePending: false,
    pickPending: false,
    closeStatus: "idle",
    closeError: "",
    resolution: null,
  };
  private listeners = new Set<() => void>();
  private sessionId: string | null = null;
  private requestId = crypto.randomUUID();
  private closed = false;
  private reading: Promise<void> | null = null;
  private openRequested = false;
  private closeWork: Promise<boolean> | null = null;
  private sending = false;
  private acknowledged = 0;
  private viewError = "";
  private desired = initialCamera();
  private captureRequest: {
    requestId: string;
    revision: number;
    selection?: PreviewSelection;
  } | null = null;
  async capture(selection?: PreviewSelection) {
    if (this.state.pickPending) throw new Error("PREVIEW_PICK_PENDING");
    if (
      this.closed ||
      this.state.closeStatus !== "idle" ||
      !this.sessionId ||
      this.state.status !== "ready" ||
      this.state.frame?.revision !== this.state.revision
    )
      throw new Error("PREVIEW_VIEW_PENDING");
    if (!this.captureRequest && selection) {
      selection = previewSelectionSchema.parse(selection);
      if (
        !this.state.frame.frozen ||
        selection.sequence !== this.state.frame.sequence ||
        selection.sha256 !== this.state.frame.sha256
      )
        throw new Error("PREVIEW_SELECTION_FRAME_MISMATCH");
    }
    this.captureRequest ??= {
      requestId: crypto.randomUUID(),
      revision: this.state.revision,
      ...(selection ? { selection: structuredClone(selection) } : {}),
    };
    this.update({ capturePending: true });
    let result: unknown;
    try {
      result = await this.call("validation.preview.capture", {
        projectId: this.projectId,
        runId: this.runId,
        sessionId: this.sessionId,
        ...this.captureRequest,
      });
    } catch (error) {
      if (
        /PREVIEW_(REVISION_CONFLICT|VIEW_PENDING|SELECTION_[A-Z_]+|SAVED_(FRAME|STORAGE)_LIMIT|SOURCE_[A-Z_]+|FRAME_[A-Z_]+|REQUIRES_COMPLETED_CAPTURE)/.test(
          String(error),
        )
      ) {
        this.captureRequest = null;
        this.update({ capturePending: false });
      }
      throw error;
    }
    this.captureRequest = null;
    this.update({ capturePending: false });
    return result;
  }
  constructor(
    readonly projectId: string,
    readonly runId: string,
    readonly snapshotId: string,
    private call: Call,
  ) {}
  async pick(point: { x: number; y: number }, rectangle?: PreviewRectangle) {
    const frame = this.state.frame;
    if (
      !frame?.frozen ||
      this.state.revision !== frame.revision ||
      this.state.pickPending ||
      this.state.capturePending ||
      this.closed ||
      this.state.closeStatus !== "idle"
    )
      throw new Error("PREVIEW_PICK_FRAME_MISMATCH");
    this.update({ pickPending: true });
    try {
      return await requestPreviewPick(
        this.call,
        this.projectId,
        frame,
        point,
        () =>
          !this.closed &&
          this.state.closeStatus === "idle" &&
          this.state.revision === frame.revision &&
          this.state.frame?.sha256 === frame.sha256 &&
          this.state.frame?.sequence === frame.sequence,
        rectangle,
      );
    } finally {
      this.update({ pickPending: false });
    }
  }
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
  refresh(): Promise<void> {
    if (
      this.closed ||
      this.state.closeStatus !== "idle" ||
      this.state.status === "closed"
    )
      return Promise.resolve();
    return (this.reading ?? this.beginRead()).catch((error: unknown) => {
      if (this.state.closeStatus === "idle")
        this.update({ error: String(error) });
    });
  }
  private beginRead() {
    this.reading = this.read().finally(() => {
      this.reading = null;
    });
    return this.reading;
  }
  private async read() {
    if (!this.sessionId) this.openRequested = true;
    const raw = await this.call(
      this.sessionId ? "validation.preview.read" : "validation.preview.open",
      this.sessionId
        ? { projectId: this.projectId, sessionId: this.sessionId }
        : {
            projectId: this.projectId,
            runId: this.runId,
            requestId: this.requestId,
          },
    );
    const result = responseSchema.parse(raw);
    if (
      result.projectId !== this.projectId ||
      result.runId !== this.runId ||
      result.snapshotId !== this.snapshotId ||
      (this.sessionId && result.sessionId !== this.sessionId)
    )
      throw new Error("PREVIEW_SESSION_MISMATCH");
    this.sessionId = result.sessionId;
    if (this.closed) {
      await this.release();
      return;
    }
    if (this.state.closeStatus !== "idle") return;
    let frame = result.frame;
    if (frame && frame.sessionId !== result.sessionId)
      throw new Error("PREVIEW_FRAME_SESSION_MISMATCH");
    if (frame && this.state.frame && frame.sequence < this.state.frame.sequence)
      throw new Error("PREVIEW_STALE_FRAME");
    const size = this.state.resolution && resolutions[this.state.resolution];
    if (
      frame &&
      (frame.revision !== this.state.revision ||
        frame.frozen !== this.state.frozen ||
        (size && (frame.width !== size.width || frame.height !== size.height)))
    )
      frame = this.state.frame;
    const resolution =
      this.state.resolution ??
      (Object.keys(resolutions) as (keyof typeof resolutions)[]).find(
        (key) =>
          resolutions[key].width === frame?.width &&
          resolutions[key].height === frame?.height,
      ) ??
      null;
    this.update({
      status: result.status,
      error: result.error || this.viewError,
      frame,
      resolution,
    });
    if (result.status === "ready" && this.acknowledged < this.state.revision)
      void this.sendView();
  }
  setCamera(camera: CameraView) {
    if (
      this.state.capturePending ||
      this.state.frozen ||
      this.closed ||
      this.state.closeStatus !== "idle" ||
      this.state.status !== "ready" ||
      !this.sessionId ||
      !this.state.frame
    )
      return;
    this.desired = camera;
    this.update({ revision: this.state.revision + 1 });
    void this.sendView();
  }
  setResolution(resolution: keyof typeof resolutions) {
    if (
      this.state.capturePending ||
      this.state.frozen ||
      this.closed ||
      this.state.closeStatus !== "idle" ||
      this.state.status !== "ready" ||
      !this.sessionId ||
      !Object.hasOwn(resolutions, resolution) ||
      resolution === this.state.resolution
    )
      return;
    this.update({ resolution, revision: this.state.revision + 1 });
    void this.sendView();
  }
  setFrozen(frozen: boolean) {
    if (
      this.state.capturePending ||
      this.state.pickPending ||
      this.closed ||
      this.state.closeStatus !== "idle" ||
      this.state.status !== "ready" ||
      !this.sessionId ||
      !this.state.frame ||
      frozen === this.state.frozen
    )
      return;
    this.update({ frozen, revision: this.state.revision + 1 });
    void this.sendView();
  }
  private async sendView() {
    if (this.sending || this.closed || this.state.closeStatus !== "idle")
      return;
    this.sending = true;
    try {
      let revision: number;
      do {
        revision = this.state.revision;
        await this.call("validation.preview.view", {
          projectId: this.projectId,
          sessionId: this.sessionId,
          revision,
          frozen: this.state.frozen,
          camera: this.desired,
          width: this.state.resolution
            ? resolutions[this.state.resolution].width
            : this.state.frame!.width,
          height: this.state.resolution
            ? resolutions[this.state.resolution].height
            : this.state.frame!.height,
        });
        this.acknowledged = revision;
        this.viewError = "";
        if (this.state.closeStatus === "idle") this.update({ error: "" });
      } while (
        !this.closed &&
        this.state.closeStatus === "idle" &&
        revision !== this.state.revision
      );
    } catch (error) {
      this.viewError = String(error);
      if (this.state.closeStatus === "idle")
        this.update({ error: this.viewError });
    } finally {
      this.sending = false;
    }
  }
  private async release() {
    if (this.sessionId)
      await this.call("validation.preview.close", {
        projectId: this.projectId,
        sessionId: this.sessionId,
      });
  }
  requestClose(): Promise<boolean> {
    if (this.closed) return Promise.resolve(false);
    if (this.state.closeStatus === "confirmed") return Promise.resolve(true);
    if (this.closeWork) return this.closeWork;
    this.update({ closeStatus: "pending", closeError: "" });
    this.closeWork = this.confirmClose().finally(() => {
      this.closeWork = null;
    });
    return this.closeWork;
  }
  private async confirmClose() {
    const deadline = Date.now() + 10_000;
    try {
      if (this.reading)
        await beforeCloseDeadline(
          this.reading.catch(() => undefined),
          deadline,
        );
      // Recover only an open that was already sent, using its original request.
      if (!this.sessionId && this.openRequested)
        await beforeCloseDeadline(this.beginRead(), deadline);
      if (this.sessionId)
        await confirmPreviewClosed(
          this.call,
          this.projectId,
          this.sessionId,
          deadline,
        );
      this.update({ closeStatus: "confirmed", status: "closed", error: "" });
      return !this.closed;
    } catch (error) {
      this.update({ closeStatus: "failed", closeError: String(error) });
      return false;
    }
  }
  close() {
    this.closed = true;
    this.listeners.clear();
    if (this.state.closeStatus !== "confirmed")
      void this.release().catch(() => undefined);
  }
}

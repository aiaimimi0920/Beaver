import {
  previewPickSchema,
  type PreviewRectangle,
} from "../../shared/preview-pick";
import type { LiveFrame } from "./live-scene-preview";

export async function requestPreviewPick(
  call: (method: string, input: unknown) => Promise<unknown>,
  projectId: string,
  frame: LiveFrame,
  point: { x: number; y: number },
  current: () => boolean,
  rectangle?: PreviewRectangle,
) {
  const input = {
    projectId,
    sessionId: frame.sessionId,
    revision: frame.revision,
    sequence: frame.sequence,
    sha256: frame.sha256,
    point,
    ...(rectangle ? { rectangle } : {}),
    requestId: crypto.randomUUID(),
  };
  const deadline = Date.now() + 15000;
  while (current()) {
    const response = await call("validation.preview.pick", input);
    if (!current()) throw new Error("PREVIEW_PICK_FRAME_MISMATCH");
    if (
      typeof response !== "object" ||
      response === null ||
      !("status" in response)
    )
      throw new Error("PREVIEW_PICK_INVALID");
    if (response.status === "ready" && "result" in response) {
      const result = previewPickSchema.parse(response.result);
      if (
        result.requestId !== input.requestId ||
        result.sessionId !== frame.sessionId ||
        result.revision !== frame.revision ||
        result.sequence !== frame.sequence ||
        result.sha256 !== frame.sha256 ||
        result.point.x !== point.x ||
        result.point.y !== point.y ||
        !!rectangle !== !!result.rectangle ||
        (rectangle &&
          (["x", "y", "width", "height"] as const).some(
            (key) => result.rectangle?.[key] !== rectangle[key],
          ))
      )
        throw new Error("PREVIEW_PICK_FRAME_MISMATCH");
      return result;
    }
    if (response.status !== "pending") throw new Error("PREVIEW_PICK_INVALID");
    if (Date.now() >= deadline) throw new Error("PREVIEW_PICK_TIMEOUT");
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  throw new Error("PREVIEW_PICK_FRAME_MISMATCH");
}

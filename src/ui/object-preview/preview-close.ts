import { z } from "zod";

const receiptSchema = z.object({
  projectId: z.string(),
  sessionId: z.string(),
  closed: z.boolean(),
});

// A timed-out request may still finish; keep its identity for a safe retry.
export async function beforeCloseDeadline<T>(
  work: Promise<T>,
  deadline: number,
): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      work,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new Error("PREVIEW_CLOSE_TIMEOUT")),
          Math.max(0, deadline - Date.now()),
        );
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

export async function confirmPreviewClosed(
  call: (method: string, input: unknown) => Promise<unknown>,
  projectId: string,
  sessionId: string,
  deadline: number,
) {
  while (Date.now() < deadline) {
    const receipt = receiptSchema.parse(
      await beforeCloseDeadline(
        call("validation.preview.close", { projectId, sessionId }),
        deadline,
      ),
    );
    if (receipt.projectId !== projectId || receipt.sessionId !== sessionId)
      throw new Error("PREVIEW_CLOSE_SESSION_MISMATCH");
    if (receipt.closed) return;
    await new Promise<void>((resolve) =>
      setTimeout(resolve, Math.min(100, Math.max(0, deadline - Date.now()))),
    );
  }
  throw new Error("PREVIEW_CLOSE_TIMEOUT");
}

import { z } from "zod";

const vector = z.tuple([
  z.number().finite(),
  z.number().finite(),
  z.number().finite(),
]);
export const previewRectangleSchema = z
  .strictObject({
    x: z.number().finite().min(0),
    y: z.number().finite().min(0),
    width: z.number().finite().positive(),
    height: z.number().finite().positive(),
  })
  .refine((r) => r.x + r.width <= 1 && r.y + r.height <= 1);
export type PreviewRectangle = z.infer<typeof previewRectangleSchema>;
export const previewPickSchema = z
  .strictObject({
    requestId: z.string().min(1).max(128),
    sessionId: z.string(),
    revision: z.number().int().nonnegative(),
    sequence: z.number().int().positive(),
    sha256: z.string().regex(/^[a-f0-9]{64}$/),
    point: z.strictObject({
      x: z.number().min(0).max(1),
      y: z.number().min(0).max(1),
    }),
    capability: z.enum([
      "frozen-static-mesh-ray",
      "frozen-static-mesh-frustum",
    ]),
    rectangle: previewRectangleSchema.optional(),
    nodePaths: z.array(z.string().min(1).max(4096)).max(32).optional(),
    truncated: z.boolean().optional(),
    triangles: z.number().int().min(0).max(100000),
    skipped: z.number().int().nonnegative(),
    hit: z
      .strictObject({
        nodePath: z.string().min(1).max(4096),
        triangle: z.number().int().nonnegative(),
        position: vector,
        normal: vector,
        distance: z.number().finite().nonnegative(),
      })
      .nullable(),
  })
  .refine((r) => {
    if (r.capability === "frozen-static-mesh-ray") {
      return (
        r.rectangle === undefined &&
        r.nodePaths === undefined &&
        r.truncated === undefined
      );
    }
    const box = r.rectangle;
    return (
      box !== undefined &&
      r.hit === null &&
      r.nodePaths !== undefined &&
      r.truncated !== undefined &&
      new Set(r.nodePaths).size === r.nodePaths.length &&
      (r.nodePaths.length === 0 || r.triangles > 0) &&
      r.nodePaths.every((p) => new TextEncoder().encode(p).length <= 4096) &&
      r.nodePaths.reduce(
        (sum, p) => sum + new TextEncoder().encode(p).length,
        0,
      ) <= 32768 &&
      r.point.x >= box.x &&
      r.point.x <= box.x + box.width &&
      r.point.y >= box.y &&
      r.point.y <= box.y + box.height
    );
  }, "PREVIEW_PICK_INVALID");
export type PreviewPick = z.infer<typeof previewPickSchema>;

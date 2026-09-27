import { z } from "zod";
import { imageRegionSchema } from "./object-rework-image";
import { previewPickSchema } from "./preview-pick";

export const previewSelectionSchema = z
  .strictObject({
    kind: z.literal("image-regions"),
    sequence: z.number().int().positive(),
    sha256: z.string().regex(/^[a-f0-9]{64}$/),
    regions: z.array(imageRegionSchema).min(1).max(8),
    prompt: z
      .string()
      .refine((s) => new TextEncoder().encode(s).length <= 4000),
    coordinateSpace: z.literal("normalized-image"),
    hitCapability: z.enum([
      "unavailable",
      "frozen-static-mesh-ray",
      "frozen-static-mesh",
    ]),
    picks: z
      .array(
        z.strictObject({
          region: z.number().int().min(0).max(7),
          result: previewPickSchema,
        }),
      )
      .max(8)
      .optional(),
  })
  .refine((value) => {
    const picks = value.picks ?? [];
    return (
      (value.hitCapability === "unavailable"
        ? picks.length === 0
        : picks.length > 0) &&
      new Set(picks.map((p) => p.region)).size === picks.length &&
      picks.every(({ region, result }) => {
        const r = value.regions[region];
        return (
          r &&
          (!result.rectangle ||
            (value.hitCapability === "frozen-static-mesh" &&
              result.rectangle.x === r.x &&
              result.rectangle.y === r.y &&
              result.rectangle.width === r.width &&
              result.rectangle.height === r.height)) &&
          result.sequence === value.sequence &&
          result.sha256 === value.sha256 &&
          result.point.x >= r.x &&
          result.point.x <= r.x + r.width &&
          result.point.y >= r.y &&
          result.point.y <= r.y + r.height
        );
      })
    );
  }, "PREVIEW_SELECTION_PICK_INVALID");
export type PreviewSelection = z.infer<typeof previewSelectionSchema>;

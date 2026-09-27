import { z } from "zod";

export const imageRegionSchema = z
  .strictObject({
    x: z.number().min(0).max(1),
    y: z.number().min(0).max(1),
    width: z.number().positive().max(1),
    height: z.number().positive().max(1),
    prompt: z
      .string()
      .refine((value) => new TextEncoder().encode(value).length <= 1000),
  })
  .refine(
    (region) => region.x + region.width <= 1 && region.y + region.height <= 1,
  );

export const objectReworkImageSchema = z
  .strictObject({
    path: z
      .string()
      .max(1024)
      .regex(/\.png$/i),
    sha256: z.string().regex(/^[a-f0-9]{64}$/),
    width: z.number().int().min(1).max(16384),
    height: z.number().int().min(1).max(16384),
    regions: z.array(imageRegionSchema).min(1).max(8),
  })
  .refine((image) => image.width * image.height <= 16_777_216);

export type ObjectReworkImage = z.infer<typeof objectReworkImageSchema>;
export type ImageRegion = z.infer<typeof imageRegionSchema>;

export function normalizedRegion(
  start: { x: number; y: number },
  end: { x: number; y: number },
): ImageRegion | null {
  if (![start.x, start.y, end.x, end.y].every(Number.isFinite)) return null;
  const clamp = (n: number) => Math.min(1, Math.max(0, n));
  const x = Math.min(clamp(start.x), clamp(end.x));
  const y = Math.min(clamp(start.y), clamp(end.y));
  const result = imageRegionSchema.safeParse({
    x,
    y,
    width: Math.max(clamp(start.x), clamp(end.x)) - x,
    height: Math.max(clamp(start.y), clamp(end.y)) - y,
    prompt: "",
  });
  return result.success ? result.data : null;
}

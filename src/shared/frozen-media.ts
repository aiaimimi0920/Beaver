import { z } from "zod";

export function frozenMediaSchemas(byteLimit: number) {
  const encoded = z
    .string()
    .max(4 * Math.ceil(byteLimit / 3))
    .regex(/^[A-Za-z0-9+/]*={0,2}$/)
    .refine((value) => value.length % 4 === 0);
  return [
    z.strictObject({
      kind: z.literal("image"),
      mime: z.enum(["image/png", "image/jpeg", "image/gif", "image/webp"]),
      base64: encoded,
    }),
    z.strictObject({
      kind: z.literal("audio"),
      mime: z.enum(["audio/wav", "audio/mpeg", "audio/ogg", "audio/flac"]),
      base64: encoded,
    }),
  ] as const;
}

export type FrozenMediaContent = z.infer<
  ReturnType<typeof frozenMediaSchemas>[number]
>;

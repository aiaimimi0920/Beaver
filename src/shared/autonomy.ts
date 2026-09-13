import { z } from "zod";

export const askRatioSchema = z.union([
  z.literal(0),
  z.literal(10),
  z.literal(30),
  z.literal(70),
  z.literal(100),
]);
export type AskRatio = z.infer<typeof askRatioSchema>;
export const autonomyLevels = [0, 10, 30, 70, 100] as const;
export const autonomyLabel = (ratio: number) =>
  ratio === 0 ? "完全自动" : `询问 ${ratio}%`;
export function shouldAutomate(ratio: number, importance: number): boolean {
  return ratio === 0 || (ratio < 100 && importance <= 100 - ratio);
}

export function automaticChoice(
  ratio: number,
  question: {
    importance?: number;
    recommended?: string;
    reason?: string;
    options?: { label: string }[] | null;
  },
): string | undefined {
  if (ratio === 100) return;
  if (
    !question.importance ||
    !question.reason?.trim() ||
    !question.recommended ||
    !question.options?.some((o) => o.label === question.recommended)
  )
    throw new Error(
      "请通过 beaver_ask_user 提供有效的推荐选项、importance 和 reason",
    );
  return shouldAutomate(ratio, question.importance)
    ? question.recommended
    : undefined;
}

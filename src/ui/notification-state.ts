export type NotificationTone = "success" | "info" | "warning" | "error";
export interface Notification {
  id: number;
  tone: NotificationTone;
  text: string;
  count: number;
}
export type NotificationInput = Pick<Notification, "tone" | "text">;

export function appendNotification(
  current: Notification[],
  next: Notification,
): Notification[] {
  if (!next.text.trim()) return current;
  const existing = current.find(
    (item) => item.tone === next.tone && item.text === next.text,
  );
  return existing
    ? current.map((item) =>
        item.id === existing.id ? { ...item, count: item.count + 1 } : item,
      )
    : [...current, next];
}

export function notificationLifetime(tone: NotificationTone): number | null {
  // Action errors have no other guaranteed record: keep them until dismissed.
  return tone === "error" ? null : tone === "warning" ? 4200 : 3200;
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

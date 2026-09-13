export function recoverableFailure(message: string): boolean {
  const text = message.toLowerCase();
  return (
    !["401", "403", "unauthorized", "quota", "context", "permission"].some(
      (s) => text.includes(s),
    ) &&
    [
      "connection reset",
      "connection closed",
      "stream disconnected",
      "temporarily unavailable",
      "502 bad gateway",
      "503 service unavailable",
    ].some((s) => text.includes(s))
  );
}

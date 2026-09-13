export function nativeTitlebarCommand(
  button: number,
  detail: number,
  inDragRegion: boolean,
  interactive: boolean,
): "startDragging" | "toggleMaximize" | null {
  if (button !== 0 || !inDragRegion || interactive) return null;
  return detail === 2 ? "toggleMaximize" : "startDragging";
}

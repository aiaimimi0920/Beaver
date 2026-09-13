import type { AssetFrame, ObserverView, Point } from "../../shared/asset-task";

export const defaultView: ObserverView = {
  target: [0, 0, 1],
  yaw: 0.65,
  pitch: 0.2,
  distance: 5,
  width: 720,
  height: 540,
  seq: 0,
};
export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

// Pointer coordinates are CSS pixels. The contained image may have letterboxing.
export function imagePoint(
  rect: Rect,
  width: number,
  height: number,
  x: number,
  y: number,
): Point | null {
  if (
    ![rect.left, rect.top, rect.width, rect.height, width, height, x, y].every(
      Number.isFinite,
    ) ||
    rect.width <= 0 ||
    rect.height <= 0 ||
    width <= 0 ||
    height <= 0
  )
    return null;
  const scale = Math.min(rect.width / width, rect.height / height);
  const w = width * scale;
  const h = height * scale;
  const px = (x - rect.left - (rect.width - w) / 2) / w;
  const py = (y - rect.top - (rect.height - h) / 2) / h;
  return px < 0 || px > 1 || py < 0 || py > 1 ? null : [px, py];
}

export function moveView(
  view: ObserverView,
  dx: number,
  dy: number,
  pan: boolean,
): ObserverView {
  if (!pan)
    return {
      ...view,
      yaw: view.yaw - dx * 0.008,
      pitch: Math.max(-1.5, Math.min(1.5, view.pitch + dy * 0.008)),
    };
  const scale = view.distance * 0.002;
  const right = [Math.cos(view.yaw), Math.sin(view.yaw), 0] as const;
  const up = [
    -Math.sin(view.yaw) * Math.sin(view.pitch),
    Math.cos(view.yaw) * Math.sin(view.pitch),
    Math.cos(view.pitch),
  ] as const;
  const axis = (i: 0 | 1 | 2) =>
    view.target[i] - right[i] * dx * scale + up[i] * dy * scale;
  return { ...view, target: [axis(0), axis(1), axis(2)] };
}

export function zoomView(view: ObserverView, delta: number): ObserverView {
  return {
    ...view,
    distance: Math.max(
      0.02,
      Math.min(
        100000,
        view.distance * Math.exp(Math.max(-1, Math.min(1, delta * 0.001))),
      ),
    ),
  };
}

export function newerFrame(
  previous: AssetFrame | undefined,
  next: AssetFrame,
): boolean {
  if (
    !previous ||
    previous.sessionId !== next.sessionId ||
    previous.generation !== next.generation
  )
    return true;
  return (
    previous.id !== next.id &&
    next.capturedAt >= previous.capturedAt &&
    next.viewRevision >= previous.viewRevision &&
    next.sceneRevision >= previous.sceneRevision
  );
}

// One command in flight and one replaceable latest target; never replay a drag backlog.
export class LatestView {
  private pending?: ObserverView;
  private active = false;
  private closed = false;
  constructor(
    private send: (view: ObserverView) => Promise<unknown>,
    private failed: (error: unknown) => void,
  ) {}
  set(view: ObserverView) {
    if (this.closed) return;
    this.pending = view;
    void this.flush();
  }
  dispose() {
    this.closed = true;
    this.pending = undefined;
  }
  private async flush() {
    if (this.active) return;
    this.active = true;
    try {
      while (this.pending && !this.closed) {
        const view = this.pending;
        this.pending = undefined;
        try {
          await this.send(view);
        } catch (error) {
          if (!this.closed) this.failed(error);
        }
      }
    } finally {
      this.active = false;
    }
  }
}

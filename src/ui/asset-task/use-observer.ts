import { useCallback, useEffect, useRef, useState } from "react";
import type {
  AssetFrame,
  ObserverStatus,
  ObserverView,
} from "../../shared/asset-task";
import { assetUrl, call } from "../api";
import { errorMessage } from "../notification-state";
import { defaultView, LatestView, newerFrame } from "./preview-controls";

export interface DisplayedFrame {
  frame: AssetFrame;
  url: string;
}

export function useObserver(id: string, finished: boolean) {
  const [status, setStatus] = useState<ObserverStatus | null>(null);
  const [display, setDisplay] = useState<DisplayedFrame | null>(null);
  const [error, setError] = useState("");
  const [now, setNow] = useState(Date.now());
  const view = useRef<ObserverView>({ ...defaultView });
  const queue = useRef<LatestView | null>(null);
  const session = useRef("");

  useEffect(() => {
    const clock = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(clock);
  }, []);
  useEffect(() => {
    if (finished) {
      setStatus(null);
      return;
    }
    let stopped = false;
    let timer = 0;
    let last: DisplayedFrame | null = null;
    const abort = new AbortController();
    const subscriber = `window-${id}`;
    const poll = async () => {
      let connected = false;
      try {
        const next = await call<ObserverStatus>("assetTask.status", {
          id,
          subscriber,
        });
        if (stopped) return;
        setStatus(next);
        connected = next.connected;
        if (next.connected && next.sessionId && next.view) {
          if (session.current !== next.sessionId) {
            session.current = next.sessionId;
            view.current = next.view;
            queue.current?.dispose();
            const sessionId = next.sessionId;
            queue.current = new LatestView(
              (target) =>
                call("assetTask.view", { id, sessionId, view: target }),
              (failure) => setError(errorMessage(failure)),
            );
          } else if (next.view.seq > view.current.seq) view.current = next.view;
        } else {
          queue.current?.dispose();
          queue.current = null;
          session.current = "";
        }
        const frame = next.frame;
        if (
          next.connected &&
          frame &&
          frame.sessionId === next.sessionId &&
          frame.generation === next.generation &&
          newerFrame(last?.frame, frame)
        ) {
          const response = await fetch(
            assetUrl(
              "asset-task",
              `${id}/${frame.sessionId}/frames/${frame.id}`,
            ),
            { signal: abort.signal, cache: "no-store" },
          );
          if (!response.ok) throw new Error("画面已更新，正在获取下一帧");
          const blob = await response.blob();
          if (blob.size > 4 * 1024 * 1024) throw new Error("预览超过大小限制");
          const url = URL.createObjectURL(blob);
          try {
            const image = new Image();
            image.src = url;
            await image.decode();
            if (
              image.naturalWidth !== frame.width ||
              image.naturalHeight !== frame.height
            )
              throw new Error("画面尺寸与元数据不一致");
            if (stopped) {
              URL.revokeObjectURL(url);
              return;
            }
            const previous = last;
            last = { frame, url };
            setDisplay(last);
            if (previous) URL.revokeObjectURL(previous.url);
          } catch (failure) {
            URL.revokeObjectURL(url);
            throw failure;
          }
        }
        setError("");
      } catch (failure) {
        if (!stopped) {
          setError(errorMessage(failure));
          // A rotated frame cache or decode failure does not disconnect the view channel.
          if (!connected) {
            setStatus((previous) =>
              previous ? { ...previous, connected: false } : null,
            );
            queue.current?.dispose();
            queue.current = null;
            session.current = "";
          }
        }
      } finally {
        if (!stopped) timer = window.setTimeout(() => void poll(), 250);
        else
          void call("assetTask.status", {
            id,
            subscriber,
            release: true,
          }).catch(() => {});
      }
    };
    void poll();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      abort.abort();
      queue.current?.dispose();
      queue.current = null;
      session.current = "";
      if (last) URL.revokeObjectURL(last.url);
      setDisplay(null);
      void call("assetTask.status", { id, subscriber, release: true }).catch(
        () => {},
      );
    };
  }, [id, finished]);
  const changeView = useCallback((next: ObserverView) => {
    if (!queue.current) return;
    const target = { ...next, seq: view.current.seq + 1 };
    view.current = target;
    queue.current?.set(target);
  }, []);
  return { status, display, error, now, view, changeView };
}

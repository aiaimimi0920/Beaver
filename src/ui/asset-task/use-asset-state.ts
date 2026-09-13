import { useCallback, useEffect, useRef, useState } from "react";
import type { AssetWindowState } from "../../shared/asset-task";
import { call, type Run } from "../api";
import { useNotify } from "../Notifications";
import { errorMessage } from "../notification-state";

export function useAssetState(id: string) {
  const [state, setState] = useState<AssetWindowState | null>(null);
  const [error, setError] = useState("");
  const refreshRef = useRef<() => Promise<void>>(async () => {});
  const notify = useNotify();
  useEffect(() => {
    let disposed = false;
    let fetching = false;
    let pending = false;
    const refresh = async () => {
      if (fetching) {
        pending = true;
        return;
      }
      fetching = true;
      try {
        do {
          pending = false;
          const next = await call<AssetWindowState>("assetTask.state", { id });
          if (!disposed) {
            setState(next);
            setError("");
          }
        } while (pending && !disposed);
      } catch (failure) {
        if (!disposed) setError(errorMessage(failure));
      } finally {
        fetching = false;
      }
    };
    refreshRef.current = refresh;
    const unsubscribe = window.beaver.subscribe(() => void refresh());
    const timer = window.setInterval(() => void refresh(), 1500);
    void refresh();
    return () => {
      disposed = true;
      unsubscribe();
      window.clearInterval(timer);
    };
  }, [id]);
  const run: Run = useCallback(
    async (work, message) => {
      try {
        await work();
        await refreshRef.current();
        if (message) notify({ tone: "success", text: message });
      } catch (failure) {
        notify({ tone: "error", text: errorMessage(failure) });
      }
    },
    [notify],
  );
  return { state, error, run };
}

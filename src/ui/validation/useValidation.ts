import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../api";
import { useNotify } from "../Notifications";
import { errorMessage } from "../notification-state";
import type { Mutate } from "./types";

// One request at a time per selection; late results cannot replace another project/run.
export function useValidationQuery<T>(
  method: string,
  input: Record<string, string> | null,
  isRunning: (value: T) => boolean,
) {
  const key = JSON.stringify(input);
  const [result, setResult] = useState<{ key: string; data: T }>();
  const [failure, setFailure] = useState<{ key: string; error: string }>();
  const active = useRef(isRunning);
  active.current = isRunning;
  const trigger = useRef<() => void>(() => {});
  useEffect(() => {
    if (key === "null") return;
    let alive = true,
      inFlight = false,
      again = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const schedule = (delay: number) => {
      clearTimeout(timer);
      timer = setTimeout(() => void fetch(), delay);
    };
    async function fetch() {
      if (!alive) return;
      if (inFlight) {
        again = true;
        return;
      }
      inFlight = true;
      let polling = false;
      try {
        const data = await call<T>(method, JSON.parse(key) as unknown);
        if (!alive) return;
        setResult({ key, data });
        setFailure(undefined);
        polling = active.current(data);
      } catch (error) {
        if (alive) setFailure({ key, error: errorMessage(error) });
      } finally {
        inFlight = false;
        if (alive && (again || polling)) schedule(again ? 500 : 2000);
        again = false;
      }
    }
    trigger.current = () => schedule(250);
    const unsubscribe = window.beaver.subscribe(() => schedule(500));
    void fetch();
    return () => {
      alive = false;
      clearTimeout(timer);
      unsubscribe();
    };
  }, [key, method]);
  return {
    data: result?.key === key ? result.data : undefined,
    error: failure?.key === key ? failure.error : undefined,
    refresh: useCallback(() => trigger.current(), []),
  };
}

export function useValidationMutation(projectId: string, refresh: () => void) {
  const [pending, setPending] = useState(0);
  const notify = useNotify();
  const mutate: Mutate = useCallback(
    async <T>(method: string, input: Record<string, unknown> = {}) => {
      setPending((count) => count + 1);
      try {
        const value = await call<T>(method, {
          ...input,
          projectId,
          requestId: crypto.randomUUID(),
        });
        refresh();
        return value;
      } catch (error) {
        notify({ tone: "error", text: errorMessage(error) });
        throw error;
      } finally {
        setPending((count) => count - 1);
      }
    },
    [projectId, refresh, notify],
  );
  return { busy: pending > 0, mutate };
}

// Mutation failures have already been surfaced by useValidationMutation.
export const perform = (work: Promise<unknown>) => {
  void work.catch(() => {});
};

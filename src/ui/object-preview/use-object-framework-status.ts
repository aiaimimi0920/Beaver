import { useCallback, useEffect, useState } from "react";
import {
  parseObjectFrameworkStatus,
  type ObjectFrameworkStatus,
} from "../../shared/object-framework";
import { call } from "../api";
import { errorMessage } from "../notification-state";

export type FrameworkQuery =
  | { kind: "loading" }
  | { kind: "error"; message: string }
  | { kind: "loaded"; data: ObjectFrameworkStatus };

export function useObjectFrameworkStatus(projectId: string | undefined) {
  const [attempt, setAttempt] = useState(0);
  const [result, setResult] = useState<{
    projectId: string;
    query: FrameworkQuery;
  }>();
  const retry = useCallback(() => setAttempt((value) => value + 1), []);
  useEffect(() => {
    if (!projectId) return;
    let disposed = false;
    let sequence = 0;
    const refresh = async () => {
      const read = ++sequence;
      setResult({ projectId, query: { kind: "loading" } });
      try {
        const response = await call<unknown>("objectFramework.status", {
          projectId,
        });
        const data = parseObjectFrameworkStatus(response, projectId);
        if (!disposed && read === sequence)
          setResult({ projectId, query: { kind: "loaded", data } });
      } catch (failure: unknown) {
        if (!disposed && read === sequence)
          setResult({
            projectId,
            query: { kind: "error", message: errorMessage(failure) },
          });
      }
    };
    void refresh();
    const unsubscribe = window.beaver.subscribe(() => void refresh());
    return () => {
      disposed = true;
      unsubscribe();
    };
  }, [projectId, attempt]);
  const query: FrameworkQuery =
    result?.projectId === projectId && result
      ? result.query
      : { kind: "loading" };
  return { query, retry };
}

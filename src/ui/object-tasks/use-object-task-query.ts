import { useEffect, useMemo, useSyncExternalStore } from "react";
import { call } from "../api";
import {
  ObjectTaskQuery,
  type ObjectTaskQueryState,
} from "./object-task-query";

const emptySubscribe = () => () => {};
const emptySnapshot: ObjectTaskQueryState = { kind: "loading" };
const emptyGetSnapshot = () => emptySnapshot;

export function useObjectTaskQuery(projectId?: string) {
  const query = useMemo(
    () => (projectId ? new ObjectTaskQuery(projectId, call) : null),
    [projectId],
  );
  const state = useSyncExternalStore(
    query?.subscribe ?? emptySubscribe,
    query?.getSnapshot ?? emptyGetSnapshot,
    emptyGetSnapshot,
  );
  useEffect(() => {
    if (!query) return;
    const unsubscribe = window.beaver.subscribe(() => {
      void query.refresh();
    });
    void query.refresh();
    return () => {
      unsubscribe();
      query.cancel();
    };
  }, [query]);
  return {
    state,
    refresh: async () => {
      if (!query) return false;
      await query.refresh();
      return query.getSnapshot().kind === "ready";
    },
  };
}

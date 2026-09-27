import { useEffect, useMemo, useSyncExternalStore } from "react";
import { call } from "../api";
import { ObjectTaskWorkspace } from "./object-task-workspace";

export function useObjectTasks(projectId: string) {
  const query = useMemo(
    () => new ObjectTaskWorkspace(projectId, call),
    [projectId],
  );
  const state = useSyncExternalStore(query.subscribe, query.getSnapshot);
  useEffect(() => {
    const unsubscribe = window.beaver.subscribe(() => {
      void query.refresh();
    });
    void query.refresh();
    return () => {
      unsubscribe();
      query.cancel();
    };
  }, [query]);
  return { state, workspace: query, refresh: query.refresh };
}

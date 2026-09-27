import { useEffect, useRef, useState } from "react";
import { call } from "../api";
import { ObjectImportSession } from "./object-import-session";
import {
  browserDraftStorage,
  loadImportDraft,
  saveImportDraft,
} from "./object-import-draft";

export function useObjectImportDraft(projectId?: string) {
  const [loaded] = useState(() =>
    loadImportDraft(projectId, browserDraftStorage),
  );
  const [session] = useState(
    () => new ObjectImportSession(projectId, call, undefined, loaded.draft),
  );
  const [groupName, updateGroupName] = useState(loaded.draft?.groupName ?? "");
  const name = useRef(groupName);
  const [error, setError] = useState(loaded.error);
  useEffect(() => {
    const unsubscribe = session.subscribe(() => {
      setError(
        saveImportDraft(
          session.targetProjectId,
          browserDraftStorage,
          session.getSnapshot(),
          name.current,
        ),
      );
    });
    return () => {
      unsubscribe();
      session.cancel(true);
    };
  }, [session]);
  const setGroupName = (value: string) => {
    name.current = value;
    updateGroupName(value);
    setError(
      saveImportDraft(
        session.targetProjectId,
        browserDraftStorage,
        session.getSnapshot(),
        value,
      ),
    );
  };
  return {
    session,
    groupName,
    setGroupName,
    error,
    restored: Boolean(loaded.draft),
  };
}

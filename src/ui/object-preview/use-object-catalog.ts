import { useEffect, useState } from "react";
import { call } from "../api";
import {
  parseObjectCatalog,
  type ObjectCatalogRecord,
} from "../../shared/object-catalog";
export type { ObjectCatalogRecord } from "../../shared/object-catalog";

export type ObjectCatalogQuery =
  | { kind: "loading" }
  | { kind: "ready"; objects: ObjectCatalogRecord[] }
  | { kind: "error"; message: string };

export function useObjectCatalog(
  projectId: string | undefined,
  query: string,
  reload = 0,
) {
  const [state, setState] = useState<ObjectCatalogQuery>({ kind: "loading" });
  useEffect(() => {
    if (!projectId) {
      setState({ kind: "ready", objects: [] });
      return;
    }
    let active = true;
    setState({ kind: "loading" });
    call<unknown>("object.list", {
      projectId,
      query: query.trim() || undefined,
    })
      .then((value) => {
        if (active)
          setState({
            kind: "ready",
            objects: parseObjectCatalog(value, projectId),
          });
      })
      .catch((error: unknown) => {
        if (active) {
          setState({
            kind: "error",
            message: error instanceof Error ? error.message : String(error),
          });
        }
      });
    return () => {
      active = false;
    };
  }, [projectId, query, reload]);
  return state;
}

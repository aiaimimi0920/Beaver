import { useEffect, useState } from "react";
import { call } from "../api";

export interface ObjectCatalogRecord {
  id: string;
  projectId: string;
  name: string;
  components: Array<{ id: string; kind: string; name: string }>;
  files: Array<{ path: string; role: string }>;
  references: Array<{
    projectId: string;
    objectId: string;
    versionId?: string;
  }>;
  versions: Array<{ versionId: string; manifest: unknown }>;
}

export type ObjectCatalogQuery =
  | { kind: "loading" }
  | { kind: "ready"; objects: ObjectCatalogRecord[] }
  | { kind: "error"; message: string };

function validate(value: unknown): ObjectCatalogRecord[] {
  if (!Array.isArray(value)) throw new Error("对象查询返回格式无效");
  return value.map((item) => {
    if (!item || typeof item !== "object") throw new Error("对象记录无效");
    const record = item as Partial<ObjectCatalogRecord>;
    if (
      typeof record.id !== "string" ||
      typeof record.projectId !== "string" ||
      typeof record.name !== "string" ||
      !Array.isArray(record.components) ||
      !Array.isArray(record.files) ||
      !Array.isArray(record.references) ||
      !Array.isArray(record.versions)
    ) {
      throw new Error("对象记录字段不完整");
    }
    return record as ObjectCatalogRecord;
  });
}

export function useObjectCatalog(projectId: string | undefined, query: string) {
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
        if (active) setState({ kind: "ready", objects: validate(value) });
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
  }, [projectId, query]);
  return state;
}

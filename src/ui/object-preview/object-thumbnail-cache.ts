import {
  readFrozenObjectVersion,
  type ObjectCatalogRecord,
} from "../../shared/object-catalog";
import {
  ObjectScenePreview,
  type SceneResult,
  type SceneTarget,
} from "./object-scene-preview";

export function catalogSceneTarget(
  object: ObjectCatalogRecord,
): SceneTarget | null {
  const version = object.versions.at(-1);
  if (!version) return null;
  const manifest = readFrozenObjectVersion(object, version);
  const scene = manifest?.files.find((file) => /\.(tscn|scn)$/.test(file.path));
  return scene
    ? {
        projectId: object.projectId,
        objectId: object.id,
        versionId: version.versionId,
        path: scene.path,
        sha256: scene.sha256,
      }
    : null;
}

type Call = (method: string, input: unknown) => Promise<unknown>;
type Thumbnail = {
  result: SceneResult;
  image: SceneResult["run"]["evidence"][number];
};
let active = 0;
const waiting: (() => void)[] = [];
async function slot<T>(work: () => Promise<T>): Promise<T> {
  if (active >= 3) await new Promise<void>((resolve) => waiting.push(resolve));
  else active++;
  try {
    return await work();
  } finally {
    const next = waiting.shift();
    if (next) next();
    else active--;
  }
}

// Read-only: list browsing never requests a capture or owns the render lifetime.
export async function readCatalogThumbnail(
  target: SceneTarget,
  call: Call,
  signal: AbortSignal,
): Promise<Thumbnail | null> {
  return slot(async () => {
    if (signal.aborted) return null;
    const session = new ObjectScenePreview(target, call);
    const close = () => session.close();
    signal.addEventListener("abort", close, { once: true });
    try {
      await session.refresh();
      if (signal.aborted) return null;
      const { result, error } = session.getSnapshot();
      if (error) throw new Error(error);
      if (!result || result.run.status !== "completed") return null;
      if (result.integrityError) throw new Error(result.integrityError);
      const image = result.run.evidence.find((item) => item.kind === "image");
      if (!image) throw new Error("已完成预览缺少图像");
      return { result, image };
    } finally {
      signal.removeEventListener("abort", close);
      session.close();
    }
  });
}

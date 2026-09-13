import type { Feature } from "./types";
import { featureCategories, featureCategory } from "./game-design";
import { enginePackages, type EnginePackage } from "./engine-packages";

export type PlanningPackage =
  | EnginePackage
  | {
      id: string;
      kind: "source";
      name: string;
      category: string;
      description: string;
    };

export function planningPackages(
  features: readonly Feature[],
): PlanningPackage[] {
  return [
    ...features.map((f): PlanningPackage => ({
      id: f.id,
      kind: "source",
      name: f.name,
      category:
        featureCategories.find((c) => c.id === featureCategory(f))?.name ??
        "基础能力",
      description: f.description,
    })),
    ...enginePackages,
  ];
}

export function filterPackages(
  packages: readonly PlanningPackage[],
  search: string,
  category: string,
  kind: string,
  selected?: readonly string[],
) {
  const query = search.trim().toLowerCase();
  return packages.filter(
    (p) =>
      (!category || p.category === category) &&
      (!kind || p.kind === kind) &&
      (!selected || selected.includes(p.id)) &&
      `${p.name} ${p.description} ${p.id} ${p.kind === "planning" ? p.sourceKey : ""}`
        .toLowerCase()
        .includes(query),
  );
}

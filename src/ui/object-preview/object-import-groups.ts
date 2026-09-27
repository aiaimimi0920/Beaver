import type { FileGroup, ObjectImportState } from "./object-import-session";

export function addImportGroup(groups: FileGroup[], name: string): FileGroup[] {
  const trimmed = name.trim();
  if (!trimmed) return groups;
  let sequence = 1;
  while (groups.some((group) => group.id === `group-${sequence}`)) sequence++;
  return [...groups, { id: `group-${sequence}`, name: trimmed, paths: [] }];
}

export function assignImportGroup(
  state: ObjectImportState,
  path: string,
  groupId: string,
): FileGroup[] {
  if (
    !state.fileSnapshot?.files.some((file) => file.path === path) &&
    !state.selectedPaths.includes(path)
  )
    return state.fileGroups;
  if (groupId && !state.fileGroups.some((group) => group.id === groupId))
    return state.fileGroups;
  return state.fileGroups.map((group) => ({
    ...group,
    paths:
      group.id === groupId
        ? [...group.paths.filter((item) => item !== path), path]
        : group.paths.filter((item) => item !== path),
  }));
}

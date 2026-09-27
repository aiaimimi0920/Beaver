import { dialog } from "electron";

export async function chooseDirectory(): Promise<string | null> {
  const result = await dialog.showOpenDialog({
    properties: ["openDirectory", "createDirectory"],
  });
  return result.canceled ? null : (result.filePaths[0] ?? null);
}

export async function chooseTool(): Promise<string | null> {
  const result = await dialog.showOpenDialog({ properties: ["openFile"] });
  return result.canceled ? null : (result.filePaths[0] ?? null);
}

export async function chooseImportFiles(): Promise<string[]> {
  const result = await dialog.showOpenDialog({
    properties: ["openFile", "multiSelections"],
  });
  return result.canceled ? [] : result.filePaths;
}

export async function chooseImportDirectory(): Promise<string | null> {
  const result = await dialog.showOpenDialog({ properties: ["openDirectory"] });
  return result.canceled ? null : (result.filePaths[0] ?? null);
}

export const windowCommands = [
  "state",
  "minimize",
  "toggleMaximize",
  "close",
] as const;

export type WindowCommand = (typeof windowCommands)[number];
export interface WindowState {
  maximized: boolean;
}

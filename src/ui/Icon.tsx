const paths = {
  sword: "M4 20l5-5 M3 14l7 7 M8 13L18 3h3v3L11 16 M15 6l3 3",
  book: "M12 6v15 M12 6C8 2 3 4 3 4v15s5-2 9 2c4-4 9-2 9-2V4s-5-2-9 2",
  globe:
    "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M3 12h18 M12 3c-5 6-5 12 0 18 5-6 5-12 0-18",
  crown: "M3 6l4 4 5-7 5 7 4-4-2 13H5z M5 16h14",
  compass: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M16 8l-3 5-5 3 3-5z",
  puzzle:
    "M3 3h6v3a3 3 0 1 0 6 0V3h6v6h-3a3 3 0 0 0 0 6h3v6h-6v-3a3 3 0 0 0-6 0v3H3z",
  city: "M3 21V9h6v12 M9 21V3h7v18 M16 21V12h5v9 M12 6h1 M12 10h1 M12 14h1 M5 12h1 M18 15h1",
  wheel: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M3 10h18 M12 13v8 M8 10l4 5 4-5",
  leaf: "M4 20l13-13 M5 16C-1 5 12 3 21 3c0 10-2 20-13 16 M9 15v-5 M13 11h5",
  target: "M20 12a8 8 0 1 1-8-8 M16 12a4 4 0 1 1-4-4 M12 12L22 2 M17 2v5h5",
  heart: "M12 21S2 15 2 8c0-6 8-7 10-1 2-6 10-5 10 1 0 7-10 13-10 13z",
  chip: "M6 6h12v12H6z M9 9h6v6H9z M9 2v4 M15 2v4 M9 18v4 M15 18v4 M2 9h4 M2 15h4 M18 9h4 M18 15h4",
  moon: "M20 15A9 9 0 0 1 9 3a9 9 0 1 0 11 12z",
  palette:
    "M12 3a9 9 0 1 0 0 18h2a2 2 0 0 0 0-4 2 2 0 0 1 0-4h4c5 0 3-10-6-10z M7 8h.01 M12 6h.01 M17 8h.01 M5 13h.01",
  layers: "M12 3l10 5-10 5L2 8z M2 12l10 5 10-5 M2 16l10 5 10-5",
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  sidebar: "M4 4h16v16H4z M9 4v16",
  tasks: "M5 5h14v14H5z M8 9l2 2 5-5 M8 15h8",
  assets: "M3 4h18v16H3z M3 16l5-5 4 4 3-3 6 6 M15 8h.01",
  features: "M12 3l9 5v9l-9 5-9-5V8z M3 8l9 5 9-5 M12 13v9",
  project: "M5 3h14v18H5z M8 8h8 M8 12h8 M8 16h5",
  settings:
    "M10 3h4l1 3 3-1 2 3-2 3 2 3-2 3-3-1-1 3h-4l-1-3-3 1-2-3 2-3-2-3 2-3 3 1z M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
  refresh: "M20 10a8 8 0 1 0-2 8 M20 4v6h-6",
  minimize: "M5 12h14",
  maximize: "M5 5h14v14H5z",
  restore: "M8 8h12V3H8v5 M4 8h12v12H4z",
  close: "M6 6l12 12 M18 6L6 18",
  back: "M15 5l-7 7 7 7",
  arrowUpRight: "M6 18L18 6 M6 6h12v12",
  add: "M12 5v14 M5 12h14",
  grip: "M9 5h.01 M15 5h.01 M9 12h.01 M15 12h.01 M9 19h.01 M15 19h.01",
  lock: "M5 10h14v11H5z M8 10V7a4 4 0 0 1 8 0v3 M12 14v3",
  unlock: "M5 10h14v11H5z M8 10V7a4 4 0 0 1 8 0 M12 14v3",
  folder: "M3 6h7l2 3h9l-2 10H3z",
  import: "M12 3v12 M8 11l4 4 4-4 M4 15v6h16v-6",
  sparkles:
    "M13 3l2.5 6.5L22 12l-6.5 2.5L13 21l-2.5-6.5L4 12l6.5-2.5z M4 2v5 M1.5 4.5h5",
  search: "M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0 M15 15l6 6",
  tag: "M3 3h8l10 10-8 8L3 11z M7 7h.01",
  chevronDown: "M6 9l6 6 6-6",
  play: "M7 4l14 8-14 8z",
  code: "M8 6l-6 6 6 6 M16 6l6 6-6 6 M14 3l-4 18",
  review: "M12 3l8 3v6c0 5-8 9-8 9s-8-4-8-9V6z M8 12l3 3 5-6",
  speech: "M9 3h6v12H9z M5 10v3a7 7 0 0 0 14 0v-3 M12 20v2",
  music:
    "M9 18V5l11-2v13 M9 7l11-2 M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0 M20 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0",
  translation:
    "M3 5h12 M9 3v2 M6 5c0 7 7 11 7 11 M12 5c0 7-8 12-8 12 M14 21l4-10 4 10 M16 17h4",
  tools: "M4 4h16v12H4z M8 21h8 M12 16v5",
} as const;

export type IconName = keyof typeof paths;
export function Icon({ name }: { name: IconName }) {
  return (
    <svg
      className="icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.65"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}

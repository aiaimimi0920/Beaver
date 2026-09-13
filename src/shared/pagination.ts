export function paginateText(
  text: string,
  columns: number,
  rows: number,
): string[] {
  const width = Math.max(1, Math.floor(columns));
  const height = Math.max(1, Math.floor(rows));
  const pages: string[] = [];
  let page = "",
    line = 0,
    column = 0;
  for (const char of text) {
    if (column >= width && char !== "\n") {
      line++;
      column = 0;
    }
    if (line >= height) {
      pages.push(page);
      page = "";
      line = 0;
    }
    page += char;
    if (char === "\n") {
      line++;
      column = 0;
    } else column++;
  }
  if (page || !pages.length) pages.push(page);
  return pages;
}

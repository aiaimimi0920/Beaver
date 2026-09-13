import { Resvg } from "@resvg/resvg-js";
import { Data } from "resedit";

// Tauri decodes the first ICO entry for its runtime window/tray image.
export const iconSizes = [256, 128, 64, 48, 32, 24, 20, 16] as const;

export function renderBrandIcon(svg: string | Buffer, size: number) {
  return new Resvg(svg, {
    fitTo: { mode: "width", value: size },
    font: { loadSystemFonts: false },
  }).render();
}

export function buildBrandAssets(svg: string | Buffer): Map<string, Buffer> {
  const assets = new Map<string, Buffer>();
  const ico = new Data.IconFile();
  for (const size of iconSizes) {
    const rendered = renderBrandIcon(svg, size);
    if (rendered.width !== size || rendered.height !== size)
      throw new Error("The master brand icon must be square");
    const png = rendered.asPng();
    assets.set(`beaver-${size}.png`, png);
    ico.icons.push({ data: Data.RawIconItem.from(png, size, size, 32) });
  }
  assets.set("beaver.ico", Buffer.from(ico.generate()));
  return assets;
}

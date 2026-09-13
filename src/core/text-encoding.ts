export function decodeResourceText(bytes: Uint8Array): string {
  let encoding = "utf-8";
  if (
    (bytes[0] === 255 &&
      bytes[1] === 254 &&
      bytes[2] === 0 &&
      bytes[3] === 0) ||
    (bytes[0] === 0 && bytes[1] === 0 && bytes[2] === 254 && bytes[3] === 255)
  )
    throw new Error("暂不支持 UTF-32，请下载原始文件查看");
  if (bytes[0] === 255 && bytes[1] === 254) encoding = "utf-16le";
  if (bytes[0] === 254 && bytes[1] === 255) encoding = "utf-16be";
  try {
    const text = new TextDecoder(encoding, { fatal: true }).decode(bytes);
    if (text.includes("\0")) throw new Error("binary");
    return text;
  } catch {
    throw new Error("无法识别或损坏的文本编码，请下载原始文件查看");
  }
}

const TOKENS = {
  bg: "#06080d",
  surface: "#090c11",
  panel: "#0e1218",
  rail: "#070a0f",
  control: "#111720",
  controlHover: "#18222c",
  text: "#f7f8ef",
  muted: "#929a9f",
  line: "rgba(255, 255, 255, 0.12)",
  accent: "#d9ff38",
  secondary: "#22c55e",
  info: "#06b6d4",
  danger: "#f43f5e",
  focusSurface: "#f3f5f1",
  focusInk: "#20252b",
};

const TOKEN_META = [
  ["bg", "主背景", "--neuro-bg"],
  ["surface", "一级表面", "--neuro-surface"],
  ["panel", "面板", "--neuro-panel"],
  ["rail", "侧栏", "--neuro-rail"],
  ["control", "控件", "--neuro-control"],
  ["controlHover", "控件悬浮", "--neuro-control-hover"],
  ["text", "正文", "--neuro-text"],
  ["muted", "次级文本", "--neuro-text-muted"],
  ["line", "分隔线", "--neuro-line"],
  ["accent", "信号黄", "--neuro-signal-yellow"],
  ["secondary", "信号绿", "--neuro-signal-green"],
  ["info", "信息蓝", "--neuro-info-blue"],
  ["danger", "危险红", "--neuro-danger-red"],
  ["focusSurface", "焦点白", "--neuro-focus-surface"],
  ["focusInk", "焦点面正文", "--neuro-focus-ink"],
];

const toast = document.querySelector("#lab-toast");
const dialogLayer = document.querySelector("#dialog-layer");
let toastTimer = null;

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("is-visible");
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => toast.classList.remove("is-visible"), 2400);
}

async function copyText(value, successMessage) {
  try {
    await navigator.clipboard.writeText(value);
    showToast(successMessage);
  } catch {
    showToast(value);
  }
}

function colorToRgba(value, fallbackBackground = "#000000") {
  const probe = document.createElement("span");
  probe.style.color = value;
  probe.style.display = "none";
  document.body.append(probe);
  const computed = getComputedStyle(probe).color;
  probe.remove();
  const srgb = computed.match(/^color\(srgb\s+([\d.-]+)\s+([\d.-]+)\s+([\d.-]+)(?:\s*\/\s*([\d.-]+))?\)$/);
  if (srgb) {
    const [, r, g, b, alpha = "1"] = srgb;
    return [Number(r) * 255, Number(g) * 255, Number(b) * 255, Number(alpha)];
  }
  const numbers = computed.match(/[\d.]+/g)?.map(Number) || [0, 0, 0, 1];
  const [r, g, b, alpha = 1] = numbers;
  if (alpha >= 0.999) return [r, g, b, 1];
  const [br, bg, bb] = colorToRgba(fallbackBackground);
  return [
    Math.round(r * alpha + br * (1 - alpha)),
    Math.round(g * alpha + bg * (1 - alpha)),
    Math.round(b * alpha + bb * (1 - alpha)),
    1,
  ];
}

function relativeLuminance(value, background) {
  const rgb = colorToRgba(value, background).slice(0, 3).map((component) => component / 255);
  return rgb
    .map((component) => component <= 0.03928 ? component / 12.92 : ((component + 0.055) / 1.055) ** 2.4)
    .reduce((sum, component, index) => sum + component * [0.2126, 0.7152, 0.0722][index], 0);
}

function contrastRatio(foreground, background) {
  const foregroundLuminance = relativeLuminance(foreground, background);
  const backgroundLuminance = relativeLuminance(background, background);
  const lighter = Math.max(foregroundLuminance, backgroundLuminance);
  const darker = Math.min(foregroundLuminance, backgroundLuminance);
  return (lighter + 0.05) / (darker + 0.05);
}

function renderContrast() {
  const pairs = [
    ["正文 / 背景", TOKENS.text, TOKENS.bg, 4.5],
    ["正文 / 面板", TOKENS.text, TOKENS.panel, 4.5],
    ["次级文本 / 面板", TOKENS.muted, TOKENS.panel, 4.5],
    ["信号黄 / 背景", TOKENS.accent, TOKENS.bg, 3],
    ["信号绿 / 背景", `color-mix(in srgb, ${TOKENS.secondary} 76%, white)`, TOKENS.bg, 3],
    ["信息蓝 / 背景", `color-mix(in srgb, ${TOKENS.info} 76%, white)`, TOKENS.bg, 3],
    ["危险红 / 背景", `color-mix(in srgb, ${TOKENS.danger} 78%, white)`, TOKENS.bg, 3],
    ["焦点正文 / 白面", TOKENS.focusInk, TOKENS.focusSurface, 4.5],
  ];
  document.querySelector("#contrast-grid").replaceChildren(...pairs.map(([label, foreground, background, required]) => {
    const ratio = contrastRatio(foreground, background);
    const item = document.createElement("div");
    const state = ratio >= required ? "pass" : ratio >= 3 ? "warn" : "fail";
    item.className = `contrast-item is-${state}`;
    item.innerHTML = `<div><span>${label}</span><b>目标 ${required.toFixed(1)}+</b></div><strong>${ratio.toFixed(1)}</strong>`;
    return item;
  }));
}

function renderTokens() {
  document.querySelector("#token-editor").replaceChildren(...TOKEN_META.map(([key, label, variable]) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "token-field token-field--readonly";
    button.innerHTML = `<i style="--token-color:${TOKENS[key]}"></i><span><b>${label}</b><small>${variable}</small><code>${TOKENS[key]}</code></span>`;
    button.addEventListener("click", () => copyText(TOKENS[key], `${variable} 已复制。`));
    return button;
  }));
}

function cssExport() {
  return TOKEN_META.map(([key, , variable]) => `  ${variable}: ${TOKENS[key]};`).join("\n");
}

document.querySelector("#copy-css").addEventListener("click", () => {
  copyText(`:root {\n${cssExport()}\n}`, "Neuro 颜色令牌已复制。");
});

document.querySelector("#export-json").addEventListener("click", () => {
  const link = document.createElement("a");
  const payload = JSON.stringify({ schemaVersion: 1, name: "Neuro 设计方案", tokens: TOKENS }, null, 2);
  link.href = URL.createObjectURL(new Blob([payload], { type: "application/json" }));
  link.download = "neuro-design-tokens.json";
  link.click();
  URL.revokeObjectURL(link.href);
  showToast("Neuro 令牌 JSON 已导出。");
});

document.querySelector("#toggle-grid").addEventListener("click", (event) => {
  const pressed = event.currentTarget.getAttribute("aria-pressed") !== "true";
  event.currentTarget.setAttribute("aria-pressed", String(pressed));
  document.body.classList.toggle("no-grid", !pressed);
});

document.querySelectorAll("[data-target]").forEach((button) => button.addEventListener("click", () => {
  document.querySelectorAll("[data-target]").forEach((item) => item.classList.toggle("is-active", item === button));
  document.querySelector(`#${button.dataset.target}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
}));

document.querySelectorAll("[data-copy-color]").forEach((button) => button.addEventListener("click", () => {
  copyText(button.dataset.copyColor, `${button.dataset.copyColor} 已复制。`);
}));

document.querySelectorAll(".notice").forEach((notice) => notice.addEventListener("click", () => notice.remove()));
document.querySelector("#open-dialog").addEventListener("click", () => {
  dialogLayer.hidden = false;
  document.querySelector("#close-dialog").focus();
});

function closeDialog() {
  dialogLayer.hidden = true;
  document.querySelector("#open-dialog").focus();
}

document.querySelector("#close-dialog").addEventListener("click", closeDialog);
document.querySelector("#cancel-dialog").addEventListener("click", closeDialog);
dialogLayer.addEventListener("mousedown", (event) => { if (event.target === dialogLayer) closeDialog(); });
window.addEventListener("keydown", (event) => { if (event.key === "Escape" && !dialogLayer.hidden) closeDialog(); });

renderContrast();
renderTokens();

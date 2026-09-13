export {};

let choice = "";
let revision = 0;
const selection = document.querySelector<HTMLElement>("#selection")!;
const notes = document.querySelector<HTMLTextAreaElement>("#notes")!;
const fallback = document.querySelector<HTMLTextAreaElement>("#copy-fallback")!;

function pick(value: string): void {
  revision += 1;
  choice = value;
  for (const card of document.querySelectorAll<HTMLElement>(".candidate"))
    card.classList.toggle("selected", card.dataset.choice === choice);
  for (const button of document.querySelectorAll<HTMLButtonElement>(
    "[data-pick]",
  ))
    button.setAttribute("aria-pressed", String(button.dataset.pick === choice));
  fallback.hidden = true;
  selection.textContent = choice
    ? `已选 ${choice} · 未发送`
    : "六组全部重想 · 未发送";
}
for (const button of document.querySelectorAll<HTMLButtonElement>(
  "[data-pick]",
))
  button.addEventListener("click", () => pick(button.dataset.pick ?? ""));
document.querySelector("#clear")!.addEventListener("click", () => pick(""));
notes.addEventListener("input", () => {
  revision += 1;
  fallback.hidden = true;
  selection.textContent = choice
    ? `已选 ${choice} · 未发送`
    : "意见已修改 · 未发送";
});
document
  .querySelector<HTMLButtonElement>("#theme")!
  .addEventListener("click", (event) => {
    const light = document.body.classList.toggle("light");
    const button = event.currentTarget as HTMLButtonElement;
    button.textContent = light ? "深色预览" : "浅色预览";
    button.setAttribute("aria-pressed", String(light));
  });
document.querySelector("#copy")!.addEventListener("click", () => {
  const text = `Beaver 图标第四轮：${choice ? `选择 ${choice}` : "六组均需调整"}。\n${notes.value.trim() || (choice ? "请继续细化这个方向。" : "请重新探索其他方向。")}`;
  const copiedRevision = ++revision;
  const showFallback = (): void => {
    if (copiedRevision !== revision) return;
    fallback.value = text;
    fallback.hidden = false;
    fallback.focus();
    fallback.select();
    selection.textContent = "请复制下方意见，发回对话";
  };
  if (!navigator.clipboard) {
    showFallback();
    return;
  }
  void navigator.clipboard
    .writeText(text)
    .then(() => {
      if (copiedRevision !== revision) return;
      fallback.hidden = true;
      selection.textContent = "已复制，请粘贴回对话";
    })
    .catch(showFallback);
});

export {};

let choice = "";
const status = document.querySelector<HTMLElement>("#selection")!;
const notes = document.querySelector<HTMLTextAreaElement>("#notes")!;
for (const button of document.querySelectorAll<HTMLButtonElement>(
  "[data-pick]",
)) {
  button.addEventListener("click", () => {
    choice = button.dataset.pick ?? "";
    for (const card of document.querySelectorAll<HTMLElement>(".candidate"))
      card.classList.toggle("selected", card.dataset.choice === choice);
    for (const item of document.querySelectorAll<HTMLButtonElement>(
      "[data-pick]",
    ))
      item.setAttribute("aria-pressed", String(item === button));
    status.textContent = `已选 ${choice} · 未发送`;
  });
}
document
  .querySelector<HTMLButtonElement>("#theme")!
  .addEventListener("click", (event) => {
    const light = document.body.classList.toggle("light");
    const button = event.currentTarget as HTMLButtonElement;
    button.textContent = light ? "深色预览" : "浅色预览";
    button.setAttribute("aria-pressed", String(light));
  });
document.querySelector("#copy")!.addEventListener("click", () => {
  const text = `Beaver 图标第一轮：${choice ? `选择 ${choice}` : "三组均需调整"}。\n${notes.value.trim() || "请继续细化这个方向。"}`;
  const fallback =
    document.querySelector<HTMLTextAreaElement>("#copy-fallback")!;
  fallback.value = text;
  if (!navigator.clipboard) {
    fallback.hidden = false;
    fallback.select();
    status.textContent = "请复制下方意见，发回对话";
    return;
  }
  void navigator.clipboard
    .writeText(text)
    .then(() => {
      fallback.hidden = true;
      status.textContent = "已复制，请粘贴回对话";
    })
    .catch(() => {
      fallback.hidden = false;
      fallback.select();
      status.textContent = "请复制下方意见，发回对话";
    });
});

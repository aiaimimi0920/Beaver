"use strict";
(() => {
  let choice = "";
  const status = document.querySelector("#selection");
  const notes = document.querySelector("#notes");
  for (const button of document.querySelectorAll("[data-pick]")) {
    button.addEventListener("click", () => {
      choice = button.dataset.pick ?? "";
      for (const card of document.querySelectorAll(".candidate"))
        card.classList.toggle("selected", card.dataset.choice === choice);
      for (const item of document.querySelectorAll("[data-pick]"))
        item.setAttribute("aria-pressed", String(item === button));
      status.textContent = `\u5DF2\u9009 ${choice} \xB7 \u672A\u53D1\u9001`;
    });
  }
  document.querySelector("#theme").addEventListener("click", (event) => {
    const light = document.body.classList.toggle("light");
    const button = event.currentTarget;
    button.textContent = light
      ? "\u6DF1\u8272\u9884\u89C8"
      : "\u6D45\u8272\u9884\u89C8";
    button.setAttribute("aria-pressed", String(light));
  });
  document.querySelector("#copy").addEventListener("click", () => {
    const text = `Beaver \u56FE\u6807\u7B2C\u4E00\u8F6E\uFF1A${choice ? `\u9009\u62E9 ${choice}` : "\u4E09\u7EC4\u5747\u9700\u8C03\u6574"}\u3002
${notes.value.trim() || "\u8BF7\u7EE7\u7EED\u7EC6\u5316\u8FD9\u4E2A\u65B9\u5411\u3002"}`;
    const fallback = document.querySelector("#copy-fallback");
    fallback.value = text;
    if (!navigator.clipboard) {
      fallback.hidden = false;
      fallback.select();
      status.textContent =
        "\u8BF7\u590D\u5236\u4E0B\u65B9\u610F\u89C1\uFF0C\u53D1\u56DE\u5BF9\u8BDD";
      return;
    }
    void navigator.clipboard
      .writeText(text)
      .then(() => {
        fallback.hidden = true;
        status.textContent =
          "\u5DF2\u590D\u5236\uFF0C\u8BF7\u7C98\u8D34\u56DE\u5BF9\u8BDD";
      })
      .catch(() => {
        fallback.hidden = false;
        fallback.select();
        status.textContent =
          "\u8BF7\u590D\u5236\u4E0B\u65B9\u610F\u89C1\uFF0C\u53D1\u56DE\u5BF9\u8BDD";
      });
  });
})();

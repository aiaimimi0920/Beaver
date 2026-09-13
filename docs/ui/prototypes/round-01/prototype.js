"use strict";
(() => {
  // docs/ui/prototypes/round-01/prototype.ts
  function element(selector) {
    const result = document.querySelector(selector);
    if (!result) throw new Error(`Missing prototype element: ${selector}`);
    return result;
  }
  function escape(value) {
    const entities = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;"
    };
    return value.replace(/[&<>"']/g, (c) => entities[c] ?? c);
  }
  var app = element("#app");
  var view = element("#view");
  var dialog = element("#dialog");
  var returnFocus = null;
  var modalAction = null;
  var noticeTimer = 0;
  var page = "create";
  var createTab = "results";
  var gameTab = "overview";
  var settingsTab = "ai";
  var taskFilter = "all";
  var selectedTask = 1;
  var selectedDoc = "character";
  var editingDoc = false;
  var assetFilter = "全部";
  var selectedAssets = /* @__PURE__ */ new Set(["portrait-1"]);
  var setupStep = 1;
  var environmentStep = 0;
  var aiMode = "platform";
  var featureTab = "installed";
  var game = {
    name: "夜航调饮室",
    genre: "视觉小说",
    secondary: "模拟",
    theme: "赛博朋克",
    rating: "全年龄",
    size: "独立",
    style: "像素 2D",
    online: false
  };
  var gameDraft = null;
  var tasks = [
    {
      id: 1,
      title: "统一三位客人的立绘风格",
      status: "done",
      accepted: false,
      refs: ["人物设定.md", "立绘参考 1–3"],
      messages: [],
      draft: ""
    },
    {
      id: 2,
      title: "完成第一晚的调饮与对话",
      status: "working",
      accepted: false,
      refs: [],
      messages: [],
      draft: ""
    },
    {
      id: 3,
      title: "制作雨夜酒吧的环境音乐",
      status: "working",
      accepted: false,
      refs: ["雨声参考.wav"],
      messages: [],
      draft: ""
    },
    {
      id: 4,
      title: "补充第二位客人的故事",
      status: "paused",
      accepted: false,
      refs: [],
      messages: [],
      draft: ""
    }
  ];
  var assets = [
    {
      id: "portrait-1",
      name: "阿澄 · 立绘",
      type: "图片",
      icon: "image",
      path: "assets/characters/cheng.png"
    },
    {
      id: "portrait-2",
      name: "远舟 · 立绘",
      type: "图片",
      icon: "image",
      path: "assets/characters/yuan.png"
    },
    {
      id: "portrait-3",
      name: "老板 · 立绘",
      type: "图片",
      icon: "image",
      path: "assets/characters/owner.png"
    },
    {
      id: "bar",
      name: "雨夜酒吧 · 背景",
      type: "图片",
      icon: "image",
      path: "assets/backgrounds/bar.png"
    },
    {
      id: "music",
      name: "雨夜 · 环境音乐",
      type: "音频",
      icon: "audio",
      path: "assets/audio/rain-night.ogg"
    },
    {
      id: "sound",
      name: "调饮 · 杯具音效",
      type: "音频",
      icon: "audio",
      path: "assets/audio/glass.wav"
    },
    {
      id: "model",
      name: "吧台 · 模型",
      type: "模型",
      icon: "box",
      path: "assets/models/counter.glb"
    },
    {
      id: "scene",
      name: "酒吧 · 场景",
      type: "场景",
      icon: "game",
      path: "scenes/bar.tscn"
    },
    {
      id: "video",
      name: "开场 · 分镜预览",
      type: "视频",
      icon: "play",
      path: "assets/video/opening.mp4"
    },
    {
      id: "translation",
      name: "第一晚 · 中英对照",
      type: "翻译",
      icon: "book",
      path: "localization/night-01.csv"
    }
  ];
  var docs = {
    direction: {
      title: "游戏方向与创作约定",
      path: "docs/direction/创作约定.md",
      text: "# 游戏方向与创作约定\n\n通过调饮与对话，了解城市中普通人的生活。\n\n## 不变的方向\n- 内容原创，不复刻参考作品的人物与剧情。\n- 玩家判断的重点是对话选择，不是操作速度。\n- 基础评级和视觉风格以游戏总览中的设定为准。"
    },
    world: {
      title: "城市与酒吧",
      path: "docs/world/城市与酒吧.md",
      text: "# 城市与酒吧\n\n故事发生在一座沿海的近未来城市。夜航是一间深夜营业的小酒吧。\n\n## 日常生活\n连续的雨季影响了城市交通，店里的客人大多来自附近社区。\n\n## 叙事边界\n科技是生活的背景，不用技术名词替代人物的真实动机。"
    },
    character: {
      title: "阿澄 · 人物介绍",
      path: "docs/characters/阿澄.md",
      text: "# 阿澄 · 人物介绍\n\n## 身份与动机\n社区配送员，夜班结束后偶尔来店里休息。正在攒钱离开城市，却舍不得熟悉的人。\n\n## 说话方式\n平时直接、偶尔开玩笑；谈到战争时会停顿，不会突然长篇解释。\n\n## 人物关系\n与老板是多年的朋友。老板知道她在攒钱，但从不替她做决定。\n\n## 视觉约定\n旧款防雨外套，配色低饱和。表情变化优先于大量身体动作。\n\n## 相关资料\n城市与酒吧.md · 第一晚剧情.md · 角色视觉规范.md"
    },
    story: {
      title: "第一晚剧情",
      path: "docs/story/第一晚剧情.md",
      text: "# 第一晚剧情\n\n开店 → 第一位客人 → 学习调饮 → 第二位客人 → 打烊。\n\n## 关键选择\n玩家可以追问，也可以安静地递上一杯饮料。沉默同样是一种选择。\n\n## 完成标准\n每个选择有可理解的反馈，第一晚能够从头玩到结尾。"
    },
    systems: {
      title: "调饮规则",
      path: "docs/systems/调饮规则.md",
      text: "# 调饮规则\n\n玩家选择原料、调制方式和饮品，再交给客人。\n\n## 输入与反馈\n同一配方必须产生一致结果。饮品影响后续对话，但错误饮品不应直接终止故事。\n\n## 边界\n首版不需要真实液体物理，也不需要多人同步。"
    },
    levels: {
      title: "酒吧空间",
      path: "docs/levels/酒吧空间.md",
      text: "# 酒吧空间\n\n可交互区域：客人座位、调饮台、收音机、门口。\n\n## 镜头\n以固定机位为主。重要互动无需玩家寻找隐藏的点击区域。"
    },
    art: {
      title: "角色视觉规范",
      path: "docs/art/角色视觉规范.md",
      text: "# 角色视觉规范\n\n## 比例\n角色使用统一的头身比、光照方向与描边粗细。\n\n## 颜色\n控制高饱和强调色的数量，保持背景与立绘的可读性。\n\n## 声音\n人声、音乐与环境声有明确层级，不能互相遮盖。"
    },
    tech: {
      title: "存档与验收约定",
      path: "docs/technical/存档与验收.md",
      text: "# 存档与验收约定\n\n## 存档\n在对话段落之间保存，不丢失玩家已经做出的选择。\n\n## 验收\n首次启动、继续游戏、重新开始、音量调整、窗口尺寸变化都需要验证。"
    },
    release: {
      title: "本地化与交付",
      path: "docs/release/本地化与交付.md",
      text: "# 本地化与交付\n\n## 本地化\n角色称谓和配方名称保持一致，文本长度变化不能破坏布局。\n\n## 交付\n首个交付目标是可双击运行的 Windows 玩家程序。渠道发布为后续目标。"
    }
  };
  var groups = [
    ["方向与约定", ["direction"]],
    ["世界与故事", ["world", "story"]],
    ["人物与关系", ["character"]],
    ["玩法与系统", ["systems"]],
    ["关卡与场景", ["levels"]],
    ["美术与声音", ["art"]],
    ["技术与测试", ["tech"]],
    ["发行与本地化", ["release"]]
  ];
  var icons = {
    sidebar: '<rect x="3" y="4" width="18" height="16" rx="1"/><path d="M9 4v16"/>',
    chat: '<path d="M4 4h16v12H9l-5 4z"/><path d="M8 8h8M8 12h5"/>',
    book: '<path d="M4 4h6l2 2 2-2h6v16h-6l-2-2-2 2H4zM12 6v12"/>',
    image: '<rect x="3" y="4" width="18" height="16" rx="1"/><path d="m3 16 5-5 4 4 4-3 5 5"/><circle cx="16" cy="8" r="1"/>',
    game: '<path d="M7 6h10l4 12h-4l-3-3h-4l-3 3H3zM6 10h5M8.5 7.5v5M16 10h.1M18 12h.1"/>',
    settings: '<circle cx="12" cy="12" r="4"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M5 5l3 3M16 16l3 3M5 19l3-3M16 8l3-3"/>',
    box: '<path d="m12 2 9 5v10l-9 5-9-5V7zM3 7l9 5 9-5M12 12v10"/>',
    plus: '<path d="M12 4v16M4 12h16"/>',
    folder: '<path d="M3 5h6l3 3h9l-2 12H3z"/>',
    audio: '<path d="M5 9v6M9 5v14M13 8v8M17 3v18M21 9v6"/>',
    play: '<path d="m8 4 12 8-12 8z"/>'
  };
  function icon(name) {
    return `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] ?? icons.box}</svg>`;
  }
  function hydrateIcons() {
    document.querySelectorAll("[data-icon]").forEach((node) => {
      node.innerHTML = icon(node.dataset.icon ?? "box");
    });
  }
  function button(label, action, className = "") {
    return `<button class="${className}" data-action="${action}">${label}</button>`;
  }
  function placeholder(label, kind = "image", detail = "预览区域") {
    return `<div class="placeholder">${icon(kind)}<span>${label}</span><small>${detail}</small></div>`;
  }
  function tabs(items, current, key) {
    return `<nav class="tabs" aria-label="页面分区">${items.map(([id, title]) => `<button data-${key}="${id}" class="${current === id ? "active" : ""}" aria-current="${current === id ? "page" : "false"}">${title}</button>`).join("")}</nav>`;
  }
  function status(task2) {
    return task2.status === "working" ? "执行中" : task2.status === "paused" ? "已中止 · 可继续" : task2.accepted ? "已认可" : "待验收";
  }
  function task() {
    return tasks.find((item) => item.id === selectedTask) ?? tasks[0];
  }
  function textToDocument(text) {
    return text.split("\n").map(
      (line) => line.startsWith("# ") ? "" : line.startsWith("## ") ? `<h2>${escape(line.slice(3))}</h2>` : line ? `<p>${escape(line)}</p>` : "<br>"
    ).join("");
  }
  function notice(message) {
    const node = element("#notice");
    node.innerHTML = `${escape(message)}<button data-action="dismiss-notice" aria-label="关闭通知">×</button>`;
    node.hidden = false;
    window.clearTimeout(noticeTimer);
    noticeTimer = window.setTimeout(() => {
      node.hidden = true;
    }, 5e3);
  }
  function closeModal() {
    dialog.close();
  }
  dialog.addEventListener("close", () => {
    if (dialog.open) return;
    document.body.style.overflow = "";
    modalAction = null;
    if (returnFocus?.isConnected) returnFocus.focus();
  });
  function modal(title, body, confirmLabel, confirm) {
    if (dialog.open) dialog.close();
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    dialog.innerHTML = `<header><h2 id="dialog-title">${title}</h2><button class="quiet" data-action="close-modal" aria-label="关闭弹窗">×</button></header><div class="dialog-body">${body}</div><footer>${button("取消", "close-modal")}${confirmLabel ? button(confirmLabel, "confirm-modal", "primary") : ""}</footer>`;
    modalAction = confirm ?? null;
    dialog.showModal();
    document.body.style.overflow = "hidden";
    hydrateIcons();
    (dialog.querySelector(
      "textarea, input:not([type=checkbox]):not([type=radio]), select"
    ) ?? dialog.querySelector("button"))?.focus();
  }
  function render() {
    app.classList.toggle("onboarding", page === "onboarding");
    element(".project-picker").disabled = page === "onboarding";
    element("#project-name").textContent = page === "onboarding" ? "开始创作" : game.name;
    document.querySelectorAll("[data-page]").forEach((node) => {
      if (node.dataset.page === page) node.setAttribute("aria-current", "page");
      else node.removeAttribute("aria-current");
    });
    element(".count").textContent = String(
      tasks.filter((item) => item.status === "working").length
    );
    view.innerHTML = page === "create" ? renderCreate() : page === "docs" ? renderDocs() : page === "assets" ? renderAssets() : page === "game" ? renderGame() : page === "settings" ? renderSettings() : renderSetup();
    hydrateIcons();
  }
  function renderCreate() {
    const current = task();
    const filtered = tasks.filter(
      (item) => taskFilter === "all" || (taskFilter === "working" ? item.status === "working" : item.status === "done" && !item.accepted)
    );
    const content = createTab === "results" ? renderResults(current) : createTab === "conversation" ? `
    <div class="message user"><small>你 · 原始要求</small><p>${escape(current.title)}。保留已经确定的设定，以最终游戏里的表现为准。</p></div>
    ${current.refs.length ? `<div class="message"><small>你提供的参考</small><p>${current.refs.map(escape).join(" · ")}</p></div>` : ""}
    <div class="message"><small>Codex · 简要汇报（示例）</small><p>${current.status === "done" ? "结果已经整理在当前任务中，可以查看素材或试玩。" : "围绕这个目标继续执行。需要你处理的问题会在这里显示。"}</p></div>
    ${current.messages.map((message) => `<div class="message user"><small>你 · 补充要求</small><p>${escape(message)}</p></div>`).join("")}
    <details class="fold"><summary>执行记录</summary><p class="muted">工具调用、代码修改、检查记录按需展开，不占用默认创作界面。</p></details>
  ` : `<p class="muted">当前任务的文件变化与回退入口。</p><div class="list-row"><div><code>assets/characters/cheng.png</code><small>修改 · 立绘</small></div><span>前后对比</span></div><div class="list-row"><div><code>docs/art/角色视觉规范.md</code><small>修改 · 资料</small></div><span>差异阅读</span></div><div class="list-row"><div><code>scenes/characters/cheng.tscn</code><small>修改 · 游戏引用</small></div><span>变更内容</span></div><div class="result-actions">${button("回退此任务", "rollback", "danger")}</div><p class="muted section-gap">这里只展示文件与回退边界，不要求用户阅读代码才能验收。</p>`;
    return `<div class="workspace">
    <aside class="directory"><div class="section-head"><h2>创作</h2><div class="spacer"></div>${button("＋", "new-task")}</div><div class="filter-bar"><select id="task-filter" aria-label="任务筛选"><option value="all" ${taskFilter === "all" ? "selected" : ""}>全部任务</option><option value="working" ${taskFilter === "working" ? "selected" : ""}>执行中</option><option value="review" ${taskFilter === "review" ? "selected" : ""}>待验收</option></select></div><div class="directory-body">${filtered.map((item) => `<button class="task-item ${item.id === selectedTask ? "active" : ""}" data-task="${item.id}"><strong>${escape(item.title)}</strong><small class="${item.status === "working" ? "positive" : item.status === "done" && !item.accepted ? "attention" : ""}">${status(item)}</small></button>`).join("") || '<p class="muted">没有对应的任务</p>'}</div><div class="directory-footer"><small>任务按你的目标组织<br>不同目标可以同时推进</small></div></aside>
    <section class="detail"><div class="detail-title"><h1>${escape(current.title)}</h1><div class="spacer"></div><span class="status ${current.status === "working" ? "positive" : "attention"}">${status(current)}</span>${current.status === "working" ? button("中止", "stop-task") : current.status === "paused" ? button("继续", "resume-task") : ""}</div>
      ${tabs(
      [
        ["results", "结果"],
        ["conversation", "对话"],
        ["changes", "变更与回退"]
      ],
      createTab,
      "create-tab"
    )}
      <div class="scroll">${content}</div>
      <div class="composer"><textarea id="task-input" aria-label="任务反馈" placeholder="指出哪里不对，或补充你想要的效果…">${escape(current.draft)}</textarea><div class="row">${button("添加参考", "references")}${button("执行限制", "limits", "quiet")}<div class="spacer"></div>${button(current.status === "paused" ? "保存补充（演示）" : "提交修改（演示）", "send-feedback", "primary")}</div></div>
    </section></div>`;
  }
  function renderResults(current) {
    if (current.status !== "done")
      return `<div class="result-summary"><p>${current.status === "working" ? "正在执行这个创作目标。" : "任务已中止，已有结果与对话保留。"}</p><small>这里显示最近汇报、阶段结果和需要你处理的问题。</small></div>${placeholder("阶段结果展示区", "game", "素材、文档、可玩版本；有结果时出现")}${current.status === "paused" ? `<div class="result-actions">${button("稍后继续", "later")}</div>` : ""}`;
    return `<div class="result-summary"><p>三位客人的描边、比例与配色已统一。</p><small>已接入游戏 · 等你查看（示例）</small></div><div class="result-grid"><div>${placeholder("三张立绘", "image", "缩略图 / 前后对比")}${button("查看素材", "review-assets", "quiet")}</div><div>${placeholder("游戏内效果", "game", "截图 / 试玩入口")}${button("试玩检查", "play", "quiet")}</div></div><div class="section-gap"><div class="result-link"><span>角色视觉规范.md</span>${button("阅读资料", "open-art-doc", "quiet")}</div><div class="result-link"><span>5 个文件变更 · 可回退</span>${button("查看变化", "task-changes", "quiet")}</div></div><div class="result-actions">${button(current.accepted ? "已认可此结果" : "认可结果", "accept-result")}${button("继续调整", "focus-feedback", "quiet")}</div>`;
  }
  function renderDocs() {
    const current = docs[selectedDoc];
    return `<div class="workspace"><aside class="directory"><div class="section-head"><h2>资料</h2><div class="spacer"></div>${button("＋", "new-doc")}</div><div class="directory-body">${groups.map(([title, ids]) => `<details class="doc-group" ${ids.some((id) => id === selectedDoc) ? "open" : ""}><summary>${title}</summary>${ids.map((id) => `<button class="${id === selectedDoc ? "active" : ""}" data-doc="${id}">${docs[id].title}</button>`).join("")}</details>`).join("")}</div><div class="directory-footer"><small>按内容归档，不按工种分配</small></div></aside><section class="detail"><div class="section-head"><h2>${current.title}</h2><div class="spacer"></div>${button(editingDoc ? "取消" : "编辑原文", editingDoc ? "cancel-doc" : "edit-doc")}${button(editingDoc ? "保存草稿（演示）" : "交给 AI 修改", editingDoc ? "save-doc" : "revise-doc", "primary")}</div><div class="scroll"><article class="document"><small class="path">示例归档路径 · <code>${current.path}</code></small>${editingDoc ? `<textarea class="doc-editor" id="doc-editor" aria-label="文档原文">${escape(current.text)}</textarea>` : textToDocument(current.text)}</article></div><footer class="page-footer"><small>文档原文可人工维护；不在这里规定 Codex 必须读取哪些上下文。</small></footer></section></div>`;
  }
  function renderAssets() {
    const selected = assets.filter((asset) => selectedAssets.has(asset.id));
    const filtered = assets.filter(
      (asset) => assetFilter === "全部" || asset.type === assetFilter
    );
    return `<section class="page"><div class="page-head"><h1>素材</h1><div class="spacer"></div>${button("导入参考", "import-reference")}</div><nav class="filter-bar" aria-label="素材类型">${["全部", "图片", "音频", "模型", "场景", "视频", "翻译"].map((type) => `<button class="${type === assetFilter ? "active" : ""}" data-asset-filter="${type}">${type}</button>`).join("")}</nav><div class="assets-layout"><div class="scroll asset-grid">${filtered.map((asset) => `<article class="asset ${selectedAssets.has(asset.id) ? "selected" : ""}"><button data-asset="${asset.id}" aria-label="预览${asset.name}">${placeholder(asset.type, asset.icon, "预览占位")}<strong>${asset.name}</strong></button><label><input type="checkbox" data-asset-check="${asset.id}" aria-label="选择${asset.name}" ${selectedAssets.has(asset.id) ? "checked" : ""}></label></article>`).join("")}</div><aside class="inspector"><h2>${selected.length > 1 ? `已选 ${selected.length} 项` : selected[0]?.name ?? "未选择素材"}</h2>${placeholder(selected.length > 1 ? "选中素材对比" : selected[0]?.type === "音频" ? "试听与时间点" : selected[0]?.type === "翻译" ? "原文 / 译文" : "选中素材预览", selected[0]?.icon ?? "image", "图片框选 / 音频反馈 / 模型观察")}${selected.length ? `<ul class="selected-names">${selected.map((asset) => `<li>${asset.name}</li>`).join("")}</ul><small><code>${selected.length === 1 ? selected[0].path : "保留选中对象作为参考"}</code></small>` : ""}<div class="section-gap stack">${button("提出修改", "asset-feedback", "primary")}${button("按这些参考创作", "asset-reference")}${button("统一风格", "asset-style")}</div></aside></div></section>`;
  }
  function renderGame() {
    return `<section class="page">${tabs(
      [
        ["overview", "总览"],
        ["priorities", "创作重点"],
        ["features", "功能"],
        ["builds", "试玩与导出"]
      ],
      gameTab,
      "game-tab"
    )}<div class="scroll">${gameTab === "overview" ? renderOverview() : gameTab === "priorities" ? renderPriorities() : gameTab === "features" ? renderFeatures() : renderBuilds()}</div></section>`;
  }
  function renderOverview() {
    const current = gameDraft ?? game;
    const field = (name, label, choices) => `<div class="field"><small>${label}</small>${gameDraft ? choices ? `<select data-game-field="${name}" aria-label="${label}">${choices.map((value) => `<option ${current[name] === value ? "selected" : ""}>${value}</option>`).join("")}</select>` : `<input data-game-field="${name}" aria-label="${label}" value="${escape(String(current[name]))}">` : `<strong>${escape(String(current[name]))}</strong>`}</div>`;
    return `<div class="overview-grid"><div class="overview-left">${gameDraft ? `<input data-game-field="name" aria-label="游戏名称" value="${escape(current.name)}">` : `<h1>${escape(current.name)}</h1>`}<div class="section-gap">${placeholder("当前游戏画面", "game", "最近可玩版本 / 截图")}</div><div class="row">${button("试玩", "play", "primary")}${button("提出修改", "game-feedback")}</div><p>一间深夜营业的酒吧。通过调饮和对话，了解客人的生活。</p><div class="result-link"><span>创作约定</span>${button("阅读", "open-direction", "quiet")}</div><div class="result-link"><span>第一晚 · 待试玩验收</span>${button("查看任务", "open-story-task", "quiet")}</div><small class="section-gap">模板：原创对话调饮起步项目 · 仅创建时选择</small></div><aside class="profile"><header><strong>${gameDraft ? "编辑设定" : "设定已锁定"}</strong><div class="spacer"></div><label><input type="checkbox" role="switch" id="edit-game" aria-label="编辑游戏设定" ${gameDraft ? "checked" : ""}>编辑</label></header>${gameDraft ? '<p class="attention">修改基础方向可能破坏已有内容。</p>' : ""}<div class="pair">${field("genre", "主类别", ["视觉小说", "角色扮演", "冒险"])}${field("secondary", "副类别", ["模拟", "无", "解谜"])}</div>${field("theme", "主题")}<div class="pair">${field("rating", "目标评级", ["全年龄", "青少年", "成年人"])}${field("size", "游戏大小", ["独立", "标准", "大型", "AAA"])}</div>${field("style", "表现风格", ["像素 2D", "手绘 2D", "低多边形 3D"])}<div class="field"><small>在线多人游戏</small>${gameDraft ? `<label><input id="game-online" type="checkbox" ${current.online ? "checked" : ""}>启用在线多人</label>${current.online ? `<div class="section-gap">${button("服务器容量与架构", "network")}</div>` : ""}` : `<strong>${current.online ? "开启" : "关闭"}</strong>`}</div>${gameDraft ? `<div class="row section-gap">${button("取消", "discard-game")}${button("保存设定（演示）", "save-game", "primary")}</div>` : ""}</aside></div>`;
  }
  function renderPriorities() {
    const stages = [
      ["设计", ["故事性", "角色", "游戏性", "图像", "用户界面"]],
      ["开发", ["物理", "人工智能", "声音", "模组", "网络"]],
      ["后期", ["动画", "优化", "插画", "教程", "过场动画"]]
    ];
    return `<div class="row"><h1>创作重点</h1><div class="spacer"></div>${button("保存重点（演示）", "save-priorities", "primary")}</div><p class="muted section-gap">表达希望投入的重点，不是给 Codex 安排执行顺序。</p><div class="stage-grid section-gap">${stages.map(([stage, labels]) => `<section><h2>${stage}</h2>${labels.map((label, index) => `<label class="range-row"><span>${label}</span><input type="range" min="0" max="100" value="${index < 2 ? 75 : 35}" aria-label="${stage}阶段${label}重点"></label>`).join("")}</section>`).join("")}</div><details class="fold"><summary>宣传与试玩计划 · 后续目标</summary><div class="row wrap section-gap">${button("媒体联系", "future")}${button("发布试玩版", "future")}${button("广告宣传", "future")}</div></details>`;
  }
  function renderFeatures() {
    const rows = featureTab === "installed" ? [
      [
        "对话与分支",
        "项目已定制 · 基线 v1.0 → 新版 v1.1",
        "查看更新差异",
        "feature-update"
      ],
      [
        "存档与读档",
        "已添加 · 保留项目自己的修改",
        "查看详情",
        "feature-detail"
      ]
    ] : [
      ["任务与目标", "玩法功能 · 规划项", "加入规划", "feature-plan"],
      ["天气与昼夜", "场景功能 · 规划项", "加入规划", "feature-plan"],
      ["多人联机", "网络功能 · 规划项", "加入规划", "feature-plan"],
      [
        "对话分支源码包",
        "有源码的功能包 · 可交给 AI 接入",
        "交给 AI 添加",
        "feature-add"
      ]
    ];
    return `<div class="row"><h1>功能</h1><div class="spacer"></div><span class="muted">功能包可在创建后添加</span></div><div class="filter-bar section-gap"><button data-feature-tab="installed" class="${featureTab === "installed" ? "active" : ""}">项目已有</button><button data-feature-tab="catalog" class="${featureTab === "catalog" ? "active" : ""}">功能目录</button></div>${rows.map(([name, description, label, action]) => `<div class="list-row"><div><strong>${name}</strong><small>${description}</small></div>${button(label, action)}</div>`).join("")}<details class="fold"><summary>规划项与源码包的区别</summary><p class="muted section-gap">规划项表示希望游戏具备的功能，不代表已经提供实现。源码包才能进入接入和版本更新流程。此处全部为布局示例。</p></details>`;
  }
  function renderBuilds() {
    return `<div class="row"><h1>试玩与导出</h1><div class="spacer"></div>${button("生成可运行程序", "build", "primary")}</div><div class="overview-grid section-gap"><div>${placeholder("游戏运行画面", "game", "试玩在游戏窗口进行；此处保留截图与反馈入口")}<div class="result-actions">${button("启动试玩（演示）", "launch-game")}${button("截图反馈", "game-feedback")}</div></div><div class="stack"><label class="form-field"><span>目标系统</span><select aria-label="导出目标系统"><option>Windows · 首版</option><option disabled>macOS · 待支持</option><option disabled>Linux · 待支持</option></select></label><p>导出的是玩家可运行的程序，不是仅打包 Godot 源项目。</p><small>首版不包含 Steam、itch、Epic 或微信小游戏的渠道发布。</small></div></div><h2 class="section-gap">版本记录</h2><div class="list-row"><div><strong>第一晚 · 可玩预览</strong><small>示例版本 · 包含对话、调饮和立绘修改 · 待验收</small></div>${button("查看关联任务", "open-story-task")}</div><div class="list-row"><div><strong>任务前的项目状态</strong><small>按任务追踪变更；标准回退支持保留文件</small></div>${button("回退入口", "rollback")}</div>`;
  }
  function renderSettings() {
    return `<section class="page"><div class="page-head"><h1>设置</h1></div>${tabs(
      [
        ["ai", "AI 服务"],
        ["environment", "创作环境"],
        ["behavior", "运行与回退"]
      ],
      settingsTab,
      "settings-tab"
    )}<div class="scroll"><div class="settings-body">${settingsTab === "ai" ? renderAI(false) : settingsTab === "environment" ? renderEnvironment(false) : `<details class="fold" open><summary>后台运行</summary><div class="stack"><label class="row"><input type="checkbox" checked>关闭窗口后继续在本机执行任务</label><p class="muted">托盘右键退出整个程序时停止执行。下次打开保留结果，由用户明确继续，不自动产生新的调用费用。</p></div></details><details class="fold" open><summary>任务与回退</summary><div class="stack"><label class="row"><input type="checkbox" checked>每个任务保留修改前状态</label><label class="row"><input type="checkbox" checked>回退时允许保留指定文件</label><p class="muted">并发任务有重叠修改时展示冲突，不直接覆盖整个项目。标准回退与对话型回退是两个入口。</p></div></details><details class="fold"><summary>执行位置</summary><p class="section-gap">本机执行。云端执行是后续能力，与使用平台额度或自配 API 无关。</p></details>`}</div></div></section>`;
  }
  function renderAI(onboarding) {
    return `<div class="setup-choice"><button data-ai-mode="platform" class="${aiMode === "platform" ? "active" : ""}">平台额度<small>由 Beaver 平台提供 AI 服务</small></button><button data-ai-mode="custom" class="${aiMode === "custom" ? "active" : ""}">自己的 API<small>填写服务地址与密钥</small></button></div><div class="section-gap stack">${aiMode === "platform" ? `<div class="list-row"><div><strong>平台账户</strong><small>登录、额度和用量显示区域；不虚构余额</small></div>${button("账户入口", "account")}</div>` : `<label class="form-field"><span>API Base URL</span><input placeholder="你的兼容服务地址" aria-label="API Base URL" autocomplete="off"></label><label class="form-field"><span>API Key</span><input type="password" placeholder="布局原型中请勿填写真实密钥" aria-label="API Key" autocomplete="off"></label>${button("验证连接（演示）", "verify-api")}`}<details class="fold"><summary>按服务单独配置</summary><p class="muted section-gap">文本与编程、图像、音频等服务，都可以分别选择平台额度或自己的 API；展开后显示对应地址、密钥和模型配置。</p></details><p class="muted">AI 请求来源可切换；Codex、Godot 与 Blender 仍在本机执行。</p>${onboarding ? "" : button("保存配置（演示）", "save-ai", "primary")}</div>`;
  }
  function renderEnvironment(onboarding) {
    const state = environmentStep === 0 ? "待准备" : environmentStep === 1 ? "等待连接验证" : "已就绪（演示）";
    const rows = [
      ["游戏制作", "Godot · 引擎、导出模板与项目插件"],
      ["模型制作", "Blender · 应用、插件与连接配置"],
      ["AI 执行", "Codex · 独立配置、skills 与 MCP"],
      ["配套依赖", "相关运行时、文档检索与测试工具"]
    ];
    return `<div class="row"><h2>${environmentStep === 2 ? "环境已就绪（演示）" : "准备创作环境"}</h2><div class="spacer"></div>${!onboarding ? button("重新检测（演示）", "detect") : ""}</div>${rows.map(([title, description]) => `<div class="list-row"><div><strong>${title}</strong><small>${description}</small></div><span class="${environmentStep === 2 ? "positive" : "muted"}">${state}</span></div>`).join("")}<details class="fold"><summary>安装位置与已有环境</summary><div class="stack"><label class="form-field"><span>Beaver 管理目录</span><input placeholder="由用户确认的本地目录" aria-label="环境安装目录"></label><label class="row"><input type="checkbox" checked>优先复用兼容的已有应用</label><p class="muted">独立维护 Beaver 配置，不覆盖个人 Codex 配置。首次安装前显示组件来源、协议、空间与系统授权要求。</p></div></details><details class="fold"><summary>高级 · 插件、skills 与 MCP</summary><p class="section-gap">按应用分组展示已选能力、版本、连接状态与更新入口。普通用户无需编辑这些配置。</p></details><div class="result-actions">${button(environmentStep === 0 ? "自动准备（演示）" : environmentStep === 1 ? "验证连接（演示）" : "重新准备（演示）", "prepare", onboarding ? "" : "primary")}${button("查看失败状态", "install-error", "quiet")}</div>`;
  }
  function renderSetup() {
    const body = setupStep === 1 ? `<h1>选择 AI 服务</h1>${renderAI(true)}` : setupStep === 2 ? renderEnvironment(true) : `<h1>开始你的游戏</h1><div class="setup-choice"><button data-action="new-project">创建新游戏<small>选择起步模板、基础方向和初始功能包</small></button><button data-action="import-project">接管已有项目<small>选择 Godot 项目，不强行覆盖游戏模板</small></button></div>`;
    return `<section class="page"><div class="scroll"><div class="setup"><nav class="setup-steps" aria-label="首次使用步骤"><span class="${setupStep === 1 ? "active" : ""}">1 · AI 服务</span><span class="${setupStep === 2 ? "active" : ""}">2 · 准备环境</span><span class="${setupStep === 3 ? "active" : ""}">3 · 开始创作</span></nav>${body}</div></div><footer class="page-footer"><small>流程预览 · 不产生真实下载、安装或 AI 调用</small><div class="spacer"></div>${setupStep > 1 ? button("上一步", "setup-back") : ""}${setupStep < 3 ? `<button class="primary" data-action="setup-next" ${setupStep === 2 && environmentStep !== 2 ? "disabled" : ""}>${setupStep === 1 ? "下一步" : "开始创作"}</button>` : ""}</footer></section>`;
  }
  function newTask(goal = "", references = []) {
    modal(
      "新建创作任务",
      `<div class="stack"><label class="form-field"><span>你希望完成什么？</span><textarea id="new-task-goal" placeholder="描述你想要的游戏效果…">${escape(goal)}</textarea></label>${references.length ? `<div><small>你选中的参考</small><p>${references.map(escape).join(" · ")}</p></div>` : "<small>可以不指定资料，由 Codex 自己查找所需上下文。</small>"}<details class="fold"><summary>停止条件与限制</summary><div class="stack"><label class="form-field"><span>在什么情况下停下</span><textarea placeholder="默认持续执行至目标完成；也可以填写你希望的停止条件。"></textarea></label><label class="form-field"><span>不得修改的范围</span><input placeholder="例如：不要改第一晚已经确认的剧情"></label></div></details></div>`,
      "发起任务（演示）",
      () => {
        const goal2 = element("#new-task-goal").value.trim();
        if (!goal2) {
          element("#new-task-goal").focus();
          return;
        }
        const id = Math.max(...tasks.map((item) => item.id)) + 1;
        tasks.unshift({
          id,
          title: goal2,
          status: "working",
          accepted: false,
          refs: references,
          messages: [],
          draft: ""
        });
        selectedTask = id;
        taskFilter = "all";
        page = "create";
        createTab = "conversation";
        closeModal();
        render();
        notice("已添加一个演示任务，没有调用 Codex。");
      }
    );
  }
  function newProject(importing) {
    modal(
      importing ? "接管已有项目" : "创建新游戏",
      importing ? `<div class="stack"><label class="form-field"><span>Godot 项目目录</span><input id="new-project-path" placeholder="选择包含 project.godot 的目录"></label><label class="form-field"><span>项目名称</span><input id="new-project-name" value="我的已有游戏"></label><p class="muted">先识别现有项目，不应用起步模板。已有资料和素材保留，后续通过任务整理与继续制作。</p></div>` : `<div class="stack"><label class="form-field"><span>游戏名称</span><input id="new-project-name" value="我的新游戏"></label><label class="form-field"><span>起步模板 · 仅在创建时应用</span><select><option>原创对话调饮</option><option>空白 Godot 项目</option><option>其他游戏模板（目录占位）</option></select></label><div class="form-grid"><label class="form-field"><span>主类别</span><select><option>视觉小说</option><option>其他类别（卡牌选择区）</option></select></label><label class="form-field"><span>副类别</span><select><option>无</option><option>模拟</option></select></label><label class="form-field"><span>主题</span><input placeholder="选择主题或自行填写"></label><label class="form-field"><span>游戏大小</span><select><option>独立</option><option>标准</option><option>大型</option><option>AAA</option></select></label><label class="form-field"><span>目标评级</span><select><option>全年龄</option><option>青少年</option><option>成年人</option></select></label><label class="form-field"><span>表现风格</span><select><option>像素 2D</option><option>手绘 2D</option><option>低多边形 3D</option></select></label></div><label class="row"><input type="checkbox" id="new-project-online">在线多人游戏</label><div id="new-project-network" hidden>${button("服务器容量与架构", "network")}</div><details class="fold"><summary>初始功能包</summary><label class="choice-row"><input type="checkbox" checked>对话与分支</label><label class="choice-row"><input type="checkbox" checked>存档与读档</label><small>可以以后再添加；不同于起步模板。</small></details></div>`,
      importing ? "进入接管布局" : "进入新游戏布局",
      () => {
        const name = element("#new-project-name").value.trim();
        if (!name) return;
        game = { ...game, name };
        gameDraft = null;
        page = "game";
        gameTab = "overview";
        closeModal();
        render();
        notice("仅切换到项目布局，没有创建目录、导入文件或应用模板。");
      }
    );
  }
  function rollback() {
    modal(
      "回退任务",
      `<div class="stack"><p>将“${escape(task().title)}”的修改恢复到任务前。</p><label class="choice-row"><input type="radio" name="rollback" value="standard" checked><span>标准回退<small>按任务变更记录执行确定性回退，可保留指定文件。</small></span></label><label class="choice-row"><input type="radio" name="rollback" value="conversation"><span>对话型回退<small>描述要恢复或保留的内容，交给 Codex 处理。</small></span></label><details class="fold"><summary>保留指定文件</summary><label class="choice-row"><input type="checkbox">保留当前角色立绘</label><label class="choice-row"><input type="checkbox">保留角色视觉规范.md</label></details><p class="inline-note">如果其他并行任务也修改了这些文件，先展示冲突与受影响任务，不覆盖其他任务的成果。</p></div>`,
      "查看下一步",
      () => {
        const conversational = dialog.querySelector('input[name="rollback"]:checked')?.value === "conversation";
        closeModal();
        if (conversational)
          newTask("回退这个任务的部分修改，保留我认可的立绘。", [task().title]);
        else
          notice(
            "标准回退下一步：列出恢复文件、保留文件与冲突，再由用户确认。原型未回退任何文件。"
          );
      }
    );
  }
  var actions = {
    collapse: () => {
      const collapsed = app.classList.toggle("collapsed");
      const control = element("#collapse");
      control.setAttribute("aria-expanded", String(!collapsed));
      control.setAttribute("aria-label", collapsed ? "展开侧栏" : "收起侧栏");
    },
    guide: () => {
      const guide = element("#guide");
      guide.hidden = !guide.hidden;
      element('[data-action="guide"]').setAttribute(
        "aria-expanded",
        String(!guide.hidden)
      );
      if (!guide.hidden) guide.scrollIntoView({ block: "start" });
    },
    onboarding: () => {
      page = "onboarding";
      setupStep = 1;
      environmentStep = 0;
      render();
    },
    workspace: () => {
      if (dialog.open) closeModal();
      page = "create";
      render();
    },
    environment: () => {
      page = "settings";
      settingsTab = "environment";
      render();
    },
    projects: () => modal(
      "项目",
      `<div class="list-row"><div><strong>${escape(game.name)}</strong><small>当前示例项目</small></div>${button("进入", "workspace")}</div><div class="result-actions">${button("创建新游戏", "new-project")}${button("接管已有项目", "import-project")}</div>`
    ),
    "new-project": () => newProject(false),
    "import-project": () => newProject(true),
    "new-task": () => newTask(),
    "close-modal": closeModal,
    "confirm-modal": () => modalAction?.(),
    "dismiss-notice": () => {
      element("#notice").hidden = true;
    },
    "stop-task": () => {
      task().status = "paused";
      render();
      notice("中止状态演示：保留结果和对话，可稍后继续。");
    },
    "resume-task": () => {
      task().status = "working";
      render();
      notice("继续执行状态演示，没有产生 AI 调用。");
    },
    later: () => notice("可以切换到其他任务，之后从任务列表继续。"),
    "accept-result": () => {
      task().accepted = true;
      render();
      notice("仅将这个示例结果标为已认可，不代表其他任务已完成。");
    },
    "focus-feedback": () => element("#task-input").focus(),
    "send-feedback": () => {
      const current = task();
      if (!current.draft.trim()) {
        element("#task-input").focus();
        return;
      }
      current.messages.push(current.draft.trim());
      current.draft = "";
      createTab = "conversation";
      render();
      notice(
        current.status === "paused" ? "补充已保留在演示对话中；任务仍中止，需明确继续。" : "补充已显示在演示对话中，没有实际生成或修改。"
      );
    },
    references: () => modal(
      "添加参考",
      `<p>只添加你主动选择的参考，不限制 Codex 自己查找其他资料。</p>${assets.slice(0, 4).map(
        (asset) => `<label class="choice-row"><input type="checkbox" value="${asset.name}" data-reference>${asset.name}</label>`
      ).join("")}`,
      "添加到当前任务",
      () => {
        const refs = Array.from(
          dialog.querySelectorAll("[data-reference]:checked")
        ).map((node) => node.value);
        task().refs = [.../* @__PURE__ */ new Set([...task().refs, ...refs])];
        closeModal();
        createTab = "conversation";
        render();
      }
    ),
    limits: () => modal(
      "执行限制",
      '<div class="stack"><label class="form-field"><span>停止条件</span><textarea placeholder="默认持续执行至目标完成"></textarea></label><label class="form-field"><span>不得修改的内容</span><textarea placeholder="按需填写"></textarea></label><small>用于限制边界，不在应用中编排 Codex 的内部步骤。</small></div>',
      "保存限制（演示）",
      () => {
        closeModal();
        notice("限制编辑入口演示，未更改真实任务。");
      }
    ),
    "task-changes": () => {
      createTab = "changes";
      render();
    },
    rollback,
    "review-assets": () => {
      page = "assets";
      assetFilter = "图片";
      selectedAssets = /* @__PURE__ */ new Set(["portrait-1", "portrait-2", "portrait-3"]);
      render();
    },
    "open-art-doc": () => {
      page = "docs";
      selectedDoc = "art";
      render();
    },
    "open-direction": () => {
      page = "docs";
      selectedDoc = "direction";
      render();
    },
    "open-story-task": () => {
      page = "create";
      selectedTask = 2;
      taskFilter = "all";
      render();
    },
    "new-doc": () => newTask("为游戏补充一份资料，并归档到合适的文档目录。"),
    "edit-doc": () => {
      editingDoc = true;
      render();
    },
    "cancel-doc": () => {
      editingDoc = false;
      render();
    },
    "save-doc": () => {
      docs[selectedDoc].text = element("#doc-editor").value;
      editingDoc = false;
      render();
      notice("仅更新网页中的示例文档，刷新还原；未修改项目文件。");
    },
    "revise-doc": () => newTask(`请修改《${docs[selectedDoc].title}》：`, [docs[selectedDoc].path]),
    "import-reference": () => notice("此处放本地文件选择、拖入参考与归档入口；原型不读取你的文件。"),
    "asset-feedback": () => selectedAssets.size ? newTask(
      "请调整这些素材：",
      assets.filter((asset) => selectedAssets.has(asset.id)).map((asset) => asset.name)
    ) : notice("先选择需要修改的素材。"),
    "asset-reference": () => selectedAssets.size ? newTask(
      "根据这些参考生成新的素材：",
      assets.filter((asset) => selectedAssets.has(asset.id)).map((asset) => asset.name)
    ) : notice("先选择参考素材。"),
    "asset-style": () => selectedAssets.size ? newTask(
      "统一这些素材的风格，保留内容含义。",
      assets.filter((asset) => selectedAssets.has(asset.id)).map((asset) => asset.name)
    ) : notice("先选择需要统一风格的素材。"),
    "game-feedback": () => newTask("试玩后，我希望调整：", ["当前可玩版本 / 游戏截图"]),
    play: () => {
      if (dialog.open) closeModal();
      page = "game";
      gameTab = "builds";
      render();
    },
    export: () => {
      page = "game";
      gameTab = "builds";
      render();
    },
    "launch-game": () => notice("这里启动独立的游戏试玩窗口；本页只有画面与交互占位。"),
    build: () => modal(
      "导出可运行游戏",
      '<div class="stack"><p>Windows 玩家程序</p><p class="muted">目标区域：构建检查、导出目录、完成后的启动与打开文件夹入口，以及失败原因和重试。</p><label class="form-field"><span>输出位置</span><input placeholder="选择导出目录"></label></div>',
      "查看导出结果布局",
      () => {
        closeModal();
        notice(
          "导出结果区域：玩家程序路径、启动游戏、打开目录。原型没有生成游戏程序。"
        );
      }
    ),
    "discard-game": () => {
      gameDraft = null;
      render();
    },
    "save-game": () => {
      if (!gameDraft?.name.trim()) {
        notice("游戏名称不能为空。");
        return;
      }
      game = { ...gameDraft, name: gameDraft.name.trim() };
      gameDraft = null;
      render();
      modal(
        "项目设定已更改，存在风险",
        '<div class="stack"><p class="danger">基础方向变化可能使现有剧情、素材、玩法或联机结构不再兼容。</p><p>需要修改游戏时，应另行发起任务并验收。保存设定不代表已经完成游戏重构或兼容检查。</p><small>这是保存后的强提示布局，仅改变本页示例数据。</small></div>',
        "我已知晓",
        closeModal
      );
    },
    network: () => modal(
      "在线多人规划",
      '<div class="form-grid"><label class="form-field"><span>网络架构</span><select><option>独立服务器</option><option>玩家主机</option></select></label><label class="form-field"><span>服务区域</span><input placeholder="例如：亚洲"></label><label class="form-field"><span>每局人数</span><input type="number" value="4"></label><label class="form-field"><span>峰值同时在线人数</span><input type="number" value="100"></label><label class="form-field"><span>初期服务器实例数</span><input type="number" value="1"></label></div><p class="muted section-gap">这里只表达规划，不购买或部署服务器。</p>',
      "应用（演示）",
      () => {
        closeModal();
        notice("服务器规划弹窗占位；未部署或保存真实容量配置。");
      }
    ),
    "save-priorities": () => notice("保存创作重点的入口，不会按这三个阶段替 Codex 拆分或编排任务。"),
    "feature-detail": () => notice("详情区域：功能说明、来源版本、项目定制、关联文件与历史任务。"),
    "feature-plan": () => notice("规划项已选择的状态演示，不代表该功能已实现或已接入。"),
    "feature-add": () => newTask("为游戏接入这个功能包，并验证现有玩法未被破坏。", [
      "对话分支源码包"
    ]),
    "feature-update": () => modal(
      "对话与分支 · 更新差异",
      '<div class="stack"><p>上次采用 v1.0 → 官方 v1.1</p><div class="list-row"><div><strong>修复快速点击跳过选项的问题</strong><small>官方增量变更</small></div></div><div class="list-row"><div><strong>增加对话历史接口</strong><small>官方增量变更</small></div></div><p class="inline-note">你的项目已经修改过此功能。保留基线、当前项目和新版差异，让 Codex 合并，不直接覆盖。</p><small>以上为功能包更新布局的示例内容。</small></div>',
      "交给 AI 合并",
      () => {
        closeModal();
        newTask("根据官方版本差异更新对话功能，保留项目已有定制。", [
          "官方 v1.0",
          "项目当前版本",
          "官方 v1.1 与差异"
        ]);
      }
    ),
    account: () => notice("平台登录与额度展示入口。此原型不登录、不充值，也不显示虚构余额。"),
    "verify-api": () => notice("这里显示连接验证成功或失败的结果；原型没有发送网络请求。"),
    "save-ai": () => notice("配置保存入口演示。没有存储地址或密钥，请勿在原型中填写真实凭据。"),
    prepare: () => {
      environmentStep = environmentStep === 2 ? 0 : environmentStep + 1;
      render();
      notice(
        environmentStep === 1 ? "演示下一状态：应用与插件准备后，验证工具连接。未执行真实安装。" : environmentStep === 2 ? "就绪布局演示，不代表本机工具实际已安装或连接。" : "已重置演示状态。"
      );
    },
    detect: () => {
      environmentStep = 0;
      render();
      notice("检测状态示例，没有扫描本机程序。");
    },
    "install-error": () => modal(
      "Blender 插件连接失败 · 状态示例",
      '<div class="stack"><p class="danger">无法验证插件连接，环境尚未就绪。</p><p>在这里展示失败的组件、具体原因、可重试的步骤和用户需要授予的系统权限。</p><details><summary>查看诊断记录</summary><p class="muted section-gap">诊断摘要与日志入口。不会要求普通用户先理解 MCP 配置。</p></details></div>',
      "重试这一步（演示）",
      () => {
        closeModal();
        environmentStep = 1;
        render();
        notice("已回到连接验证布局，没有重装应用。");
      }
    ),
    "setup-next": () => {
      setupStep = Math.min(3, setupStep + 1);
      render();
    },
    "setup-back": () => {
      setupStep = Math.max(1, setupStep - 1);
      render();
    },
    future: () => notice("后续版本的渠道与宣传能力；不属于本轮业务实现。")
  };
  document.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target.closest("button") : null;
    if (!target || target.disabled) return;
    const data = target.dataset;
    if (data.action) {
      actions[data.action]?.();
      return;
    }
    if (data.page && ["create", "docs", "assets", "game", "settings"].includes(data.page)) {
      page = data.page;
      editingDoc = false;
    }
    if (data.task) {
      selectedTask = Number(data.task);
    }
    if (data.createTab) createTab = data.createTab;
    if (data.gameTab) gameTab = data.gameTab;
    if (data.settingsTab) settingsTab = data.settingsTab;
    if (data.doc) {
      selectedDoc = data.doc;
      editingDoc = false;
    }
    if (data.assetFilter) assetFilter = data.assetFilter;
    if (data.asset) selectedAssets = /* @__PURE__ */ new Set([data.asset]);
    if (data.aiMode) aiMode = data.aiMode;
    if (data.featureTab) featureTab = data.featureTab;
    render();
  });
  document.addEventListener("input", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement))
      return;
    if (target.id === "task-input") task().draft = target.value;
    if (gameDraft && target.dataset.gameField && target.dataset.gameField !== "online") {
      const key = target.dataset.gameField;
      gameDraft[key] = target.value;
    }
  });
  document.addEventListener("change", (event) => {
    const target = event.target;
    if (!(target instanceof HTMLInputElement || target instanceof HTMLSelectElement))
      return;
    if (target.id === "task-filter") {
      taskFilter = target.value;
      render();
    }
    if (target instanceof HTMLInputElement && target.dataset.assetCheck) {
      if (target.checked) selectedAssets.add(target.dataset.assetCheck);
      else selectedAssets.delete(target.dataset.assetCheck);
      render();
    }
    if (target.id === "edit-game" && target instanceof HTMLInputElement) {
      if (target.checked) {
        gameDraft = { ...game };
        render();
      } else {
        target.checked = true;
        modal(
          "放弃设定草稿？",
          "<p>关闭编辑会丢弃本次未保存的基础设定。</p>",
          "放弃修改并锁定",
          () => {
            gameDraft = null;
            closeModal();
            render();
          }
        );
      }
    }
    if (gameDraft && target.dataset.gameField && target.dataset.gameField !== "online")
      gameDraft[target.dataset.gameField] = target.value;
    if (target.id === "game-online" && target instanceof HTMLInputElement && gameDraft) {
      gameDraft.online = target.checked;
      render();
    }
    if (target.id === "new-project-online" && target instanceof HTMLInputElement) {
      const network = element("#new-project-network");
      network.hidden = !target.checked;
      if (target.checked)
        network.innerHTML = '<div class="form-grid"><label class="form-field"><span>网络架构</span><select><option>独立服务器</option><option>玩家主机</option></select></label><label class="form-field"><span>每局人数</span><input type="number" value="4"></label><label class="form-field"><span>峰值在线人数</span><input type="number" value="100"></label><label class="form-field"><span>初期服务器实例数</span><input type="number" value="1"></label></div>';
    }
  });
  render();
})();

# Beaver 本地 Godot 工具链锚定

## 当前绑定

2026-09-30 按用户要求，将当前 Beaver 使用的 Windows Godot 编辑器和导出模板绑定到以下发布目录，不混用先前的 `4.5.1-stable` 工具或其他版本的模板：

```text
C:\Users\Public\nas_home\godot\export
```

| 角色                    | 文件                                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------ |
| 编辑器                  | `godot.windows.editor.x86_64.exe`                                                                      |
| 编辑器 console launcher | `godot.windows.editor.x86_64.console.exe`                                                              |
| Debug 模板              | `godot.windows.template_debug.x86_64.exe`                                                              |
| Release 模板            | `godot.windows.template_release.x86_64.exe`                                                            |
| 模板 console launchers  | `godot.windows.template_debug.x86_64.console.exe`、`godot.windows.template_release.x86_64.console.exe` |
| 配套依赖                | `SpoutLibrary.dll`                                                                                     |

当前发布构建的完整版本为 `4.8.dev.custom_build.38b6ddee7`，engine commit 为 `38b6ddee72e16d9d646057ee6ba533c122afc47c`，发布时间为 `2026-09-24T03:12:28.6489459Z`。七个 artifacts 的 SHA-256 均与目录中的 `publication-manifest.json` 一致，见 [实物核验](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/artifacts-proof.json)。

绑定通过 `settings.save` 保存到当前实例的工具设置，不把私有 Windows 绝对路径写入跨平台默认配置，不覆盖项目级 `beaver.runtime.json` 的显式引擎绑定。其他工具、模型设置和密钥保持不变，见 [设置变更证明](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/settings-proof.json)。

## 模板准备与导出规则

- 识别上述源码构建 editor 文件名及其 console launcher，强制使用同目录的 debug/release 模板。
- 模板必须是普通文件、不是 symlink、至少 1024 bytes；实际执行 editor、debug 和 release 的 `--version`，完整版本必须相同，包括 custom build 标识。
- `game.prepareTemplates` 校验成功后返回 `reused: true` 和 `toolchain.source: "editor-siblings"`。不下载官方模板，不安装到 APPDATA，也不接受另外的 `.tpz` 混配。
- 缺少模板、版本不一致、项目显式模板与本地配套模板冲突时明确失败，不回退到旧版本或官方版本。官方编辑器仍保留原有官方稳定版模板准备方式；custom build 不能走官方下载。
- Windows 导出仅支持这套模板的 `x86_64`。选定预设必须唯一存在；仅在导出执行副本中注入 `custom_template/debug`、`custom_template/release`，保留其他预设，不改原项目和冻结输入快照。
- Native 导出复用 PE imports 依赖收集，从配套目录复制需要的 DLL；TypeScript 路径使用独立副本并以非覆盖方式复制同目录普通 DLL。导出结果和 manifest 都保存 `toolchain` 回执。
- UI 操作显示为“准备 Windows 模板”，不再暗示本地 custom build 必须下载官方模板。

源码入口为 `native/core/src/godot_bundle.rs`、`native/core/src/export_templates.rs`、`native/core/src/game_export.rs`，对应 TypeScript 入口为 `src/core/godot-bundle.ts`、`src/core/game-export-workspace.ts` 和 `src/core/game.ts`。

## 本轮真实验证

本轮构建并启动原生开发版 `0.1.19.39`，public SemVer 保持 `0.1.19`。启动脚本使用新数据/WebView 目录，经 Beaver API 创建新 blank 项目；旧实例无运行或排队任务，旧 EXE 备份与原有项目均保留。

| 验证                  | 实际结果                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------------ |
| Core bundle 回归      | `cargo test --locked -p beaver-core --lib godot_bundle`，4 passed                                      |
| Core 导出回归         | `cargo test --locked -p beaver-core --lib game_export`，10 passed                                      |
| Core 模板回归         | `cargo test --locked -p beaver-core --lib export_templates`，4 passed；拆分测试文件后再次 4 passed     |
| TypeScript 定向回归   | `npx tsx --test tests/godot-bundle.test.ts tests/export-templates.test.ts`，8 passed                   |
| 编译与类型            | `npm run typecheck`、`cargo check --locked -p beaver-desktop`、`npm run build:native` 通过             |
| 有效行门禁            | 1205 sources、17 unchanged legacy files、0 violations；未更新 baseline 或新增 exception                |
| 生产 API 模板准备     | 返回完整 custom build 版本、本地配套路径及 `reused: true`                                              |
| 生产 API Windows 导出 | `game.export` 的 internal 导出及 `game.verifyExport` 成功，包含 `Game.exe` 和 `spoutlibrary.dll`       |
| 原项目保护            | `project.godot`、`main.tscn`、`export_presets.cfg` 导出前后 SHA-256 相同                               |
| 独立启动检查          | 导出程序执行 `--headless --quit-after 5`，退出码 0，显示预期 Godot 版本，无 `ERROR:` 或 `SCRIPT ERROR` |

第一轮构建发现 `export_templates.rs` 为 507 有效行，结构门禁正确阻止交付。原 inline tests 移入 `export_templates_tests.rs` 后，生产文件为 289 有效行，测试文件为 218 有效行；重新通过定向模板测试及构建门禁。Rust/TypeScript 使用定向 formatter 检查，不改无关文件。

主要证据位于 `C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-godot-anchor-20260930`：

- [新轮启动证明](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/native-start-proof.json)
- [模板回执](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/native-template-receipt.json)
- [API 导出、包校验及原项目保护](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/native-toolchain-proof.json)
- [独立 headless 启动证明](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/export-start-proof.json)
- [构建输出](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/native-build.stdout.log)和 [Cargo 原始日志](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/native-build.stderr.log)
- [恢复原项目实例证明](../../AI/GameEditor/linshi/beaver-godot-anchor-20260930/restored-instance-proof.json)

测试后关闭 blank 测试实例，用 `.39` 恢复原来的 `fresh-round-20260929-030013-76152f2c95714d02a3ffd745f17a58bd` 数据及 WebView 目录。端口为 `4341`，项目 `8e7cf51a-7c02-4a91-a1a2-3e34d54d2439` 和两项 completed 任务历史保留，活动任务为 0，Godot 设置重开后仍指向发布目录；当前仅运行这一份 Beaver。

可运行入口为 `C:\Users\Public\nas_home\Beaver\target\release\Beaver.exe`，ProductVersion 为 `0.1.19.39`，SHA-256 为 `88334a06d210297c04c443aed353776e1f9af1f2023ae2506e9fa1f35698c314`。旧 `.38` EXE 备份为证据目录内的 `Beaver-0.1.19.38-before-anchor.exe`。

## 原木箱项目兼容性续验

恢复保留轮次后，使用相同 `.39` 和上述配套 `4.8.dev.custom_build.38b6ddee7`，已经通过 Beaver `game.play` 实际图形运行原木箱入口，再经 `game.export` / `game.verifyExport` 完成内部 Windows 导出与包校验。导出回执的 `toolchain.source` 为 `editor-siblings`，editor 和 debug/release 模板均来自用户指定目录。

正常启动导出 `Game.exe`，没有使用 headless、`--path` 或 `--main-pack` override。原项目与导出程序各两张准确窗口 PNG 已实际查看，木箱、中文 UI 和自动旋转正常；日志无 `ERROR:` 或 `SCRIPT ERROR`，导出 stderr 为空，正常关闭的导出程序退出码为 `0`。项目运行也已正常关闭，但未捕获退出码。27 项原项目受保护文件 SHA 和两项 completed/accepted 任务执行状态均保持不变；唯一 Beaver 实例仍保留供手测，没有重复创建游戏任务。

本段证据位于 `C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-crate-godot48-20260930`，包括 [完整兼容性证明](../../AI/GameEditor/linshi/beaver-crate-godot48-20260930/compatibility-proof.json)、[导出及包校验](../../AI/GameEditor/linshi/beaver-crate-godot48-20260930/export-proof.json)、[准确窗口正常关闭](../../AI/GameEditor/linshi/beaver-crate-godot48-20260930/process-closure.json)和 [最终只读状态](../../AI/GameEditor/linshi/beaver-crate-godot48-20260930/final-readonly-state.json)。完整结果、图像与边界见 [验收报告第 6.14 节](NATIVE-ACCEPTANCE-2026-09-28.md#614-2026-09-30-godot-48-原项目兼容性续验)。

## 验证边界与后续入口

Godot 发布清单保留了历史限制：`Template PCK export passed, template launch blocked by path-overrides build setting.` 本轮采用 embedded PCK 的实际 Windows EXE、无 `--path`/`--main-pack` override，已经通过独立 headless 启动；这不表示上述 override 限制已被修复，也没有修改或重编 Godot。

先前 blank 验证仅覆盖本地工具链准备、Windows 导出与短时启动；本段又补齐原木箱项目在新引擎下的图形运行、导出与独立 EXE 实跑，不是新 blank AI 创作验收，也未覆盖 4.8 下完整交互、GUT 或正式视觉流程、Blender、所有平台及正式 release。最新只读核查确认旧 `4.5.1` 视觉 run 已有 `source: user` 的完整确认并为 `userPassed`，不再重复要求用户确认该旧证据，但该确认不跨引擎版本。导出 manifest 的 `runtimeVerified: false` 保持原样，独立运行结果单独存档；不以 smoke 或图形启动证明改写产品 runtime 验收状态。

[框架计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)的 F7/F8/F9 与最终完成条件保持未勾选；下一步仍按尚未完成的具体用户工作流推进。本轮没有提交、推送、改个人 Codex provider/model 配置或代替用户确认视觉基线。

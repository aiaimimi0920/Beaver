# F4.4 交互预览关闭确认与重试证据

记录日：2026-10-01；部分验证日志的 UTC 日期为 2026-10-02。
源码开发迭代：`0.1.19.44`，公共 SemVer 保持 `0.1.19`。

## 已完成的可用流程

在生产交互预览中点击“关闭交互预览”，窗口等待当前会话 worker 收尾确认后
才退出。等待期间显示“正在关闭交互预览…”并禁止新增交互；响应丢失、超时
或错误回执时保留窗口和准确的原会话身份，提供“重试关闭交互预览”。重试
不会停止其他会话，不取消制作任务，也不改变候选、接受或发布状态。

这是 [开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 中 F4.4 的预览资源生命周期
子流程，不是完整对象取消与资源恢复闭环。F4.4、F7.3 和整体验收仍保持未完成。

## Core、Desktop 与生产 UI 的边界

`validation.preview.close` 请求仍使用 `projectId/sessionId`，返回精确身份回执：

```json
{ "closed": false, "projectId": "p", "sessionId": "s" }
```

- Core 先向匹配会话设置 stop；worker 尚未结束时返回 `closed:false`，只有
  finished 且 `join` 成功才返回 `closed:true`。worker panic 会保留
  `PREVIEW_WORKER_FAILED`，后续重试不会伪装成成功。
- 同会话的错误项目身份被拒绝。旧或不存在的会话关闭请求幂等返回成功，
  不触碰当前替代会话；替换会话也必须先确认旧 worker finished/join。
- Desktop 沿用现有生产分发及请求 schema，工具说明明确 pending 回执和
  同身份重试要求；没有新增另一路关闭 API。
- 前端只接受身份匹配且 schema 正确的回执。`closed:false` 每 100ms 继续
  查询原请求，整个显式关闭最多等待 10 秒；错误或超时由用户显式重试。
  超时后的迟到回执不会自行把失败改为成功，重复点击共用一个关闭 promise。
- 显式关闭等待 in-flight open/read；迟到结果只保留正确 session 身份，
  不更新显示或恢复交互。丢失 open 响应只重试此前已发送的原 requestId；
  从未打开的 viewer 不为关闭而创建新会话。
- 关闭等待及失败期间禁止 refresh、camera、resolution、freeze、capture
  和 pick。生产按钮只有在 `requestClose()` 确认后才调用 `onClose()`。
  卸载时的 `close()` 仍是 best-effort cleanup，不等于 awaited 显式关闭。

主要文件：

- `native/core/src/validation/live_preview.rs` 和独立关闭生命周期测试。
- `native/desktop/src/validation_catalog.rs`。
- `src/ui/object-preview/live-scene-preview.ts`：预览控制器与关闭状态。
- `src/ui/object-preview/preview-close.ts`：身份回执及有界等待。
- `src/ui/object-preview/LiveScenePreview.tsx`：等待、失败、重试生产入口。
- `tests/live-scene-preview-close.test.ts`。

## 实际通过的定向验证

以下结果属于同一完成切片；文档收尾不重复执行应用构建或功能测试。
原始日志保存在
[beaver-preview-close-20261001](../../AI/GameEditor/linshi/beaver-preview-close-20261001/)。

| 检查                                                          | 实际结果                                                   | 日志                                           |
| ------------------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------- |
| 前端关闭、相邻预览/拾取/选择及产品版本/包契约                 | 27 passed / 0 failed；其中新增关闭测试 9 项                | `frontend-tests.log`                           |
| TypeScript `npm run typecheck`                                | exit 0                                                     | `typecheck.log`                                |
| Core `validation::live_preview`                               | 6 passed / 0 failed / 1 ignored；其中新增生命周期测试 5 项 | `core-tests.log`                               |
| Desktop `validation_runtime::tests`                           | 7 passed / 0 failed，属于相邻路由/存储回归                 | `desktop-tests.log`                            |
| `npm run check:native`                                        | exit 0                                                     | `desktop-check.log`                            |
| 本次 TS/TSX、测试和 package 的 Prettier；本次 Rust 的 rustfmt | 通过                                                       | `prettier.log`；交付时另做定向 formatter check |
| `npm run check:effective-lines`                               | 1232 sources / 17 unchanged legacy files / 0 violations    | `effective-lines.log`                          |

行为验证命令：

```powershell
npx --no-install tsx --test tests/live-scene-preview.test.ts tests/live-scene-preview-close.test.ts tests/preview-picking.test.ts tests/preview-selection.test.ts tests/product-version.test.ts tests/native-release.test.ts
npm run typecheck
cargo test --locked -p beaver-core --lib validation::live_preview
cargo test --locked -p beaver-desktop validation_runtime::tests
npm run check:native
npm run check:effective-lines
```

Core ignored 项是需要 `BEAVER_TEST_GODOT` 和图形 Godot 的真实相机/关闭/lease
测试，本轮没有执行。既有 `unused_mut`、`dead_code`、`unused_imports` warnings
保留，不扩展清理，也不把 warnings 或 PowerShell stderr 包装当作失败。

有效行检查未更新 baseline。本次源文件最大为预览控制器 409 effective lines；
Core 会话管理 345、Core 关闭测试 157、Desktop catalog 53、回执等待模块 47、
生产 UI 289、前端关闭测试 258，均在当前结构门禁范围内。

## 隔离浏览器中的生产 UI 证明

Microsoft Edge headless 中直接 bundle 生产 `LiveScenePreview.tsx`，只注入 mock
API；未访问已有 Beaver API 或业务数据，未调用模型、Godot 或 Blender。

1. pending：`closed:false` 时预览保留，dismissed 为 0，camera disabled；
   控制返回 true 后 dismissed 为 1，两次 stop 均为精确 `p/s`。
2. retry：第一回响应丢失仍保留窗口、重试按钮和会话身份说明；第二回使用
   同一 `p/s`，成功后 dismissed 为 1。
3. openRace：open 未返回时点击关闭，不提前 dismiss、不发无身份 stop；
   late open 到达后只发一次正确 stop，不重复 open。

证据为 `browser-pending.log`、`browser-retry.log`、`browser-open-race.log`、
`pending.png`、`retry.png`。console 为 0 errors / 0 warnings；失败截图已人工
核对，中文显示正常。临时 browser session 和 server 均已关闭。
这证明生产 UI/controller wiring，不代表 native WebView 或真实引擎验收。

## 尚未覆盖与交付边界

worker 已结束并 joined，不等于独立认证整个引擎进程树。既有
`process::OwnedChild` 精确 PID 清理及其错误处理未在本轮扩展；完整 owner
资源归属、对象取消/重启/恢复、真实 Godot/Blender/native 关闭仍需后续切片。

本次没有构建 `.44` 原生 EXE，没有正式 release 或全量端到端验收。固定
`release/Beaver-native-0.1.19.43-win32-x64` 未替换，原手测入口继续指向 `.43`；
它尚不包含本轮关闭确认功能。手测状态见
[当前开发状态与手工测试入口](MANUAL-TEST-READINESS.md)。

未代用户保存最终 output 编号帧、批准蓝色斜撑候选、修改冻结证据或发布。
既有数据、凭据、release、非本切片的 asset-task 修改和 graphify cache 保留。

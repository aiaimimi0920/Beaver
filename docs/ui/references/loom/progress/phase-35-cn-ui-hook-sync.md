# Phase 35: Chinese Desktop UI and Hook Screenshot Workflow Entry

## Goal

Make Loom's desktop UI easier to understand for Chinese users and expose the
Hook screenshot-to-workflow route directly in the Hook Bridge page.

The user-facing requirement was:

- Translate Loom-related UI text to Chinese.
- Remove large explanatory UI copy that does not help operation.
- Make the Hook screenshot sync to Loom workflow path visible in the UI.

Protocol names and compatibility identifiers remain in English where they are
machine contracts, for example `art_hook/instantiate`,
`art_loom/update_workflow_node`, `read_arthook_session`, `shm_create_buffer`,
`image_path`, `image_base64`, and `image_buffer`.

## Implemented

### Desktop UI localization

- Translated major navigation labels:
  - `总览`
  - `MCP`
  - `Art 注册表`
  - `工作流管理`
  - `Hook 桥接`
  - `工作流工作台`
  - `智能体`
  - `运行记录`
  - `设置`
  - `关于`
- Translated the main Loom desktop panels:
  - Overview/status
  - MCP service and marketplace management
  - Art registry and Add Art wizard
  - Workflow Manager
  - Workflow Studio and Art node graph
  - Hook Bridge
  - Settings
  - About
- Removed or shortened long explanatory paragraphs across the desktop shell.
  The UI now favors labels, buttons, cards, status text, and protocol chips.
- Kept product naming as `Loom`; no `Neuro` prefix was introduced.

### Hook screenshot workflow entry

The Hook Bridge page now has a visible card:

```text
Hook 截图
截图同步到工作流
Hook 截图 -> Loom 工作流 -> Art 节点
```

Actions exposed in the card:

- `同步截图上下文`
- `生成/刷新工作流`
- `广播同步`

Compatibility method chips shown in the same card:

- `art_hook/instantiate`
- `art_loom/update_workflow_node`
- `art_loom/workflow_updated`

The desktop entry is implemented in:

```text
Loom/apps/desktop/src/App.tsx
```

Relevant functions in `HookBridgePanel`:

- `syncDesktopHook`
- `refreshHookWorkflow`
- `broadcastHookSync`

The underlying protocol methods remain in the Hook Bridge implementation:

```text
Loom/crates/loom_hook_bridge/src/lib.rs
```

This phase did not invent a fake screenshot backend. It surfaces the existing
Hook/Workflow synchronization protocol path in the UI and keeps the protocol
names visible for debugging.

### Visual style

- Preserved the existing Loom modern-gradient / terminal workbench baseline.
- Added compact pill styling for the Hook screenshot flow strip.

## Contract test update

Updated:

```text
scripts/tests/test-loom-artloom-parity-contract.ps1
```

The contract now asserts Chinese UI copy and explicit Hook screenshot workflow
visibility, while keeping protocol identifiers in English.

New/updated asserted UI surfaces include:

- `总览`
- `Art 注册表`
- `工作流管理`
- `Hook 桥接`
- `工作流工作台`
- `MCP 市场`
- `MCP 包兼容`
- `添加 Art`
- `高级端口编辑`
- `Art 节点`
- `截图同步到工作流`
- `Hook 截图`
- `同步截图上下文`
- `生成/刷新工作流`
- `广播同步`
- `通用设置`
- `引擎设置`
- `快捷键设置`
- `系统设置`

## Verification before release

Commands run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\tests\test-loom-artloom-parity-contract.ps1
npm run typecheck --prefix Loom/apps/desktop
npm run build --prefix Loom/apps/desktop
git diff --check
```

Observed results:

- Parity contract passed.
- Desktop TypeScript typecheck passed.
- Desktop Rsbuild build passed.
- `git diff --check` passed.

Browser UI smoke:

- Built desktop UI opened at `http://127.0.0.1:1427/` through a temporary Node
  static server.
- Navigation snapshot confirmed Chinese labels:
  - `总览`
  - `Art 注册表`
  - `工作流管理`
  - `Hook 桥接`
  - `工作流工作台`
  - `设置`
- Hook Bridge snapshot confirmed:
  - `截图同步到工作流`
  - `Hook 截图`
  - `Hook 截图 -> Loom 工作流 -> Art 节点`
  - `同步截图上下文`
  - `生成/刷新工作流`
  - `广播同步`
  - `art_hook/instantiate`
  - `art_loom/update_workflow_node`
  - `art_loom/workflow_updated`

The browser console showed expected `Failed to fetch` errors because the
temporary UI smoke opened the static desktop bundle without a live
`loom-daemon` on `127.0.0.1:8765`.

## Release

Generated release:

```text
C:\Users\Public\nas_home\AI\GameEditor\Neuro\release\Loom\loom-cn-ui-hook-sync-phase35
```

Executables:

```text
loom.exe
loom-daemon.exe
loom-desktop.exe
```

Zip package:

```text
C:\Users\Public\nas_home\AI\GameEditor\Neuro\release\Loom\loom-cn-ui-hook-sync-phase35\packages\Loom-loom-cn-ui-hook-sync-phase35-windows-x64.zip
```

Zip sha256:

```text
8e7d6de9e2410caea817a79fbf534b738624356407b8cbb09e715d02e46c1544
```

Release smoke:

```text
C:\Users\Public\nas_home\AI\GameEditor\Neuro\output\smoke\runs\20260613-082554-Loom-59580-35d366946e954c6aa123c3b9da6e4348\release-local-apps-loom-cn-ui-hook-sync-phase35-Loom-summary.json
C:\Users\Public\nas_home\AI\GameEditor\Neuro\output\smoke\latest\release-local-apps-loom-cn-ui-hook-sync-phase35-Loom-summary.json
```

Release smoke evidence included:

- `desktopExe` points to packaged `loom-desktop.exe`.
- `hookBridgeMethods` contains:
  - `read_arthook_session`
  - `art_loom/update_workflow_node`
  - `art_loom/instantiate_workflow`
  - `art_loom/workflow_updated`
  - `art_hook/instantiate`
- `artHookSession.method = "read_arthook_session"`.
- `artHookSession.available = true`.
- `websocketBroadcast.method = "art_hook/instantiate"`.
- `websocketBroadcast.workflowId = "wf-release-broadcast"`.
- `websocketBroadcast.nodeId = "release-node"`.
- `mcpMarketplace.packageCheckCommand = "check_mcp_package_installed"`.
- `mcpMarketplace.packageInstallCommand = "install_mcp_package"`.
- `artLoomRegistryCompat.listCommand = "list_arts"`.
- `artLoomRegistryCompat.ipcCommand = "get_ipc_status"`.
- `sharedMemoryCompat.createCommand = "shm_create_buffer"`.
- `sharedMemoryCompat.format = "rgba8"`.

Formal release verification:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-release.ps1 -VersionId loom-cn-ui-hook-sync-phase35 -Apps Loom
```

Observed result:

```text
status: passed
gitHead: 9469af12625aabbbd351b3b8035abb26f96da90d
gitDirty: false
checksumEntries: 31
```

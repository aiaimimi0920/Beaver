# Publication feedback evidence integrity

Date: 2026-09-26

## Delivered workflow

Publication now revalidates archived numbered-frame feedback before producing a
preview, starting a publication journal, applying pending writes, and committing
acceptance. It resolves the feedback's original attempt, verifies the same run
and object, and uses the existing immutable-frame and historical correspondence
resolvers. Missing archives, damaged PNG/frame records, and mismatched sources
block publication with `OBJECT_PUBLICATION_FEEDBACK_EVIDENCE`, the feedback
request ID, and the underlying failure.

Restoring the original evidence allows a fresh preview or an exact retry with
the original publication request. No replacement frame or new request is created
automatically. A failure after the journal starts leaves an `applying` operation
with a durable error; listing and aborting it remain available after reopening
the project. Completed publication replay remains readable even if an archive
later disappears.

The existing Desktop publication APIs and production `ObjectPublicationPanel`
consume this Core behavior without schema changes. The panel already renders
preview errors and operation errors and offers refresh, retry and explicit abort.
The live evidence check belongs to `prepare::load`; frozen journal integrity
continues to use `prepare::build` without requiring live archive availability.

## Verification

Before the fix, regression tests reproduced both an accepted preview with damaged
evidence and publication completion with a missing source archive. The final
focused batch passed **13 Core tests**, including two new workflow regressions
and 11 adjacent deferred-feedback, frame and relocation tests:

```powershell
rtk proxy cargo test --locked -p beaver-core object_publication_deferred -- --nocapture
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy git diff --check
```

The new regressions cover corruption, foreign source metadata and missing records
for both historical and target frames; rejection before journal creation with
unchanged task state; restored evidence and project reopen; archive loss at the
pre-commit checkpoint; pending retry after reopen; abort while evidence is still
missing; restored exact-request publication; and terminal replay after later loss.

Rust compilation and formatting, effective-line and diff checks passed. Logs:
`output/f9-publication-evidence/{red,core,fmt,structure,diff}.log`. The initial red
batch intentionally failed; only `core.log` records the final passing batch.
Production UI error and refresh paths were inspected; no frontend source changed.
This batch did not run browser, Desktop compilation, native EXE, real engine or
external model acceptance. An attempted independent read-only review failed due
to the agent provider returning HTTP 503; it supplies no review evidence.

## Remaining plan boundary

This checks integrity of the original feedback evidence and its saved historical
correspondence. It does not re-localize old regions onto the final candidate or
require a new final-candidate mapping. Standalone PNG and published-version
correspondence, final publication re-localization, cross-window coordination and
the remaining release conditions stay open. F8.5, F9.2 and F9 remain unchecked.

## 2026-10-01 增量：准确最终候选的发布授权

上节保留 2026-09-26 的历史批次边界。后续跨窗口协调已有[独立证据](FRAMEWORK-F9-PUBLICATION-WINDOWS.md)；本次进一步接通最终候选逐区确认，不再仅验证历史反馈检查点。完整流程和验证记录见 [F8.5 当前增量](FRAMEWORK-F8-ATTEMPT-RELOCATION.md#2026-10-01-增量最终候选逐区重新确认)。

发布预览为原编号反馈提供来源摘要及区域数，生产面板展示原存档与准确最终 output 编号帧，要求每个原区域对应或说明无对应原因，并重新明确确认。已发布编号反馈的后续任务及自动延后任务继承准确原始来源，不用新轮次或历史检查点映射代替最终授权。Core 在新建、原请求重试及提交前验证该确认；冻结日志保留完整请求，损坏/缺失/异源/过期帧阻止接受且不静默替换来源或目标。

未提交草稿重开恢复映射与说明，但清除最终区域确认和文件/替换授权；已经发送的发布意图只读协调，用户明确重试完整原请求。完成回执和旧 schema 1 日志保持既有安全回放语义，未完成日志仍可查询和中止。

实际浏览器验证同时修复属性顺序误拒：草稿与有效回执先经严格 schema 规范化，再比较完整值。取消确认、修改来源/目标/区域或缺少回执仍即时禁用发布，不通过放宽门禁获得正向成功。

### 有效验证

- Core 发布 **37 项**、Desktop 发布 **2 项**及相邻共享夹具消费者 **1 + 1 项**通过。
- 前端初次定向批次 **45 项**通过；门禁修复后受影响批次 **21 项**通过，存在重叠，不相加。
- TypeScript、定向 Rustfmt、Prettier、有效行及差异检查通过；有效行 **1224 sources、17 unchanged legacy files、0 violations**，未更新 baseline。
- Chromium 生产组件证明合法确认启用发布、重开不恢复授权、响应丢失后显式重试 JSON 完全相同的原请求，以及完整保存草稿遇损坏 PNG 仍发送 0 次。图片恢复后原映射可用，但授权仍未确认。

日志和截图位于 `C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-final-relocation-20261001`，浏览器汇总为 [proof.json](../../AI/GameEditor/linshi/beaver-final-relocation-20261001/browser/proof.json)。浏览器使用真实存储及 Web Locks、模拟 API 和 canvas PNG；早期证明脚本预期错误保留，不作为产品通过证据。

### 当前剩余边界

源码闭环不等于旧 `.42` 宿主中真实候选已被批准。该原生轮次仍 `awaitingAcceptance` / `awaitingGate`、0 publications；本次没有原生构建、真实候选接受/发布、native WebView 或外部模型/引擎整体验收。独立 PNG、完整三维旧拓扑和其余接入/正式发布条件仍待完成。F7、F8.5、F8、F9.2 和 F9 不因本次有界结果整体勾选。

## 2026-10-01 增量：独立冻结 PNG 发布证据

本增量补齐上一个批次尚未覆盖的独立冻结 PNG 来源。生产发布面板显示原图编号与局部上下文，原图成功读取、解码并核对身份和真实尺寸后，才允许将每个原区域对应准确最终 output 的编号区域，或填写无对应原因并重新确认。详细实现、日志和边界见 [F8.5 独立冻结 PNG 增量](FRAMEWORK-F8-ATTEMPT-RELOCATION.md#2026-10-01-增量独立冻结-png-最终确认)。

发布来源摘要绑定完整原反馈和原 PNG SHA-256。Core 复用事务中已读取的权威 attempt 与既有冻结文件安全读取，校验项目、原轮次、对象、尝试、路径、哈希、完整 PNG 解码和尺寸；继承反馈不改写原轮次。新建日志、pending 原请求重试及最终提交边界重新验证来源，失效时拒绝发布；损坏证据不妨碍未完成操作的查询和中止，终态 replay 不新增 live PNG 依赖。journal schema、`prepare::build` 和冻结日志 integrity 保持不变。

草稿保留映射与说明但不恢复授权。原图 missing、decode、dimensions 或 identity 故障均阻止对应、最终确认和发布；显式重读相同原请求后仍需新确认。已经发送的发布请求继续由既有恢复路径管理，响应丢失后不自动 mutation，用户显式 retry 重复完整原 JSON，包含 finalRelocation。

### 有效验证与交付边界

- Core publication **40 项**、attempt-file **3 项**、相邻返工图片与交付 **5 项**、Desktop publication **2 项**通过；批次有重叠，不相加。`core-rework-image.log` 的 0-test 过滤器不计覆盖，正确结果为 `core-rework-image-verified.log`。
- 前端定向 **52 项**通过，TypeScript、定向 Rustfmt、Prettier 和有效行检查通过；**1229 sources、17 unchanged legacy files、0 violations**，没有更新 baseline。
- Chromium 三组生产组件证明通过，四类来源故障发送 0 次且保留完整草稿；响应丢失后只允许显式重试完全相同的原请求。API 为模拟响应、图像为 canvas PNG，不代表原生或真实发布验收。

证据位于 `C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-final-relocation-png-20261001`：[应用汇总](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/gate-proof.json)、[浏览器汇总](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/browser/proof.json)、[资源关闭证明](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/browser/cleanup-proof.json)。有效应用结果复用且未被后续生产修改失效；收尾只纠正遗漏的相邻测试选择并验证文档，不重复整体验收。

独立 PNG 的源码工作流已补齐；完整三维旧拓扑、native WebView、真实引擎最终发布和其余正式交付条件仍未完成。本轮没有新 EXE、版本变化、外部模型调用、真实候选接受/发布、提交或推送，也未查询真实宿主当前状态；上文原生候选状态保留为历史记录。F7、F8.5、F8、F9.2 和 F9 保持未勾选。

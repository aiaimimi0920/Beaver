# 两阶段资产交付原生验证

日期：2026-09-14。范围：最小 Beaver -> Codex -> Beaver 两阶段资产交付、审批暂停、宿主重启恢复和最终合入。只运行相关功能验证，没有运行完整 NPR 制作、游戏验收或正式发布流程。

当前状态：本批最小原生闭环已完成。首轮证明真实回调、冻结文件、审批恢复和 Blender 制作，并暴露纯资产错误触发 GUT 的问题；修正验证范围后，4 项相关回归通过。新空白项目完成设计批准、模型制作、退回重提、模型批准和最终合入，交付文件哈希一致。复测包含一次操作者要求整理报告目录的恢复，不能记为首次提交自动直通。整体工作流框架仍未完成，进度见 [开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。

## 首轮事实与恢复证据

使用 `scripts/start-fresh-native-test.ps1 -Build` 编译并启动 `target/release/Beaver.exe`，内部版本 `0.1.19.1`。未替换 `release` 内的用户发布包。构建与 Rust 测试均在保留 MSVC `LIB` 的正常 PowerShell 环境运行。

首轮目录：`output/validation/fresh-round-20260914-023855-c52a0896d8fa4d15a28a9286467ab463`。其 `start-proof.json` 记录空数据、独立 WebView、唯一选定宿主、可执行文件 SHA-256 和通过 Beaver API 创建的空白 Godot 项目。初始项目仅含 `export_presets.cfg`、`main.tscn` 和 `project.godot`。

任务 ID：`1ab1413e-0c1d-4c63-a216-38b44df85fba`。输入为“做一个红色小球 Blender 资产，分两步交付：先给出简短设计说明，我批准后再制作模型并导出 GLB。每步提交可查看的文件，等我批准后继续。”设置 `assetTask: true`、`autoAccept: false`、`maxMinutes: 20`、`askRatio: 0`。操作者仅通过 Beaver API 设置工具、发起任务、读取候选、导出文件及作出测试审批；没有直接调用 Codex、Blender 或 Godot 帮应用制作产物。

- 第一阶段候选 `aceea507-0fb5-430a-96eb-08897ea59bf4`：设计说明、评审约定及模型自行生成的 `beaver.validation.json`。任务停在 `awaitingInput`，revision 为 2。
- 待审批时关闭宿主并在同一轮数据目录重启，PID 从 6236 变为 45468。关闭窗口仅隐藏，随后校验可执行文件路径并终止确切 PID。`restart-proof.json` 记录重启；API 复核候选元数据和三份冻结文件逐字节一致。
- 第一阶段批准后恢复同一个 Codex thread，以新 turn 制作模型。第二阶段候选 `5e9532da-76d3-4aaf-b8a1-38f5303a2426` 绑定第一候选作为输入；提交 `.blend`、GLB、PNG 预览和两份记录后再次暂停。
- 从候选 API 读取并查看真实 PNG，画面为深色背景上的红色球体。模型自己的验证记录报告 20 cm、2208 个三角形、单网格和材质，并记录 Blender 导出及 GLB 重导入检查。这些是受管理制作过程的报告，Beaver 尚未独立验证其模型语义。
- 第二阶段批准后原样重复同一审批请求，返回完全相同结果，决定总数仍为 2。随后两个阶段均已批准、待审批指针为空，managed `finishRound` 返回 ready。

工具版本：Codex CLI 0.153.4、Blender 5.2.1 LTS、Node v22.22.2。已配置 Godot 的目录名为 `godot-4-4-1`，其实际探测版本为 `4.5.2.rc.custom_build.2890667c8`。不能用目录名替代真实版本。第二阶段报告 glTF 导出器 5.2.40；本轮未验证第三方插件自动安装生命周期。

## 发现的问题与定向修复

`validation/task_gate.rs::required` 原先把所有非文档文件变化都交给 GUT。纯 `.blend`、GLB 和 PNG 交付因此启动 Godot 验证，并触发超时后的自动修复。首个失败 run 为 `1376cdc6-1c68-4592-99a1-26944833fb5b`，错误为“工具执行超时，已中止本次进程”，没有完成的测试步骤。模型生成的 manifest 使用空 `code.directories`；现有 GUT runner 不接受空套件，因此不能以“空测试通过”解决该问题。

修复仅调整代码验证选择：显式资产任务中，新增或修改 `.blend`、`.glb`、`.png` 以及新增的根级 `beaver.validation.json` 可与文档一起免于 GUT。删除资产、修改已有验收清单、未知文件、脚本、场景、资源和项目配置仍触发代码验证；显式 `integrationValidation` 始终保留。原有阶段 ready、审批、冻结哈希、结构和合并冲突检查继续执行。该边界没有为资产质量签发自动通过回执。

测试新增到 `native/core/src/validation/task_gate_scope_tests.rs`：验证纯资产真实 finalizer 合入且不创建 GUT 或覆盖任务，并检查混合代码、配置、删除及集成验收仍需 GUT。原有资产反馈与迟到代码变更回归同时通过。

首轮曾观察到另一个 Beaver 和一个 Godot 进程，进一步核验时它们及其父进程已退出，无法恢复精确参数。仅剩 PID 45468 与重启证明吻合。没有足够证据确认这两个短生命周期进程的来源，也未查明 Godot 超时的底层原因；本次修复解决纯资产被错误选入 GUT 的边界，不宣称修复 Godot 执行器超时。

## 修复后新轮结果

新轮目录：[fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/)。[启动证明](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/start-proof.json) 记录旧宿主 PID 45468 关闭、新宿主 PID 50512、独立数据目录和空白项目。项目 ID 为 `7479c6ed-277a-483e-bf9f-a70b7dbecc07`，任务 ID 为 `3cb21d5e-9085-4226-846a-b5501f2bea22`，使用与首轮相同的简短制作要求及任务设置。

1. 设计候选 `dd92ee65-c595-48ce-b70c-94bdd8165754` 提交两份文件，revision 2 时暂停。通过 owner API 批准后，revision 3，同一 Codex thread 以新 turn 继续制作。
2. 模型候选 `73fd60f3-d36d-46e9-b130-1f32888b22e5` 提交 Blender、GLB、PNG 和报告，revision 5 时再次暂停，上游输入绑定设计候选。操作者通过候选 API 读取文件、核对哈希并查看实际预览。
3. 候选中的 `assets/red_ball/validation.json` 是实测报告，但不属于当前免 GUT 的路径范围。操作者在最终批准前通过 owner API 退回，要求将说明和报告统一放到 `docs`，保持模型及预览不变。这是本轮的人工恢复操作；未放宽任意 JSON 的代码验证规则，也未直接修改测试项目文件。
4. Codex 将报告移到 `docs/production/red-ball-validation-data.json` 并更新链接，提交新候选 `d9679bf0-2d05-4f71-9121-5ca477dca7f1`，revision 7。模型、GLB、PNG 三份文件哈希与被退回候选完全相同。旧候选及退回原因保留。
5. 批准新候选返回 revision 8。原样重复审批返回同一结果，决定总数保持 3（两次批准、一次退回）。随后 `finishRound` 完成，revision 9，两阶段均已批准且无待审批候选。
6. 最终任务 `status: completed`，`codeValidation.status: notRequired`，无 GUT 事件。最终合入的 9 个文件与设计及模型批准清单逐一核对 SHA-256 一致；初始三个 Godot 项目文件哈希不变，被退回路径的 JSON 未进入项目。由于 `autoAccept: false`，合入完成后另通过 `task.accept` 确认任务，最终 `accepted: true`。任务认可与阶段批准分别记录。
7. `assetTask.deliveryExport` 实际导出最终模型候选的 7 个文件，磁盘字节逐一匹配候选哈希。[API 状态、事件、审批和独立检查记录](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/native-proof.json) 保留完整过程。

交付文件位于 API 创建的[导出目录](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/data/delivery-exports/candidate-8Y9cjw/)：

- [Blender 源文件](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/data/delivery-exports/candidate-8Y9cjw/assets/red_ball/red_ball.blend)：121827 字节，SHA-256 `eb3461829ff76fc4e86e80baf93ed7468db2ac85d280e4ab735a9e670cf1e4a6`。
- [GLB 模型](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/data/delivery-exports/candidate-8Y9cjw/assets/red_ball/red_ball.glb)：378936 字节，SHA-256 `bafa468c07b1eeee8b9f69f4a2f5c159fc531dc3522c418d09ae4a5fedeff1ed`。
- [固定预览](../output/validation/fresh-round-20260914-032059-9d109910c6c849a89ee6c2d4af0c17c4/data/delivery-exports/candidate-8Y9cjw/assets/red_ball/red_ball_preview.png)：768 x 768，红色球体和深色背景；SHA-256 `629f90cfd18242ce4aef5e3f2ac3c7220b5214013fdf084c8d98dae034b3c57b`。

操作者独立解析冻结 GLB 的文件头和 JSON 块：GLB 2.0、单节点 / 网格 / 材质、无相机和动画、无外部 buffer、三轴尺寸约 0.20 米、15872 个三角形，金属度 0、粗糙度约 0.35。基础网格、修改器、法线和 Blender 场景对应关系仍来自受管理制作过程的报告；没有独立重新执行 Blender 验证。固定 PNG 已查看，但 Beaver 尚未自动证明它与 `.blend` 的视觉对应关系。

制作过程中，空场景缺少 World 导致预览设置中断，Codex 检查已有对象后补建 World 并继续；反馈专用 `execute` 因缺少反馈 ID 被拒绝，首次制作使用成功的检查点继续。报告还明确当前无可用通用 GLB 标准导出和包验证入口。这些都保留为工具约定及后续语义验收的缺口，未由操作者在应用外补做模型。

## 验证命令与材料

- `rtk proxy cargo test --locked -p beaver-core validation::task_gate --lib`：4 项通过，包含 2 项新增范围回归和 2 项原有资产代码验证回归；[日志](../output/asset-delivery-native/gate-tests.log)。
- 首轮原生构建成功：[日志](../output/asset-delivery-native/fresh-build.log)。PowerShell 重定向日志为 UTF-16LE；Cargo 进度被显示为 stderr 记录，实际构建退出成功。
- 首轮完整 API 状态、候选、事件及测试审批快照：[native-evidence.json](../output/asset-delivery-native/native-evidence.json)。其中保留失败现场，不把首轮记为最终合入成功。
- 修复后 `scripts/start-fresh-native-test.ps1 -Executable .\target\release\Beaver.exe -Port 4337 -Build` 成功完成原生构建和新轮启动：[日志](../output/asset-delivery-native/fixed-build.log)。构建中的 `check:effective-lines` 通过：476 个源文件、18 个未修改历史超限文件、0 个违规，未更新基线。
- `rtk proxy cargo fmt --all -- --check`、本批三份 Markdown 的 Prettier 检查和 `git diff --check` 通过；本批文件严格 UTF-8 解码、无 BOM，本地文档链接有效。文档更新未改变源码，未重复运行已通过的 Rust 功能测试。

当前编译产物为 [target/release/Beaver.exe](../target/release/Beaver.exe)，内部版本 `0.1.19.1`，SHA-256 `2480C848ECC94B7265C0AEA8682DBE0B8EDDD843B32D29EE258C728AAB5DCDE8`。没有替换用户发布目录中的包。

宿主重启证明仅覆盖待审批状态、候选文件和恢复后继续制作。它没有证明未保存 Blender 场景自动恢复、相对操作恰好执行一次、完整原生 UI 联动或人物建模质量。资产目录中的普通 JSON 报告仍触发代码验证，当前需放到 `docs`；产物类型与检查器的精细对应尚未实现。后续框架工作仍包含资产子任务、统一 attempt、源依赖、插件生命周期及语义和视觉验收。

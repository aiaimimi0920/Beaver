# F3.4 生产任务泳道证据

日期：2026-09-25。范围：创作 → 对象任务中的全层级状态泳道、责任树和迭代执行记录入口。

## 可用结果

生产页读取现有持久化快照，将全部粗修、中修、精修按等待开始、执行中、需要关注、已接受、已撤销分列。卡片展示真实任务身份、对象与迭代、需求和依赖；父子关系可逐级展开，独立中修单独标明。失败与撤销保持区分，责任父子关系不从依赖推断。

中修和精修均可打开现有执行/恢复对话框；精修入口绑定其实际父中修。原有派发、修订、取消操作保留在管理任务折叠区。项目 key、查询更新和操作禁用边界复用原有生产面板。

中修展示“已接受精修数 / 已规划精修数（当前计划）”，撤销项不算已接受；无精修时显示进度未知。父任务继续展示自身持久化状态，不因子项已接受而自动显示完成。未引入模拟任务、固定百分比、新存储或 Desktop API。

实现：src/ui/object-tasks/ObjectTaskBoard.tsx、object-task-board.css、ObjectTasksPanel.tsx；回归：tests/object-task-board.test.ts。

## 验证

- 新增泳道回归 4 项通过，覆盖所有状态、责任树与依赖区分、精修历史归属和接受事实边界；日志 output/f3-board-tests-final.log。
- 相邻导航、执行查询和执行界面 15 项通过，日志 output/f3-board-tests.log。该初轮日志同时保留新增测试因正则转义错误而加载失败的记录；修复后单独复验新增 4 项，未重复运行已通过的相邻测试。
- npm run typecheck 通过，日志 output/f3-board-typecheck-final.log。
- 四个源码、样式及测试文件 Prettier --check 通过，日志 output/f3-board-format-final.log。
- npm run check:effective-lines 通过：1041 sources、17 unchanged legacy files、0 violations，日志 output/f3-board-structure-final.log；保留既有 unused-field 警告。
- git diff --check 通过；本轮文件 UTF-8 无 BOM。

测试验证组件渲染、回调和真实执行 session 归属；生产父层接线经过源码核查。未运行浏览器视觉或原生交互验收，未修改 Rust 行为、构建或发布新程序。

## 2026-09-25 层级进度补充

粗修卡片现在沿责任父子关系汇总全部所属精修；中修复用同一投影。两层均展示已接受与已规划计数，以及等待开始、执行中、待验收、失败、撤销数量。依赖任务和其他独立迭代不会混入汇总。取消保留在已规划分母中，不记为接受；汇总不会改变持久化状态。

没有中修的粗修、没有精修的中修会列入“尚未细化子任务”，即使其他分支已经接受，也明确提示整体进度未知。所有卡片说明当前计数不能替代父任务验收。本轮闭合生产看板中的层级事实查看流程，复用既有快照查询、订阅和 Desktop/Core 数据，无需新增存储或 API。

验证：看板 6 项测试通过（output/f3-progress-tests.log），覆盖跨层汇总、独立任务排除、部分未规划、全部撤销、空粗修和快照不变；npm run typecheck 通过（output/f3-progress-types.log）；有效行检查通过且未更新基线（output/f3-progress-lines.log）。三个源码/测试文件及两份文档 Prettier 检查通过，git diff --check 通过。未运行原生视觉验收或构建发布程序。

本批当时尚未记录必要/可选工作分类或规划完成声明，因此未输出完成率。必要/可选分类由下方后续切片补齐。

## 2026-09-25 必要与可选工作分类

草稿编辑器可为粗修、中修、精修选择必要或可选工作，保存、提交和重新打开项目后保留。已提交且未启动的任务可通过现有定义修订入口变更分类；复用原因、revision 冲突检查、不可变历史和幂等回执。比较与历史界面显示分类变化。已启动任务仍受原有修订限制。

看板显示每项分类，并沿责任树汇总必要精修和必要中修。粗修下可选中修的后代不计入该粗修的必要工作；进入该中修时仍按其自身子任务分类展示本地范围。撤销必要项会提示修订目标与验收、记录决定；空必要范围不代表完成。分类不改变显式依赖、阶段检查或发布门槛，也不自动接受父任务。

旧 proposal、任务和定义缺省为 required；Rust 序列化省略默认字段，保持旧冻结定义 JSON。optional 显式持久化在原有 JSON 实体中，无 SQL 表迁移。旧版严格读取器不能保证读取新增 optional 字段，未宣称支持旧程序读取新可选数据；回退程序前需使用升级前的数据备份。

实现覆盖 object_task_types.rs、object_task_definition.rs、object_task_commit_records.rs、共享 schema、草稿/修订编辑与比较、看板进度。新增回归覆盖旧数据默认、提交冲突、存储重开、分类修订幂等、真实组件回调保存及重开、修订请求与可选父分支计数。

验证日志统一为 output/f3-requirement-*.log：UI/看板/预览 12 项、相邻修订/工作区/插入 33 项通过；Core 分类 3 项、定义 9 项、提交 4 项及队列 7 项通过，队列覆盖可选依赖仍须满足。TypeScript 检查与 Desktop 编译检查通过。有效行检查通过且未修改基线。初次 Core 过滤器误用了文件名并运行 0 项，随后已纠正为模块路径，以上数量来自实际执行记录。

最终检查：1070 个源码文件、17 个未改动历史超限文件、0 个违规；定向 Prettier、rustfmt 和 git diff --check 通过。本轮 24 个源码/测试/文档文件均为有效 UTF-8 且无 BOM。保留既有 Rust unused/dead-code 警告。

F3.4 保持未勾选：规划完成声明、非空计划未登记需求的表达和正式原生交互验收尚未闭合。本轮不输出精确百分比，未构建发布程序或运行全量原生验收。

## 2026-09-25 Desktop 规划字段集成

Desktop 工具发现契约现已在 saveDraft 的任务和 revisePlanned 的定义中公布 requirement 与 pendingPlanning，保留旧请求的可选字段默认值和严格对象约束。Desktop 请求校验只检查顶层，嵌套数据由 Core 反序列化；本次修复的是对外公布的 schema 缺失，直接 UI 请求此前已能传入字段。字符串 schema 限制字符数，描述明确 Core 另行限制 4,000 UTF-8 字节及精修归属。

revisePlanned 成功后现会触发原有业务变更通知，供生产查询订阅刷新；不唤醒调度器。新增真实 Desktop dispatcher 回归贯通草稿保存、提交、定义修订、关闭并重开项目、历史查询与幂等重试，确认项目数据不写入 host 存储。非法分类、精修待规划事项、Unicode 字节超限和错误字段类型被拒绝，失败不留下草稿或改变任务快照。

验证：Desktop 规划字段 3 项、相邻定义修订 3 项、业务副作用 1 项通过，日志 output/f3-metadata-desktop.log、f3-metadata-revisions.log、f3-metadata-effects.log。有效行检查通过：1074 sources、17 unchanged legacy files、0 violations，日志 output/f3-metadata-lines.log；未更新基线。定向 rustfmt、文档 Prettier 和 git diff --check 通过；本批文件为有效 UTF-8 且无 BOM。未重复前批已通过的 UI/Core 测试，未运行正式原生交互或发布验收。规划完成声明仍待实现。

## 2026-09-25 显式待规划事项

后续切片增加 pendingPlanning：粗修和中修可在草稿填写尚未展开的要求或阶段，保存、提交、重开项目均保留。已提交未启动项复用定义修订及原因、前后比较、不可变历史和 request ID 幂等；重复提交同一任务 ID 不得覆盖事项。精修不接受该父任务字段，内容上限为 4,000 UTF-8 字节；草稿存在待规划事项时禁用切换精修，避免隐藏仍未处理的数据。

看板沿责任树展示本任务和后代的已登记待规划事项。即使当前已规划精修全部接受，也明确显示整体进度未知。清空字段表示没有已登记事项，不表示规划完整或目标完成。旧数据默认空值，Rust 序列化省略空字段，继续沿用旧冻结定义 JSON；非空字段仍需新读取器，未提供旧程序读取新字段的保证。

本轮闭合人工记录规划缺口的保存、提交、修订和查看流程，不改变队列领取、阶段接受及发布门槛。运行中补充要求的安全切换、明确的规划完成声明及完整原生交互验收仍待后续实现；F3.4 保持未勾选。

验证：47 项前端回归通过（output/f3-planning-ui.log）；Core 规划缺口 2 项、兼容性/分类 3 项、定义修订 9 项通过（output/f3-planning-core.log、f3-planning-compatibility.log、f3-planning-definitions.log）。覆盖保存提交重开、重复 ID 冲突、历史和重开重试、精修错误归属、超长输入，以及所有已知精修接受后仍显示未规划内容。TypeScript、Desktop 编译、定向 Prettier/rustfmt、有效行检查和 git diff --check 均通过；未更新结构基线，本轮 22 个文件为有效 UTF-8 且无 BOM。日志保留在 output/f3-planning-*.log。未运行正式原生或全量端到端验收。

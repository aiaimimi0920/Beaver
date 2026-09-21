# F1：项目存储与迁移实施记录

## 2026-09-21：最终派生副本组装与目标恢复

新增 `project_derivation_assembly::create/inspect`，从核验过的准备副本在全新外部目录组装 `project`。使用正式项目初始化器生成存储元数据、schema、身份及 WAL 配置；组合转换后的实体、事件和调用并保留调用自增水位。按新身份复制文件，合并转换后的 Codex SQLite 索引且排除旧 WAL/SHM；JSONL 和非声明式内容保持原文。验证请求保存在独立来源归档中。

恢复前先把项目实体 path 绑定到最终目录，随后仅在目标执行 Journal、任务及 Validation 恢复；未解决的文件日志阻止完成。关闭数据库和文件句柄后核对最终清单，并再次检查准备来源，最后写入 `ASSEMBLY.json`。副本保留 pending，普通项目打开仍被拒绝，尚未登记或启用。准备目录可移动、源可离线；组装结果绑定最终绝对路径，移动后拒绝核验，须在新目标重新生成。失败目录保留，同名重试不覆盖。

派生定向测试共 41 项通过，新增组装回归覆盖真实日志应用、排队任务中断、父任务暂停、验证证据保留、调用水位、会话绑定、源和准备副本字节保持、离线重试、损坏拒绝、移动拒绝及日志失败保留 pending。证据为 `output/f1-derivation-assembly-tests.log`。可恢复启用、登记冲突处理及派生 API/UI 尚未实现；F1 保持进行中，F2-F9 待实施。本批未构建发布包或变更版本。

## 2026-09-21：Codex 会话索引离线转换

新增 `project_derivation_sessions`，从验证过的准备副本生成独立的 `session-stage-{generation}`。识别任务 Codex HOME 下的 `state_<数字>.sqlite`，复制主库、WAL 和 SHM 到临时目录，在关闭 trusted schema 并完成 quick_check 后，仅转换 threads 的 rollout_path/cwd、project_roots.path 和 skipped rollout 路径；thread ID、未知列/表及 JSONL 原文保留。拒绝缺失或不支持的表结构、触发器、越界路径、跨任务 HOME、错误文件类型和缺失目标。

索引写入显式指定的最终项目绝对路径，回执记录该 binding。准备目录移动和原项目离线后仍可核验；最终目录变化时拒绝旧 binding，须创建新 generation。转换不写最终目录，不覆盖文件暂存中的原始 SQLite，不移除 pending；失败代次保留。后续组装最终项目时必须采用转换后的索引且排除对应旧 WAL/SHM，完成最终绑定、活动状态恢复和启用后才能交付。

回归覆盖仍在 WAL 中的提交、源和准备副本字节保持、未知 BLOB/表及 JSONL 原文、离线恢复、目标重绑定、schema/触发器拒绝、失败保留和损坏检查。首次路径穿越用例暴露 Windows verbatim PathBuf::join 会预先规范化测试输入，已改用原始字符串保证真正测试拒绝 `..` 的边界。

## 2026-09-21：暂存数据库声明式路径转换

已将 `project_derivation_record_paths` 接入 verified Snapshot 的数据库暂存组合，在身份转换前按源身份查找准备清单，核对文件存在、类型、规范路径以及参考图/验收证据摘要，然后转换任务工作区、资产 checkpoint、参考图及框架 context checkpoint。任务反馈种子、操作日志 taskAfter、资产反馈和尝试中的路径均覆盖；验收证据文件名保留，父运行目录通过文件暂存转换。callback 原始请求/响应、任意输出和项目资源引用不作内容替换。

持久化暂存格式升为 `beaver-project-derivation-stage-v2`，旧 v1 代次保留，使用新 generation 重新生成。此暂存仍缺最终项目元数据及活动状态恢复，不能直接启用；源项目、准备副本和 pending 不变。项目自身 path、Codex SQLite 索引与最终清单留给后续绑定阶段。

回归覆盖源离线和准备目录搬移后，真实 Files 解析器读取暂存数据库指向的工作区、checkpoint、参考图；另验证错误类型、缺失文件、路径逃逸、错误摘要及跨项目引用拒绝，并保留非声明式输出。证据见 `output/f1-derivation-record-paths-tests.log` 与 `output/f1-derivation-record-paths-checks.log`。F1 保持进行中，F2-F9 待实施。

## 2026-09-21：后续任务反馈种子的身份转换

核对路径契约时发现 `assetFeedbackSeed` 是完整 Feedback，原任务转换漏掉其中的任务、项目和 reference 身份。现复用资产反馈转换器，先核验原 fingerprint，再转换声明身份并重算 fingerprint；操作日志中的 `taskAfter` 同步采用相同转换。反馈局部 ID、正文、历史、frame 和待处理路径保持原值。

新增回归调用真实 `asset_task::initial` 与 `asset_feedback::duplicate`，验证派生后后续任务初始化、反馈重放、日志任务一致性、源记录保持，以及损坏 fingerprint 和缺失任务映射拒绝。32 项派生测试通过，证据见 `output/f1-derivation-seed-tests.log`；格式与结构检查见 `output/f1-derivation-seed-format.log` 和 `output/f1-derivation-seed-structure.log`。数据库路径及最终启用仍待实施，F1 保持进行中。

## 2026-09-21：派生文件目录转换与暂存

新增 `project_derivation_files::create/inspect` 和路径映射模块，在全新 `files-stage-{generation}/project` 中按准备回执的新身份复制任务工作区、Codex HOME、验收运行证据目录及观察参考图文件名。其他路径和所有文件正文逐字节保留，目标清单要求与来源的文件类型、长度及摘要完全一致；缺失身份映射、路径逃逸和目标大小写冲突拒绝。转换只处理已声明的目录层级，不递归替换工作副本、JSONL 或历史报告正文中的 ID。

每个代次自带 pending 标记，完成回执最后写入，包含准备摘要及转换后的完整文件清单。检查重新计算预期映射，核对文件覆盖、摘要和准备来源；失败代次保留，同名重试不覆盖。源离线和准备目录移动不影响恢复检查，单独移动整个代次后仍被 pending 阻止打开。

新增 3 项回归使用真实项目准备和 Files 路径解析器验证工作区、checkpoint、参考图及验收证据路径，并覆盖 JSONL 和未知文件保留、源字节保持、不完整/损坏拒绝、跨准备回执拒绝及孤立任务目录拒绝。31 项派生定向测试通过，证据为 `output/f1-derivation-files-tests.log`；Rust 格式与有效行检查记录在 `output/f1-derivation-files-format.log` 和 `output/f1-derivation-files-structure.log`。

此阶段只转换物理路径，SQLite 数据库及项目清单仍是原始字节，不能独立启用。数据库内声明式路径字段、Codex 会话索引、活动状态中断、最终项目数据库和清单生成、完整核对、可恢复启用及 API/UI 继续待办；F1 保持进行中，F2-F9 未实施。本批不递增版本或构建发布包。

## 2026-09-21：身份转换暂存持久化

新增 `project_derivation_stage::create/inspect`，从已核验准备副本的隔离 SQLite 快照组合身份转换结果，在全新的 `identity-stage-{generation}` 目录写入 `identity.sqlite` 和验证请求来源归档，逐文件同步后最后写入 `IDENTITY-STAGE.json`。回执保存准备记录摘要及载荷清单，恢复时重新核对来源、文件摘要、归档覆盖、数据库 schema、完整性及目标归属引用。

准备副本、pending 标记和原项目保持不变；原路径离线或准备目录搬移后仍能检查暂存。失败目录保留用于诊断，同名重试拒绝覆盖，使用新 generation 重新生成。回执检查用于发现不一致，不提供对可同时改写全部文件的攻击者的真实性保证，也未证明所有文件系统的断电持久性。

`identity.sqlite` 仅包含实体和历史表，缺少最终项目存储的 `project_identity`、application_id、user_version 及 WAL 配置，不能直接替换 `project.sqlite` 或交给 ProjectStore。文件/路径和会话转换、活动状态中断、完整项目数据库生成、最终核对、可恢复启用及 API/UI 继续待办，F1 保持进行中。

定向回归覆盖持久化后的真实数据库读取、归档保留、源离线和目录搬移、pending 保持、不完整代次恢复、新代次重试、数据库及归档损坏拒绝、路径逃逸拒绝和跨准备回执混用拒绝。验证证据见 `output/f1-derivation-stage-tests.log` 与 `output/f1-derivation-stage-checks.log`；此批不递增版本或构建发布包。

## 2026-09-21：离线数据库转换组合

新增 `project_derivation_database::compose`，在单个源读取事务内重新生成并精确核对身份映射，统一分派项目、任务、资产、框架与验证实体转换，并在独立内存数据库事务中组合事件和调用历史。返回前再次验证目标归属与声明式引用；错误不返回部分目标，源事务回滚释放。

`validationRequest` 不进入目标重放表，原存储键和原始 JSON 字符串随来源项目、目标项目和请求 ID 一起保存在可序列化 Archive 中，与内存数据库共同返回。空白字符保持，源回执仍可重放，目标同请求不会误用历史结果。此阶段仅形成离线暂存结果，仍保留原路径和活动状态；尚未持久化到准备副本，也不能启用或交给调度器。

定向回归覆盖真实验证回执重放隔离、调用与框架轨迹关联、事件读取、跨实体归属、归档序列化、篡改映射拒绝及后段转换失败后的源保持和修正重试。首次测试暴露资产 reference 夹具缺少 frame，补齐真实记录结构后重跑。验证证据见 `output/f1-derivation-database-tests.log` 与 `output/f1-derivation-database-checks.log`。

下一步继续可恢复暂存持久化、文件/会话转换、活动状态中断、完整核对及启用；F1 保持进行中，未构建发布包或递增版本。

## 2026-09-21：事件与调用历史转换

新增 `project_derivation_history::copy`，在调用方持有的离线事务内复制事件和调用：映射事件任务列、调用 ID 及其任务/项目列和 JSON 归属，核对列与 JSON 一致，保留序号、事件原文和输入/输出摘要。同步保留 calls 的自增序号水位，避免最高记录被保留策略删除后，新调用复用旧游标位置。目标历史必须为空，任何转换错误由调用方回滚整个事务。

真实生产者核对发现 framework trace 的键是 call ID，修正之前将其视为局部 ID 的处理。轨迹键和自身 id 统一使用 calls 的确定性身份算法；调用已被清理时，持久轨迹仍能独立转换。Codex itemId 和摘要原文保持。包含旧轨迹映射的准备目标也需保留，在新目标重新准备。

定向回归使用真实调用生产者、查询游标、事件读取器与框架证据，覆盖任务/项目过滤、轨迹关联、清理后的轨迹、新调用游标、错误回滚和拒绝覆盖目标。证据见 `output/f1-derivation-history-tests.log` 与 `output/f1-derivation-history-checks.log`。

尚未接入准备副本的完整数据库提交；实体分派、验证请求来源归档、文件/会话转换、活动状态中断及启用仍待完成。此批不改变项目可用状态，F1 保持进行中。

## 2026-09-21：框架记录与候选摘要转换

新增 `project_derivation_framework_records::rewrite`，转换配置、操作、轨迹、观察、恢复、检查及当前/历史评价的任务归属。观察记录的存储键与 referenceId 同步转换为全局 asset-reference 新身份；恢复记录同步关联新 framework-operation。callback 操作的原始请求与响应、局部 ID、输入/输出摘要、路径及历史执行身份保持原值。

候选摘要先读取并核验真实 Candidate，再转换 taskId 并调用业务摘要算法重算。运行中检查允许真实的无摘要占位记录；历史配置 revision/hash 保持，确保过期评价继续被 gate 拒绝。回归通过真实 gate、operation reader 和证据生产者验证目标关联、损坏摘要拒绝及源数据保持。

`output/f1-derivation-framework-records-tests.log` 记录 21 项派生定向测试全部通过；格式与结构检查见 `output/f1-derivation-framework-records-checks.log`。观察键映射已修正，包含观察记录的旧准备映射将无法通过精确恢复核对；应保留旧目标，在新目标重新准备，不能静默接受旧映射。

当前仍是内存转换，未提交准备副本数据库。完整实体分派、事件/调用转换与请求归档、文件及会话转换、活动状态中断、最终核对与启用继续待办；F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：callback 回执身份转换

任务记录转换器新增 `task-callback/{task}` 支持，转换命名空间、任务及项目归属，并核对回执、请求和响应的 requestId 一致性；拒绝只读请求伪装为 mutation 回执。原始请求与响应、局部工作 ID 和历史执行身份保持，避免改变真实 callback 的完整 JSON 重试判定。回归由真实 `task_callback::call` 生成源回执，再验证目标 inspect、相同请求重放、内容及执行身份冲突拒绝和源回执保持。

证据：`output/f1-derivation-callback-records-tests.log` 与 `output/f1-derivation-callback-records-checks.log`。仍未提交准备副本数据库或启用副本，F1 保持进行中。

下一阶段框架记录转换须特别核对 `framework_checks` 与 `framework_judgments` 的 candidateSha256：它们摘要完整 Candidate，资产转换改变 taskId 后必须同步重算；过期历史 judgment 不能误绑定最新配置或候选。framework observation 的 referenceId 指向全局 asset-reference，recovery 的 operationId 指向 framework-operation；任意输入/输出摘要及叙述内容不得猜测替换。

## 2026-09-21：资产记录与反馈重放的身份转换

新增 `project_derivation_asset_records::rewrite`，转换资产状态、独立及嵌入的 reference、交付候选和 owner decision 的任务/项目身份；reference 实体 ID 也按全局映射转换。candidate、stage、attempt、frame、feedback 和请求的局部 ID 保持，执行会话身份及文件路径留给后续阶段。

核对真实反馈生产者发现 fingerprint 包含源任务与 reference ID，不能直接保留。转换前从持久化反馈重建 Submission 并验证源 fingerprint，转换后调用真实 fingerprint 算法重算，以保持目标反馈重试判定。回归使用真实资产、reference、交付 inspect、decision duplicate 和 feedback duplicate 读取边界验证关联及重放，并覆盖损坏 fingerprint 和命名空间归属冲突。

证据保存于 `output/f1-derivation-asset-records-tests.log` 与 `output/f1-derivation-asset-records-checks.log`。本批未提交准备副本数据库，框架及 callback 记录、回执归档、路径和会话转换、活动状态中断、完整核对及启用继续待办。F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：任务与操作记录的身份转换

新增 `project_derivation_task_records::rewrite`，在内存转换任务主键、项目归属、父子及依赖关系、验证引用、历史 feature adoption 和操作日志内的 `taskAfter`。功能名称保持语义 ID，功能存储键切换到目标项目；callback revision 保持数值并转移到新任务键。操作日志要求任务身份和项目归属与其嵌入任务一致，缺失映射、错误引用类型及非法 revision 均拒绝。

回归将记录转换到独立 Store，使用真实 callback 查询读取新任务与 revision，验证操作日志任务快照、功能 adoption 和父子关系一致，并再次执行真实派生归属及引用预检。源记录、提示词、路径和执行会话身份保持原值，后两项留给后续路径转换与中断恢复阶段。测试与检查证据分别保存于 `output/f1-derivation-task-records-tests.log` 和 `output/f1-derivation-task-records-checks.log`。

本批仍是纯内存转换，尚未提交到准备副本；资产及框架记录、回执归档、文件和会话转换、活动状态中断、完整核对及启用继续待办。F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：验证记录的身份转换内核

新增 `project_derivation_validation_records::rewrite`，使用已核对的身份映射在内存转换验证记录，返回新存储键和 JSON。覆盖工作流及冻结版本、运行、基准、反馈、修复决策、覆盖记录和发布候选；运行内的 judgment/confirmation 同步更新 run/baseline 引用。覆盖记录同步更新 `id`、`taskId`、`codeRunId` 并清除旧 watcher 缓存。转换只触及声明的身份字段，保留叙述文本、本地 evidence/request 身份、快照及内容签名。

发布候选先验证源 scope 与快照摘要，再调用真实业务 `release::scope` 重算目标 scope；损坏输入、缺失映射、主键不一致和将归档回执转回重放记录均拒绝。定向测试将转换后的记录写入独立测试 Store，通过真实 `release::inspect` 和工作流后续版本查询，同时证明源记录保持。独立审查发现 coverage 自身 `id` 需同步，核对真实生产者时同时补齐 `codeRunId`，已加入回归断言。

证据见 `output/f1-derivation-validation-records-tests.log` 与 `output/f1-derivation-validation-records-checks.log`。此内核尚未接入离线副本的数据库提交；任务及资产记录转换、回执归档、文件/会话转换、活动状态中断、完整核对和启用仍待实施。它不会写项目、启动作业或移除 pending，F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：持久化派生身份映射

离线准备回执升级为 `beaver-project-derivation-copy-v2`，保存经过归属和引用预检的确定性身份映射。实体和调用使用按源项目、目标项目、请求及实体类型隔离的 UUIDv8；工作流保留项目 ID 与定义键组成的业务摘要算法，版本键、任务附属记录和修复决策同步关联到对应的新身份。映射不依赖文件路径，源项目离线或副本移动后仍可重新计算核对；重复目标和与源身份冲突均拒绝。

`validationRequest` 只有参数摘要、结果和归属，缺少原始输入，无法安全重建目标请求键及重放条件，因此映射明确记录 `Archive` 处置。此处仅保存处置计划，尚未实际归档或删除任何回执。项目副本中的数据库、叙述文本及文件仍保持原内容，pending 标记继续阻止启用。恢复检查从副本临时数据库重新生成映射，拒绝篡改或缺项。旧 v1 内部准备回执不再接受，应保留原件并在全新目标重新准备。

12 项派生定向测试通过，包括映射稳定性、UUID 格式、相关身份一致性、真实工作流版本查询、错误主键拒绝、映射篡改和既有复制保护；证据见 `output/f1-derivation-identity-tests.log`。格式和有效行检查见 `output/f1-derivation-identity-checks.log`。数据库及引用改写、文件和会话索引转换、状态中断、最终启用及 API/UI 继续待办，F1 未完成，未构建发布包或递增版本。

## 2026-09-21：派生输入的归属与引用检查

独立副本准备和离线恢复检查现在对临时 SQLite 快照执行显式归属检查：项目记录必须存在，所有实体必须属于源项目，事件必须指向已知任务，调用 JSON 身份与 SQL 列必须一致。复用迁移的实体种类白名单与声明式引用检查，拒绝未知实体、宿主记录、外部项目归属、悬空引用及无法确定归属的旧验证回执；不扫描或替换提示词和调用输出中的叙述文本。此检查限定于已声明的字段，不等同于完整实体 schema 验证。

检查在创建准备目标之前执行，也接入已有副本的 `inspect`；源 SQLite 仍只在临时目录打开。新增回归覆盖错误归属、未知类型、旧回执、任务引用、事件归属和调用列冲突，并以真实项目库验证拒绝悬空引用时不创建目标且源文件清单和摘要保持。测试证据见 `output/f1-derivation-validation-tests.log`，格式和结构检查见 `output/f1-derivation-validation-checks.log`。稳定身份映射、路径及会话索引转换、派生启用与 API/UI 仍待实施，F1 保持进行中，未构建发布包或递增版本。

## 2026-09-21：验证请求回执归属修复

派生身份核对发现实际写入契约与迁移归属规则不一致：`validationRequest` 原先只保存 hash/result，无法从摘要键恢复项目归属。使用真实 `validation.settings.save` 生成双项目回执的迁移路由回归在修复前失败，转换后的项目库缺少该回执；证据为 `output/f1-validation-receipt-before.log`。初步将问题描述为阻止迁移过于宽泛，实际已复现的是回执未进入项目分区，不能据此宣称所有迁移流程都立即失败。

`validation::requests::Request` 现在要求非空字符串项目 ID，新回执保存明确 `projectId`，保留原请求键、参数摘要与结果。重放拒绝显式冲突或无效的回执归属；没有归属字段的历史回执继续按原摘要重放，不在读取时回填或猜测归属。迁移检查仍将这类历史回执报告为 `VALIDATION_REQUEST_OWNER_UNKNOWN`，本次未放宽旧数据检查。

新增 3 项回归覆盖不同项目同请求 ID、冲突重试、旧回执兼容及错误归属拒绝。真实归档、分区、转换和 Router 回归验证回执仅进入所属项目库，旧请求在迁移后重放而不再次增加设置 revision，原宿主和归档保持，其他项目及副本宿主不持有该回执。验证模块依赖测试及历史未知归属回归通过；日志为 `output/f1-validation-receipt-{tests,routing,dependencies,legacy}.log`，格式和有效行检查见 `output/f1-validation-receipt-checks.log`。

身份范围核对还确认验证媒体和服务活动运行索引直接使用 run ID；后续独立派生必须转换验证运行身份及相应引用。当前尚未实施身份/路径转换或开放派生 API/UI，F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：独立身份派生的离线副本准备

新增 `project_derivation_copy::prepare/inspect` 核心阶段。显式请求保存请求 ID、源项目 ID、目标新项目 ID 和规范源路径；同 ID、繁忙源、原始数据库写句柄、源内目标和已有目标均拒绝。持项目锁和 Windows 数据库只读共享句柄，对完整项目做文件清单与哈希复制，SQLite/WAL/SHM 一并保留，源数据库只在临时副本中检查。排除 Windows 已持锁文件的字节读取，在目标重新创建本地锁。既有备份复用新增的具名数据库句柄和清单排除能力，原备份默认清单范围不变。

目标采用独立包目录，内含 `project/`、`.beaver-migration-pending` 和最后写入的 `DERIVATION-COPY.json`；回执记录来源、新身份和文件摘要。准备完成仍保留原数据库身份和 pending，普通打开及登记读取被拒绝。`inspect` 可在源位置不可用后重新核验已完成的准备，不移除 pending、不登记、不调度。失败保留目标，未完整写入回执的副本不能通过检查；重新准备须使用新目标。当前仅 Windows 支持离线数据库写入排斥，其他平台明确拒绝。

定向测试 4 项通过，验证已提交 WAL 内容和源字节保持、源离线后的回执读取、项目锁及原始数据库写句柄排斥、身份与目标冲突不覆盖、缺失回执及篡改内容持续阻塞，证据为 `output/f1-derivation-copy-tests.log`。中途 I/O 故障注入尚未覆盖。独立审查提出的既有 writer 排斥疑虑已由实际 Windows 测试核验；链接遍历在 `safe_path` 内、进入递归之前拒绝；回执已启用未知字段拒绝。本阶段仅准备输入，任务及其他实体的身份映射、引用和路径改写、最终启用、业务 API 与 UI 尚待实现，不能视为完整独立派生功能。F1 保持进行中，未递增版本或构建发布包。

## 2026-09-21：转换副本的存储路由回归

新增 `project_migration_routing_tests.rs`，使用真实归档、恢复、分区、转换和 `ProjectStorageRouter`，补充副本到项目运行时的跨模块验证。pending 存在时拒绝打开且不发布任务索引；测试调用回执写入层后，原项目目录改名为不可用，两个项目仍可打开并按原任务身份路由。旧 queued 任务转换为 interrupted，工作区路径指向副本内；写入只进入对应项目库，副本宿主不新增任务，原宿主任务保持 queued 且无新字段。关闭后清除路由，再次打开可读，归档摘要保持一致。

本测试显式跳过工具探测，仅验证转换、回执与存储路由边界，不能替代真实工具启用、桌面重启或调度执行验收。原宿主 Router 保持原状，符合显式选择新数据目录启动的契约。首次测试将 `task_plan::eligible` 误当作完整调度条件，断言失败；核对调度器实际先检查 queued 状态后移除错误断言，产品逻辑未改变。

激活相关定向测试 3 项通过，证据为 `output/f1-migration-routing-tests.log`。Rust 格式、文档格式、有效行和 scoped diff 检查记录于 `output/f1-migration-routing-checks.log`；F1 继续进行，派生身份、真实工具及原生集成验收仍未完成，未构建发布包或递增版本。

## 2026-09-21：迁移界面

原生设置页顶部新增“数据迁移”，复用 Dialog 与统一业务桥，提供归档检查、全新副本准备、确认启用及已有分区副本继续启用。归档更改清除检查结果，准备后固定归档和副本路径；失败保留诊断及可能的部分副本位置，重新准备必须换目标。工具 JSON 路径可在启用失败后修正重试。成功说明当前环境未切换，并显示手动启动所需的数据目录。旧 Electron 不展示入口。

`scripts/migration-ui-smoke.ts` 以独立 Electron 渲染器加载真实对话框和模拟业务桥，验证目录选择取消、检查失效、失败目标恢复、重复请求防护、处理中关闭和 Escape 保护、启用确认/取消/重试、继续启用以及手动切换说明。交互、TypeScript、格式和有效行检查通过，证据为 `output/f1-migration-ui-{flow,types,format,structure}.log`。独立只读审查未发现具体问题；此检查不等于原生 API 集成、真实工具启用或回退验收。F1 仍进行中，未构建发布包、未递增版本。

## 2026-09-21：迁移原生业务入口

统一业务目录接入 `migration.inspect`、`migration.prepareProjects` 和 `migration.activate`，沿用桌面桥及经授权的 HTTP/MCP 路由与摘要调用日志。准备流程明确执行完整恢复及项目分区；启用必须存在 `PROJECT-MIGRATION.json`。运行时校验绝对路径、新目标和运行中宿主目录隔离，以进程内互斥拒绝重复并发操作，阻塞线程承担迁移 I/O。不会自动选择默认目录、唤醒调度器或发起模型请求。导入 UI 尚待接入。

桌面定向测试 3 项通过并完成编译，覆盖参数及宿主路径隔离、实际归档检查与分区、重复准备不覆盖、分区失败保留部分副本、未分区副本拒绝启用，以及工具 JSON 解析失败时 pending 保留。测试比较源目录、归档和宿主夹具字节，工具失败断言确认实际到达 JSON 解析边界。独立只读审查未发现具体新问题；不代表真实工具启用、默认环境切换或完整回退验收。证据为 `output/f1-migration-entry-tests.log`；格式和结构检查记录在同前缀 `format`、`structure` 日志。F1 继续进行，未构建发布包或递增版本。

## 2026-09-21：副本登记的只读数据库检查

推进派生身份前检查现有登记入口，发现 `project.import` 读取尚未登记的项目时使用可写 `ProjectStore::open`。带已提交 WAL 的可携带副本在登记后会发生 checkpoint 或 WAL/SHM 清理；新增回归在修改前实际失败，证明登记读取会改变源存储文件。

新增 `ProjectStore::read_project`，取得项目排他锁并重新检查清单与路径后，将 SQLite、WAL、SHM 复制到临时目录，仅打开临时数据库读取项目实体。复用版本、结构、完整性和数据库身份检查，临时连接先关闭再清理文件。`project.import` 改用该入口，源库不开 SQLite 连接；同 ID 冲突仍明确拒绝，未隐式派生新身份。已有 Runtime 的打开语义保持不变。快照一致性依赖 Beaver 写入方遵守项目锁，繁忙项目明确拒绝此离线读取。

身份登记 3 项及项目存储 4 项定向测试通过，覆盖 WAL 中的项目元数据、宿主提交失败后重试、成功和失败时源存储字节保持、忙锁、实体/数据库身份不匹配，以及原有同 ID 冲突保护。Windows 活动锁文件不参与字节读取，避免系统范围锁拒绝读取；SQLite/WAL/SHM 及清单参与比较。Rust 格式、文档格式和有效行检查通过；证据为 `output/f1-import-readonly-{before,tests,storage,format,structure}.log`。独立只读核验未发现本批具体问题。派生身份的改写、来源回执与恢复协议尚未实现，F1 保持进行中；未执行原生发布验收或递增版本。

## 2026-09-21：离线登记诊断

新增只读 `project.storage.status {id}`，直接从宿主读取当前登记，复用项目清单检查，区分 offline、legacy、invalid、detected。调用日志也走宿主，不依赖本地 Runtime，不打开或初始化项目数据库。detected 明确仅表示目录及清单可读取，不承诺数据库或恢复状态正常。管理对话框显示诊断并提供重新检查；返回路径与打开窗口时的路径不同则要求刷新，旧异步结果在清理后丢弃。

核心状态测试 3 项、桌面登记日志路由测试 1 项、渲染器交互检查及 TypeScript 检查通过。交互夹具新增离线诊断展示断言；核心证明缺失目录、legacy、有效及无效清单均不创建数据库、不改写清单或宿主登记。独立只读核验未发现本批具体问题。证据为 `output/f1-registration-diagnosis-{core,desktop,ui,types,format,structure}.log`；未运行原生发布验收。派生身份、迁移入口及剩余存储归属闭包继续实施，F1 未完成。

## 2026-09-21：项目登记管理界面

原生项目工具栏新增“管理项目登记”，复用 Dialog、目录选择器和通知。对话框固定打开时的项目 ID 与路径，重新关联和注销均发送 `expectedPath`，不因后台刷新静默替换比较条件。注销先显示确认，说明文件与历史保留及活动工作收尾；失败保留表单供重试，处理中阻止重复提交和关闭。成功后沿用 App 的状态刷新及项目选择回退。旧 Electron 业务入口不展示该原生专属操作。

`scripts/project-registration-smoke.ts` 在独立 Electron 渲染器中加载真实 React 对话框，以模拟桥验证目录选择取消、重新关联 CAS、失败重试、注销确认/取消、重复点击、处理中关闭保护和 draining 提示。此检查不启动 Beaver.exe、不触碰实际项目，不代表原生端到端验收。交互、TypeScript、定向 Prettier 和有效行检查通过；日志为 `output/f1-registration-ui-{flow,types,format,structure}.log`。未运行发布构建或全量验收，版本保持不变。

日期：2026-09-16。所属计划：[实际框架实现计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) F1。

状态：进行中。已实现项目存储基础组件、共享运行时句柄、项目内快照及任务/资料/资源/Codex HOME/回调/框架作业/验收证据/游戏导出准备/观察截图/Blender 检查点的文件句柄适配、执行及资源读取入口的工作区归属校验、离线归档的数据库归属/引用/blob 检查、归档文件清单、显式源文件引用与 Codex 会话索引路径检查，以及执行时宿主配置与任务存储的读取边界。空白项目创建现先初始化 `.beaver` 和项目本地数据库，再登记到宿主；项目创建、NPR 安装、`workflow.*` 和 `task.create` 已进入首批生产路由。任务数据分发入口已经统一使用共享任务路由选择器；动态创建、导入和首次发现项目会在 Runtime 对外可见前依序恢复 journal、任务、Validation 和 setup 状态。迁移启用、状态聚合及完整生命周期 API 仍未完成。F1.1-F1.4 均保持未勾选，本记录不表示 F1 已交付或框架已可执行。

## 1. 项目存储基础

- `native/core/src/store_schema.rs` 提取现有宿主数据库的共享 DDL。`Store::open` 保持宿主语义，项目打开逻辑单独校验数据库身份和结构。
- `project_storage_layout.rs` 固定 `.beaver/project.json`、`project.sqlite` 及六个目录：`content`、`workspaces`、`previews`、`evidence`、`operations`、`cache`。schema 和 storage 版本均为 1，清单最大 64 KiB。
- `project_storage_database.rs` 创建并校验 `project_identity`、SQLite application ID `0x42455652`、user version 1、WAL 模式及实际表结构。打开已有数据库前复制 DB/WAL/SHM 到临时目录进行校验；校验失败不修表、不补文件、不退回宿主数据库。
- `project_storage.rs` 提供显式 `ProjectStore::initialize/open`、访问句柄及 `ProjectStores::open/close`。初始化使用 `.storage-pending` 标记；失败留下目录供诊断，不覆盖已有 `.beaver`。打开期间持有 `.project.lock` 排他锁，释放顺序保证先关闭 SQLite 再释放项目锁。
- `ProjectStores` 可独立持有多个项目；同一规范路径重复打开共享同一 `ProjectRuntime`，同一 ID 同时指向不同目录时拒绝打开。缓存命中仍重查清单、pending 标记和布局。`close` 在外部运行时、数据库或文件句柄未释放时返回错误，关闭后才可对新位置重新打开。
- `project_runtime.rs` 将 `Arc<Mutex<Store>>` 与 `Arc<Files>` 配成项目运行时。数据库和文件句柄各自保留共享的 OS 排他锁，即使注册表已经销毁，后台仅持其中一个句柄也不会提前释放项目写入权。此处为宿主进程的项目存储所有权；F4 的对象修改权仍未实现。宿主位置登记、离线状态及显式重定位操作仍待接入。
- 存储路径拒绝符号链接及目录联接；项目及祖先目录存在 `.beaver-migration-pending` 时拒绝启用。清单识别会保留源文件中的未知字段，不重写外部清单。

`object_framework_status.rs` 复用只读布局检查，F0 的状态协议继续有效。`detected` 只表示识别到项目清单；对象查询、制造查询和执行 capability 仍为 `false`。

## 2. 旧数据库归属检查

`project_migration_inventory.rs` 和 `project_migration_ownership.rs` 为已经完整生成的离线迁移归档增加只读检查。命令分派已接入 `migration_bundle::command`：

```text
Beaver.exe --migration-bundle inspect-projects BACKUP
```

此命令已通过核心分派测试和桌面编译检查；本轮尚未构建包含该命令的新 EXE。

检查先调用既有归档校验，再复制归档内宿主 DB/WAL/SHM 到临时目录读取，最后重新校验归档。不会打开源项目路径，也不会原地打开归档数据库；受控用例确认归档文件清单及摘要保持一致。前提是使用已停止写入、已经完成的离线归档，本步骤不负责生成在线数据库的一致性快照。

当前枚举 `entities`、`events`、`calls` 三张业务表的全部行，绕过 UI 事件查询的 500 条限制。结果格式为 `beaver-project-inventory-v1`，包括宿主、各项目和未归属记录的计数及逐项去向；不返回凭据、事件文本或调用正文。

归属规则基于明确的实体种类与现有写入约定：

- 宿主保留 `secret`、`secret_backup`、`settings/main`、`toolSetup/main`。
- 项目、任务、任务关联资产、框架操作、动态回执、检查及评价记录按已登记项目和任务身份解析。交叉检查实体键、直接及嵌套 task/project 字段；同项目内的不同 task ID 也属于冲突。
- 无任务关联的验收 run、flow、baseline、feedback、release 和 repair decision 按项目保存；settings/manifest 使用项目键。`validationFlowRevision` 的键必须等于 `flow.id:flow.revision`。
- 事件按所属任务解析项目；调用交叉核对 SQL task/project 列与 JSON 身份。只有 task 的调用随任务进入项目；确实无关联的宿主调用留在宿主。
- 未知实体种类、缺失任务、矛盾归属及无效 JSON 明确列入 `unresolved`，不凭未知实体中的 `projectId` 猜测。旧 `validationRequest` 哈希回执缺少项目字段时报告 `VALIDATION_REQUEST_OWNER_UNKNOWN`。

报告新增 `entityReferences` 和 `contentReferences`，各自包含检查次数和问题列表。问题只记录来源种类、ID、JSON Pointer 字段、稳定原因，以及适用的目标种类和 ID，不输出原始字段正文。`contentReferences.blobs` 列出已验证 blob 及需要该内容的项目 ID 集合，允许同一内容被多个项目历史引用。

`project_migration_references.rs` 和 `project_migration_reference_fields.rs` 按显式字段检查引用：

- 任务父子、依赖、反馈、验收 run、上一 feature 任务，以及未完成 operation 的 task 和 taskAfter 引用。
- 验收 coverage、flow/revision 中的 task、run/release/baseline/feedback 之间的引用、baseline 历史链及 repair decision 的 run 键和返回结果。
- run 和 release 的冻结 flow 必须能找到 `id:revision` 历史记录，并匹配 `id`、`projectId`、`revision`、`definition`。当前 flow 已退役不会使有效历史引用失效。
- 资产候选的 `inputCandidates` 必须存在于同一任务的候选种类中。缺失引用、字段类型不正确、无法解析的归属和跨项目引用分别报告。

`project_migration_content.rs` 使用已经完成摘要验证的应用归档清单，检查任务 baseline/changes/feature snapshot、operation changes/taskAfter、feature snapshot、验收 run/release snapshot 和资产候选 files 中的 blob。逐项检查摘要格式、`blobs/<hash>` 文件存在及内容摘要与文件名一致；无归属内容明确报告。`snapshotId`、`scopeId` 和证据文件的 `sha256` 不作为 blob 引用处理。

所有结果均返回 `readyToActivate: false`。以上检查只覆盖列明的字段，不做完整旧记录 schema 校验、父子双向一致性或依赖环检查。本命令不转换、不复制到项目库、不启用路由，也不丢弃无法归属的旧记录。

## 3. 归档文件清单与已知路径检查

`project_migration_files.rs` 在报告中增加 `files`，逐项列出应用和项目归档清单中的文件与目录，记录归档相对路径、字节数、摘要、类别和归属。测试比较报告路径集合与两类归档清单，确认没有遗漏；不返回文件正文。

- `workspaces/<task>`、`codex/<task>` 和 `asset-observer/<task>` 根据已核验的任务归属解析项目；`validation/<run>` 还要求 run 实体 ID 与 JSON 的 `id` 一致。
- 被已知历史记录引用且摘要正确的 blob 保存所需项目集合，允许多个项目共享同一内容；数据库、WAL/SHM 和已知顶层分组目录标记为容器，不直接分给宿主或项目。
- 项目归档内全部内容按其清单项目 ID 列出，包括根元数据和上下文文件；这一步没有转换它们的语义或内嵌路径。
- 未知目录、孤立任务/验收文件、未引用 blob、交付导出临时目录保留为 `unresolved`，不静默删除或猜测去向。文件查找拒绝大小写折叠后有歧义的路径；非规范大小写的实际顶层目录仍保守列为未归属。

`project_migration_file_fields.rs` 枚举明确的旧字段，`project_migration_file_references.rs` 对照已经校验的归档清单检查类型、存在性、项目归属及适用的摘要：

- task 和 operation 的 `taskAfter`：工作区必须是 `workspaces/<task>` 目录；可选 `assetRestore` 必须位于相应项目任务的 `codex/<task>/asset-checkpoints/`。
- task 和 operation 的 `taskAfter.references[*].path`：仅在该任务已归档的工作区中解析。合法但缺失的源文件报告 `TASK_REFERENCE_UNAVAILABLE`；非法路径及字段形状单独报错，不回退到原项目或其他任务的副本。
- `asset-reference.imagePath`、资产状态 `lastFrame` 和 `feedback[*].reference`：检查 task/project 身份、`asset-observer/<task>/references/<id>.png` 的精确路径和 SHA-256。
- 资产状态 `checkpoint` 与 `feedback[*].checkpoint`：验证检查点文件；同项目后续任务可引用先前任务的检查点，跨项目拒绝。
- task 和 operation 的 `taskAfter.assetFeedbackSeed`：检查引用截图及检查点，允许引用同项目先前任务的证据；资产状态 `work.attempts[*].checkpoint/endCheckpoint` 同样检查。缺失可选字段保持兼容，显式错误类型列入问题。
- `project_migration_content.rs` 检查 `work.attempts[*].inputFiles[*].sha256` 指向的冻结归档 blob，并将有效内容纳入项目保留清单。工作区同一路径后来已被修改，也不会替代此前各次尝试的冻结输入；本项不负责完整校验输入文件的 path/role schema。
- `validationRun.evidence[*]`：验证 `validation/<run>/` 下的相对文件路径及摘要，拒绝目录冒充文件、路径穿越、ADS 和无效字段类型。
- run 冻结 flow 定义、步骤及 evidence 中的 `references`：复用验收路径解析器，支持 `res://`，仅以精确大小写匹配该 run 的冻结 snapshot 键并核对归档 blob。缺失快照、缺失键和非法声明分别报告；不读取当前项目文件或当前 flow 定义。

绝对路径使用既有 Windows 词法路径解析，不读取原始宿主目录。新用例在归档完成后重命名原宿主和项目目录，再执行真实 `inspect-projects` 分派；扩展路径前缀及大小写差异仍可解析。报告只输出稳定问题码及字段位置，不回显无效字段正文。检查前后核对归档内容清单和摘要，均保持一致。

文件归属只说明历史内容属于哪个项目，不授予直接复制或启用资格。任务 Codex HOME 的 `config.toml`、`AGENTS.md` 含宿主工具路径、服务地址及会话端口，需要在目标宿主重新生成或验证。当前 provider 凭据由宿主通过环境变量提供，不能将该环境序列化到项目。Codex SQLite 会话索引中的 rollout/cwd/project root 路径仍需明确转换；JSONL 正文继续按既有约定保持不透明，不做字符串批量替换。

反馈 `history[*].evidence` 是说明文本，尝试的通用 inputs/outputs/tools 也是不透明载荷，不递归猜测其中字符串为路径。剩余闭包包括会话数据库路径转换与兼容处置、其余已知上下文内嵌引用、根元数据处理和未归属内容处置。当前新增检查覆盖上述显式字段，不宣称所有旧文件引用均已验证，F1.2 保持未完成。

### 3.1 Codex 会话索引

`project_migration_sessions.rs` 仅识别清单中的 `codex/<task>/state_<数字>.sqlite`，数字限定 ASCII。报告新增 `sessionIndexes`：`indexes` 为发现的索引数，`checked` 为已检查路径数，`issues` 为稳定问题码，`records` 保存索引归档路径、table、column、rowid 及解析后的 `targetArchivePath`。未知索引命名继续作为普通不透明文件保留。

检查将 DB 和存在的 WAL/SHM 复制到私有临时目录，只读打开副本、关闭 trusted schema 并执行 quick check。必需表 `threads` 必须具有 `id`、`rollout_path`、`cwd`；可选表 `project_roots` 和 `rollout_migration_skipped_rollouts` 可缺省，但存在时必须具有相应已知路径列。视图、WITHOUT ROWID、不支持的结构、损坏数据库和错误侧车类型明确报告，结构失败时不保留该索引的部分成功记录。

`project_migration_session_paths.rs` 对照已校验清单解析 `threads.rollout_path/cwd`、`project_roots.path` 和 skipped rollout 路径。rollout 必须属于当前任务 HOME；cwd 和 project root 必须是同一项目下的目录。显式项目根、存储路径别名及 Windows 大小写/扩展前缀可解析，缺失、跨项目、目录类型错误及歧义均不猜测。skipped rollout 缺失单独报告 `SESSION_SKIPPED_ROLLOUT_UNAVAILABLE`，留待显式处置，不能据此判断历史会话已经损坏或静默丢弃它。

不查询未知列、不解析 JSONL 正文，也不输出线程 ID、无效路径值、历史载荷或 SQLite 原始错误。四项归档级用例覆盖离线原目录、真实已提交 WAL、已知路径异常和不支持的索引结构，并核对检查前后归档清单及摘要。该功能只提供核对结果，没有改动旧会话索引转换器，也没有接通目标宿主路径转换或迁移启用。

### 3.2 恢复副本内的项目分库转换

`project_migration_partition.rs` 新增 `--migration-bundle partition-projects BACKUP RESTORED_DIRECTORY`：只接受 `migration_bundle::restore` 刚生成、`RESTORE.json` 标记 `paths_rewritten=false` 且 `data`/`projects/<id>` 清单与归档逐项一致的恢复副本，拒绝已经 `prepare-import` 转换或已分库的副本。分库前重新执行离线归档检查取得清单，全程不打开源数据。转换规则：

- `project_migration_plan.rs` 按清单归属生成纯计划。实体/事件/调用只有归属明确且没有引用、内容、文件或会话索引问题时才进入项目分区；任何有问题的实体连同其任务簇（任务、资产任务、操作、引用、`<kind>/<task>` 记录、事件、调用及工作区/会话/截图文件）整体保留在宿主副本，回执 `retained`/`retainedFiles` 列出表、种类、ID 和原因（`TASK_RETAINED`、`RUN_RETAINED`、`FILE_OUTSIDE_APPLICATION` 等），不推测目的地。项目实体自身有问题时整个分库失败。
- 文件按 `Files` 项目布局搬入 `projects/<id>/.beaver`：`workspaces/<task>` → `.beaver/workspaces/<task>`，`codex/<task>` → `.beaver/workspaces/.codex/<task>`，`asset-observer/<task>` → `.beaver/evidence/asset-observer/<task>`，`validation/<run>` → `.beaver/evidence/<run>`，共享 blob 复制到每个引用它的项目 `.beaver/content/blobs`。复制后逐文件核对字节数和 SHA-256。
- `project_migration_rewrite.rs` 只改写显式字段：任务/操作 `taskAfter` 的 `workspace`（必须等于 `.beaver/workspaces/<id>`）、`assetRestore`、`assetFeedbackSeed`，资产任务的 `checkpoint`/`lastFrame`/`feedback[*]`/`work.attempts[*]` 检查点，以及 `asset-reference.imagePath`；无路径字段的实体按原始字节复制，未知字段保持不变。
- `project_migration_session_rewrite.rs` 用清单已核对的 Codex 索引记录（`threads.rollout_path`/`cwd`、`project_roots.path`、跳过 rollout 表）逐行更新分区内的 `state_*.sqlite`，目标指向分区内 HOME、任务工作副本或恢复副本中的项目根，写前确认目标文件/目录存在；JSONL 正文不解析。
- 分区通过 `ProjectStore::initialize_partition` 建立完整的 `.beaver`（清单、身份表、目录、锁），但恢复根的 `.beaver-migration-pending` 仍然存在，`ProjectStore::open` 继续拒绝该副本；所有分区写入并核对后，宿主副本在同一事务内删除已搬走的实体/事件/调用行（项目实体除外：宿主副本保留它作为登记，分区持有项目自己的副本），再删除已搬走的文件与目录，blob 保留供保留历史使用，避免双来源。回执 `PROJECT-MIGRATION.json` 以 `create_new` 写入恢复根，记录每个项目的实体/事件/调用/文件/改写字段/会话路径数量、保留项和启用阻塞项，`readyToActivate` 恒为 false。

定向用例 `project_migration_inventory_tests::file_inventory::partition` 2 项：两项目、共享 blob、观察截图、验收证据、Codex 索引及三类调用记录的分库结果（含分区可作为完整项目 Store 打开、`resolve_workspace`/`blob` 解析、索引三条路径改写、宿主副本只剩设置和保留任务）；以及拒绝重复分库和已转换副本。该批与既有迁移用例合计 5 项通过，归档前后清单一致。分库不等于启用：保留项的处置界面、宿主生成配置（HOME 内 `config.toml`/`AGENTS.md`/skills 由 `codex_home::prepare` 在下次准备时按当前宿主重建，本批按原样复制）、`.beaver-context` 与根元数据只是随工作区原样搬迁，F1.2 与 F1.3 保持未完成。

### 3.3 分库副本的启用

`activate-import BACKUP PREPARED_DIRECTORY [TOOL_PATHS_JSON]` 现在按副本根是否存在 `PROJECT-MIGRATION.json` 分流：存在时走 `project_migration_activation.rs`，否则保持原 `migration_activation::activate` 的 `prepare-import` 路径不变。分库启用分为两段：

- `stage`（不依赖工具检测，可重复运行）：核对分库回执格式和归档两份清单的 SHA-256，`RESTORE.json`/回执/归档三方的项目集合必须一致，副本 `data` 位置不能移动；对 `data/.beaver-native.lock` 取排他锁。逐个分区用 `ProjectStore::open_partition`（允许祖先 pending 标记的打开入口，只供启用检查使用）打开，`project_migration_activation_partition.rs` 检查分区内项目实体登记、树内没有链接/特殊文件、每个任务和操作 `taskAfter` 的 `workspace` 等于 `.beaver/workspaces/<id>` 且目录存在、操作与任务同项目，并只读核对分区内每个 Codex `state_*.sqlite` 的 rollout/cwd/project root 路径都落在分区内且目标存在。宿主副本：登记集合与归档一致，且要么全部仍指向原路径，要么全部已改为分区路径（部分转换即失败）；宿主 `settings.tools`/`toolSetup` 按 `prepare-import` 同样的规则再绑定（`tools/` 内的托管工具指向副本，外部工具列入 `external_tools_to_check`，setup 结果标记不可用）；宿主内残留的每个任务都是保留历史，写入 `migrationRetained{reason,source,markedAt}`（原因来自分库回执，未列出的记 `HOST_UNPARTITIONED`），旧版凭据经 `legacy_vault::prepare` 转换；以上宿主写入在同一事务内完成。随后宿主 `recover_tasks` 中断残留的运行中/排队任务，宿主 Codex 索引按 `rebase_codex_indexes` 改写；每个分区 `convert`：把分区自己的项目实体 `path` 改为分区根、执行文件日志恢复、拒绝仍阻塞的日志、中断归档时正在运行的任务。整个阶段 pending 标记和 `ACTIVATION.json` 都不变。
- `activate`：在 `stage` 之上执行既有的目标工具检测和 `settings`/`toolSetup` 写入，再次核对归档，`ACTIVATION.json`（格式 `beaver-project-activation-v1`，含每分区任务/操作/索引/中断数量、保留任务数、宿主中断数、凭据转换数、索引改写数、外部工具列表）以 `create_new` 写入后才删除 pending 标记。

`migrationRetained` 任务在核心规则里被隔离：`task_plan::eligible` 不再领取它们，`task_actions::continue_task` 拒绝继续，因而它们在启用后不会被自动执行；显式转换入口尚未提供。定向用例 `file_inventory::activation` 2 项：运行中的任务与会话索引分库后经 `stage` 转为中断状态、宿主登记和分区登记改为分区路径、工作区外的任务保留并标记且不可领取/继续、外部 `codex` 列入待检查、重复 `stage` 幂等；以及宿主登记被改动或用另一份归档启用时失败且标记保留。`migration_activation.rs` 的工具检测和回执写入被抽成 `bind_tools`/`write_receipt` 供两条路径共用，原 `prepare-import` 用例继续通过。未完成：启用后的桌面尚未把分区路径的登记自动纳入 `ProjectStorageRouter`（登记路径就是分区根，`registered_project_uses_local_storage` 可以识别 `.beaver`，但迁移入口仍未在桌面开放）、保留任务的显式转换/处置入口、目标工具检测未在自动化用例中执行（需要真实 codex/godot）。

## 4. 运行时准备与当前生产边界

`ProjectStore::into_runtime` 提供可共享的项目数据库和文件句柄。项目模式的 `Files` 将快照 blob 保存到 `.beaver/content/blobs/<sha256>`，读取和写入前校验路径组件，拒绝打开后被替换成目录联接的内容目录；原 `Files::new` 保持宿主 `blobs/<sha256>` 布局。快照不包含 `.beaver`，关闭并搬动完整项目后，数据库中的冻结快照仍可从项目自身恢复。

`Files::workspace` 为任务创建、计划子任务和资料编辑提供统一物理工作区解析：项目句柄定位 `.beaver/workspaces/<task>`，宿主句柄保留 `workspaces/<task>`。解析拒绝非法路径组件和目录联接，且不会创建目录。`Files::workspace_location` 则提供任务记录使用的位置：新项目任务固定保存斜杠分隔的 `.beaver/workspaces/<task>`，旧宿主任务继续保存历史工作区字符串。

任务创建与计划准备、后续任务与对话回退、功能包任务、资料保存和资源导入/截图保存均显式接收同一个 `&Files`；快照、恢复和 Journal 不再在这些边界从裸 root 重建句柄。资产反馈提交、验收反馈任务和自动修复也把该句柄继续传给任务创建。功能包解析的临时目录使用系统临时目录，冻结内容仍通过所传文件句柄保存。

`Files::codex_home` 将项目任务的持久会话保存到 `.beaver/workspaces/.codex/<task>`，与 `.beaver/workspaces/<task>` 工作副本分开；宿主句柄保留 `codex/<task>`。两者均拒绝非法任务标识和路径中的目录联接；`.codex` 保留为 HOME 分组名，不能作为工作副本任务 ID。解析不会创建目录。此处没有新增顶层存储目录，也不把会话放入可清理的 cache。

`launch::prepare` 和 `codex_home::prepare` 接收与调度器相同的文件句柄，代码结构基线从该句柄的冻结 blob 读取。重新准备 HOME 会按当前宿主工具配置重建配置、AGENTS 和 skills，保留已有 session JSONL；宿主凭据只进入进程环境。工作副本恢复不会覆盖 HOME。此处只完成启动准备的路径适配，尚未迁移旧 HOME 或转换旧 SQLite 会话索引。

`Execution` 持有调度器的同一 `Arc<Files>`，动态工具、持久回调、框架作业和评价继续传递它，不再从资产观察 root 或 SQLite 父目录重建文件存储。异步作业保留该句柄及项目锁；回调文件捕获在 Store 锁外进行，提交时仍重新核验身份及 revision。检查、依赖导出、输入导出和评价读取同一项目的冻结内容，不回退旧 blob 目录。

交付检查的可重建导出位于 `.beaver/cache/delivery-exports`；宿主模式保留 `<data-root>/delivery-exports`。解析拒绝 cache 或导出目录被替换为目录联接。不同尝试从各自冻结清单导出，后续修改工作区文件不会覆盖历史输入。冻结源内容仍属于 `content/blobs`，不因导出缓存的生命周期而丢失。

`Files::resolve_workspace` 在项目模式要求记录原文严格等于当前任务的 `.beaver/workspaces/<task-id>`，并从当前项目根解析物理目录；绝对路径、反斜杠、路径穿越、重复或尾随分隔符、其他别名及错误任务 ID 均被拒绝。标准目录必须已经存在；入口返回经过布局检查的物理路径，不创建缺失目录，也不沿记录中的字符串继续访问。旧宿主模式保留绝对或非标准工作副本的历史行为。启动/HOME、执行器 spawn 与 turn cwd、框架作业、回调捕获、交付完成、验收规范化/捕获、任务完成及桌面工作流检查均经过此入口；验收修复子任务也从同一 Files 派生工作区。

资源列表、文本/原始字节读取、任务资源协议和打开工作目录显式接收 Files，在访问任务工作副本前执行同一校验；项目资源与项目目录继续使用既有项目根路径语义。交付审批在核验候选文件和冻结输入前校验工作区归属，其他工作区即使含有相同字节也不能充当本任务的验收来源。拒绝/返工操作不读取工作区，保留原行为。桌面入口暂时传入旧宿主 Files；接口适配不代表生产分库已启用。

项目模式的新任务、后续/回退/功能包任务、计划子任务、资料编辑任务、集成 steering 子任务和自动验收修复子任务均持久化规范相对位置。关闭、搬动并重新打开项目后，任务资源读取、工作副本准备及 Codex HOME 准备会从新项目根重新解析；项目实体自身的 `path` 仍按既有契约保存绝对项目路径。本批不承诺抵御同宿主进程在检查和实际读取之间替换目录的竞态。迁移导入、激活及离线引用审计继续按各自历史归档布局验证；历史绝对记录仍需显式转换，不能把旧迁移的 `data/workspaces` 直接替换为项目工作区。

验收 service 与 worker 保留同一 `Arc<Files>` 及项目锁，runner、coordinator、coverage、历史源码、媒体读取、对比、确认、反馈/修复冻结和发布检查继续传递该句柄。项目证据位于 `.beaver/evidence/<run-id>`，快照使用 `.beaver/content/blobs`；宿主模式仍使用 `<data-root>/validation/<run-id>`。证据目录解析不创建目录，写入入口自行创建；无效 run ID、证据父目录或 run 目录被替换为目录联接时拒绝访问。GUT 报告及日志叶路径也使用受检解析。

游戏导出的 `prepare/prepare_snapshot` 与预设冻结读取接收同一 Files；桌面导出入口在 Journal、发布授权、内部快照及准备间复用该句柄。临时构建目录由独立 `scratch` 参数提供，冻结文件恢复完毕后 `Prepared` 持有独立临时工作区；项目关闭后该工作区仍可读取，释放时自动清理。导出目标、绝对自定义模板和宿主工具安装目录保持既有语义，不改成项目存储路径。

观察冻结截图保存于 `.beaver/evidence/asset-observer/<task-id>/references/<reference-id>.png`，项目记录使用含 `.beaver/` 的相对路径。捕获、读取、标注、反馈提交及后台最终帧使用同一 Files；`asset_agent::Context` 保留共享句柄和项目锁。项目读取只接受该任务和截图的规范位置，拒绝外来绝对路径、路径别名及目录联接，不回退宿主。宿主读取保持历史记录路径语义；保留清理只删除本任务拥有的规范文件，不按可变的记录路径删除其他文件。任务路径方法集中到 `files_task_paths.rs`，快照与文件事务仍由 `files.rs` 负责。

Blender 检查点继续位于任务 HOME 的 `asset-checkpoints`，项目记录保存 `.beaver/workspaces/.codex/<task-id>/asset-checkpoints/<name>.blend`。HTTP 协议仍传输绝对路径，由 Rust 保存入口核对任务目录和实际文件后转换；恢复入口仅接受同项目规范相对路径，允许后续任务恢复先前任务的检查点。保存前先核验会话及目录归属，保存返回后再次核验会话；路径解析拒绝非法层级、外来绝对路径及 HOME/检查点目录联接。宿主模式保留历史绝对路径和自定义检查点目录。

保留清理通过同一 Files 解析当前状态、反馈、各次尝试的开始/结束检查点，以及尚未初始化资产状态的任务 `assetRestore` 和反馈种子检查点；不删除这些引用，并保留最新两个未引用文件。`Session` 持有共享 Files 和项目锁，关闭时使用该句柄保存。项目搬动后可读取相对检查点并为后续任务解析恢复路径；后续工作区批次又证明了新项目任务的工作副本和 HOME 准备可从新根解析，但完整 Codex 会话索引转换仍未实现。反馈种子尚未应用时，其 receipt checkpoint 可为空，恢复来源由 `assetRestore` 保存。

新建项目任务的工作区字段已改为规范相对位置；历史项目记录的转换和其他旧 data-root 读写者仍需逐项适配。桌面任务数据分发层的关联任务、任务设置、合并重试、接受/回滚、资源、事件、回调状态和 reveal 入口现统一复用 `business_routing::task_runtime_handles`，由同一个 Store/Files 句柄选择项目或旧宿主存储。共享句柄显式携带项目路由所有权，项目关联任务成功后才更新 Router 子任务索引；已登记本地项目的 Runtime 关闭时禁止回退宿主，旧宿主任务继续保留兼容路径。

桌面 `backend.rs` 仍持有宿主 `Arc<Mutex<Store>>`，用于全局配置、项目登记和旧宿主数据兼容；`ProjectStorageRouter` 管理已经打开的项目 Runtime。任务、回调、任务框架、控制、资产和调用日志入口已经按任务或项目选择权威句柄，Scheduler 也能从项目 Runtime 领取并在原 Runtime 收尾。journal、状态聚合、项目生命周期、迁移导入以及剩余直接 SQL 的归属仍需继续核对。

本批已建立以下执行读取边界：

- `execution_settings.rs` 从宿主 Store 一次解析 AI 配置、凭据、工具和 MCP 设置。选定能力解析失败直接报错，其他能力保留可选语义；`validationOnly` / `integrationValidation` 不解析或解密 AI 凭据。快照不实现 `Debug` 或序列化，凭据仅按既有机制传入进程环境和日志脱敏列表。
- `launch.rs` / `codex_home.rs` 消费该快照，不再从任务 Store 读取宿主偏好或凭据。任务 Store 仍供验收修复输入冻结使用；工作流只覆盖工具与 MCP 配置。任务能力与快照不一致时拒绝启动。
- `scheduler.rs` 保持一个全局 Scheduler，以宿主回调获取实时并发上限，仍按运行任务与活动 worker 数量限流。回调在获取任务 Store 锁之前执行，兼容当前共享 Arc，避免分库准备引入锁重入。领取结果携带任务所属 `TaskRuntime`，完成、排队中断和 worker 收尾均使用该运行时的 Store/Files；Blender `Session` 自身保存 Store/Files，关闭会话不再从 Runtime 列表回退选择 Store。可见项目 Runtime 优先于宿主 Runtime：已打开项目的任务不会由宿主影子记录领取，重复任务 ID 也不会重复执行；项目 Runtime 从可见列表移除后，已启动任务仍在原 Runtime 收尾。RuntimeSource 枚举失败时，Scheduler 会先取消活动任务并退出，避免遗留 `running` 状态。
- 验收 `ToolContext` 在任务 Store 锁内读取项目及项目验收设置，释放锁后才调用工具 resolver。桌面 resolver 独立读取宿主 Godot 配置；FFmpeg 仍来自项目验收设置。工具查找在数据库锁外进行。

后续切换仍需要协调状态聚合、调度恢复、项目登记与关闭、journal、迁移导入和剩余直接 SQL 的权威位置。以上边界不等于生产分库完成。Scheduler 关闭阶段现在直接遍历会话注册表并由每个会话使用自身 Store 保存检查点，即使 RuntimeSource 枚举失败也会继续关闭会话；RuntimeSource 已产生的错误仍按原规则返回给关闭等待者，同时确保活动任务已离开 `running` 状态。

旧资产任务的语义转换仍属于 F5。持久文件提交、对象版本发布、队列及新的制造执行资格不在本批基础组件中提前开放。

## 5. 定向验证证据

只运行相关核心用例、编译、格式及结构检查，未运行全流程验收。以下日志位于仓库 `output/`：

| 检查                                                                                                                                                    | 实际结果                                                                                                                                                                                   | 证据                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `object_framework_status` 定向目标                                                                                                                      | 2 项通过：只读识别、缺失/离线/不兼容状态                                                                                                                                                   | `f1-project-storage-tests.log`                                                                                                                                                                                                                                                                             |
| `project_storage` 定向目标                                                                                                                              | 4 项通过：WAL、未知清单字段、多项目锁、搬动后的数据保留及重复 ID                                                                                                                           | `f1-project-storage-tests.log`                                                                                                                                                                                                                                                                             |
| `cargo test --locked -p beaver-core --test project_storage_links --test project_storage_rejections`                                                     | 2 + 4 项通过：链接、pending、残缺目录、异常数据库拒绝且不修写                                                                                                                              | `f1-project-storage-rejections-tests.log`                                                                                                                                                                                                                                                                  |
| `cargo test --locked -p beaver-core --lib store::tests`                                                                                                 | 3 项通过：旧记录/未知字段、事务回滚、事件查询语义                                                                                                                                          | `f1-store-tests.log`                                                                                                                                                                                                                                                                                       |
| `cargo test --locked -p beaver-core --lib project_migration_inventory_tests`                                                                            | 7 项通过：原有归属用例及新增实体引用、冻结历史、共享/缺失/错误 blob 用例                                                                                                                   | `f1-closure-tests.log`                                                                                                                                                                                                                                                                                     |
| `npm run check:native`                                                                                                                                  | 新增引用和内容检查后通过桌面 Cargo check，9.22 秒                                                                                                                                          | `f1-closure-native-check.log`                                                                                                                                                                                                                                                                              |
| `npm run check:effective-lines`                                                                                                                         | 606 个源码文件，18 个未变动历史例外，0 个违规                                                                                                                                              | `f1-closure-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                                                              |
| 修改范围 `rustfmt --check`                                                                                                                              | 通过；使用仓库配置，`skip_children=true` 防止格式化无关模块                                                                                                                                | `f1-closure-format-check.log`                                                                                                                                                                                                                                                                              |
| `execution_settings` 定向目标                                                                                                                           | 4 项通过：宿主快照/凭据隔离、验收免解密、必选与可选能力、并发默认值及边界                                                                                                                  | `f1-host-test-execution_settings.log`                                                                                                                                                                                                                                                                      |
| `scheduler::tests` 定向目标                                                                                                                             | 4 项通过：宿主额度、活动 worker 计数、锁顺序、失败不领取及关闭队列                                                                                                                         | `f1-host-test-scheduler-tests.log`                                                                                                                                                                                                                                                                         |
| Scheduler 多项目 Runtime 生命周期定向目标                                                                                                               | 12 项通过：项目 Runtime 动态加入、项目优先于宿主影子任务、原 Runtime 收尾、RuntimeSource 失败取消活动任务                                                                                  | `f1-scheduler-runtime-tests.log`                                                                                                                                                                                                                                                                           |
| `validation::jobs::tests` 定向目标                                                                                                                      | 3 项通过：项目快照、独立宿主配置、共享 Store 锁释放、缺失项目拒绝                                                                                                                          | `f1-host-test-validation-jobs-tests.log`                                                                                                                                                                                                                                                                   |
| `codex_home::tests` 及排队资产任务中断定向目标                                                                                                          | 1 + 1 项通过：配置/会话隔离、排队中断且不启动 Blender                                                                                                                                      | `f1-host-test-codex_home-tests.log`、`f1-host-test-queued_asset_state_survives_interruption_without_starting_blender.log`                                                                                                                                                                                  |
| `npm run check:native`                                                                                                                                  | 宿主边界改动后桌面 Cargo check 通过，9.37 秒                                                                                                                                               | `f1-host-native.log`                                                                                                                                                                                                                                                                                       |
| `cargo check --locked -p beaver-core --example migrated_session --example scheduler_contract`                                                           | 两个受影响示例编译通过；未运行真实 Codex 会话                                                                                                                                              | `f1-host-examples.log`                                                                                                                                                                                                                                                                                     |
| `npm run check:effective-lines`                                                                                                                         | 本批 610 个源码文件，18 个未变动历史例外，0 个违规                                                                                                                                         | `f1-host-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                                                                 |
| 本批修改范围 `rustfmt --check`                                                                                                                          | 通过，`skip_children=true`；源码 UTF-8 无 BOM、diff 空白检查通过                                                                                                                           | `f1-host-format-check.log`、`f1-host-source-check.log`                                                                                                                                                                                                                                                     |
| `cargo test --locked -p beaver-core --lib project_migration_inventory_tests`                                                                            | 11 项通过：7 项原有检查及 4 项文件清单、离线引用和异常边界用例                                                                                                                             | `f1-files-tests-verified.log`                                                                                                                                                                                                                                                                              |
| `npm run check:native`                                                                                                                                  | 文件清单及路径检查接入后桌面 Cargo check 通过，10.62 秒                                                                                                                                    | `f1-files-native-check.log`                                                                                                                                                                                                                                                                                |
| `npm run check:effective-lines`                                                                                                                         | 614 个源码文件，18 个未变动历史例外，0 个违规                                                                                                                                              | `f1-files-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                                                                |
| 文件检查批次 `rustfmt --check`                                                                                                                          | 通过，`skip_children=true`；8 个源码文件 UTF-8 无 BOM，diff 空白检查通过                                                                                                                   | `f1-files-format-check.log`、`f1-files-source-check.log`、`f1-files-diff-check.log`                                                                                                                                                                                                                        |
| `cargo test --locked -p beaver-core --lib project_migration_inventory_tests`                                                                            | 15 项通过：新增 4 项历史反馈、尝试检查点和冻结输入 blob 用例                                                                                                                               | `f1-history-tests.log`                                                                                                                                                                                                                                                                                     |
| `npm run check:native`                                                                                                                                  | 历史引用检查后桌面 Cargo check 通过，8.76 秒                                                                                                                                               | `f1-history-native-check.log`                                                                                                                                                                                                                                                                              |
| `npm run check:effective-lines`                                                                                                                         | 615 个源码文件，18 个未变动历史例外，0 个违规                                                                                                                                              | `f1-history-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                                                              |
| 历史检查批次 `rustfmt --check`                                                                                                                          | 通过，`skip_children=true`；4 个源码文件 UTF-8 无 BOM，diff 空白检查通过                                                                                                                   | `f1-history-format-check.log`、`f1-history-source-check.log`、`f1-history-diff-check.log`                                                                                                                                                                                                                  |
| `cargo test --locked -p beaver-core --lib project_migration_inventory_tests`                                                                            | 19 项通过：新增 4 项会话索引、真实 WAL、异常路径及不支持结构用例                                                                                                                           | `f1-sessions-tests.log`                                                                                                                                                                                                                                                                                    |
| `npm run check:native`                                                                                                                                  | 会话索引检查接入后桌面 Cargo check 通过，9.70 秒                                                                                                                                           | `f1-sessions-native-check.log`                                                                                                                                                                                                                                                                             |
| `npm run check:effective-lines`                                                                                                                         | 618 个源码文件，18 个未变动历史例外，0 个违规                                                                                                                                              | `f1-sessions-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                                                             |
| 会话索引批次 `rustfmt --check`                                                                                                                          | 通过，`skip_children=true`；6 个源码文件 UTF-8 无 BOM、无尾随空白，diff 检查通过                                                                                                           | `f1-sessions-format-check.log`、`f1-sessions-source-check.log`、`f1-sessions-diff-check.log`                                                                                                                                                                                                               |
| `cargo test --locked -p beaver-core --lib project_migration_inventory_tests`                                                                            | 23 项通过：新增任务副本及验收冻结源文件引用的 4 项用例                                                                                                                                     | `f1-sources-tests.log`                                                                                                                                                                                                                                                                                     |
| 源文件引用批次编译、格式、编码及结构检查                                                                                                                | 桌面 Cargo check 通过，12.85 秒；619 个源码，18 个未变动历史例外，0 个违规；5 个源码 UTF-8 无 BOM，diff 检查通过                                                                           | `f1-sources-native-check.log`、`f1-sources-effective-lines.log`、`f1-sources-format-check.log`、`f1-sources-source-check.log`、`f1-sources-diff-check.log`                                                                                                                                                 |
| `project_runtime` 及 `project_storage` 定向目标                                                                                                         | 4 + 4 项通过：独立后台句柄保留锁、关闭与重新关联、冻结内容隔离及搬动、缓存重查                                                                                                             | `f1-runtime-tests.log` 中这两个目标通过；组合命令的链接目标初次失败，见下文                                                                                                                                                                                                                                |
| `project_storage_links` 及 `project_storage_rejections` 定向目标                                                                                        | 3 + 4 项通过：修正夹具后重跑链接目标及先前未执行的拒绝目标，包含运行中内容目录联接替换                                                                                                     | `f1-runtime-rejections-tests.log`                                                                                                                                                                                                                                                                          |
| `files::tests` 及 `store::tests` 定向目标                                                                                                               | 4 + 3 项通过：保留宿主快照、冲突恢复、损坏检测、事务及历史事件语义                                                                                                                         | `f1-runtime-files-tests.log`、`f1-runtime-store-tests.log`                                                                                                                                                                                                                                                 |
| 运行时句柄批次编译、格式、编码及结构检查                                                                                                                | 桌面 Cargo check 通过，14.54 秒；621 个源码，18 个未变动历史例外，0 个违规；9 个源码 UTF-8 无 BOM，diff 检查通过                                                                           | `f1-runtime-native-check.log`、`f1-runtime-effective-lines.log`、`f1-runtime-format-check.log`、`f1-runtime-source-check.log`、`f1-runtime-diff-check.log`                                                                                                                                                 |
| `project_task_storage`、`project_resource_storage`、`project_storage_links` 定向目标                                                                    | 3 + 3 + 4 项通过：项目任务/派生/功能包、资料日志恢复、写入阻塞、路径与运行中目录联接                                                                                                       | `f1-adapters-project-tests.log`                                                                                                                                                                                                                                                                            |
| 任务创建/计划/关联/功能包/资料/资源及验收任务集成/反馈上下文定向目标                                                                                    | 3 + 4 + 1 + 1 + 2 + 5 + 6 + 1 项通过，共 23 项旧入口回归                                                                                                                                   | `f1-adapters-task_create.log`、`f1-adapters-task_plan_tests.log`、`f1-adapters-task_relations.log`、`f1-adapters-feature_tasks.log`、`f1-adapters-documents.log`、`f1-adapters-assets-tests.log`、`f1-adapters-validation-task_integration_tests.log`、`f1-adapters-validation-feedback_context_tests.log` |
| `asset_contract` 和 `object_framework_boundary` 定向目标                                                                                                | 20 + 5 项通过：资产反馈/交付协议及未开放对象执行的边界                                                                                                                                     | `f1-adapters-contract-tests.log`                                                                                                                                                                                                                                                                           |
| 文件句柄适配批次编译、格式、编码及结构检查                                                                                                              | 桌面 Cargo check 通过，9.59 秒；623 个源码，18 个未变动历史例外，0 个违规；23 个源码 UTF-8 无 BOM，格式和 diff 检查通过                                                                    | `f1-adapters-native-check.log`、`f1-adapters-effective-lines.log`、`f1-adapters-format-check.log`、`f1-adapters-source-check.log`、`f1-adapters-diff-check.log`                                                                                                                                            |
| `project_codex_storage` 和 `project_storage_links` 定向目标                                                                                             | 4 + 5 项通过：项目冻结基线、宿主配置和凭据边界、恢复后会话保留、多项目隔离、缺失 blob 不回退、非法标识及 HOME 目录联接                                                                     | `f1-home-project-tests.log`                                                                                                                                                                                                                                                                                |
| `codex_home::tests`、`execution_settings`、`validation::feedback_context_tests` 定向目标                                                                | 1 + 4 + 1 项通过；本批共 15 项定向用例，没有运行全流程                                                                                                                                     | `f1-home-unit-tests.log`、`f1-home-settings-tests.log`、`f1-home-feedback-tests.log`                                                                                                                                                                                                                       |
| HOME 批次编译、格式、编码及结构检查                                                                                                                     | 桌面 Cargo check 通过，14.34 秒；受影响 migrated_session 示例编译通过但未执行；624 个源码、18 个未变动历史例外、0 个违规；10 个源码 UTF-8 无 BOM、格式及 diff 检查通过                     | `f1-home-desktop-check.log`、`f1-home-example-check.log`、`f1-home-effective-lines.log`、`f1-home-format-check.log`、`f1-home-source-check.log`、`f1-home-diff-check.log`                                                                                                                                  |
| `project_framework_storage`、`project_storage_links`、`framework_operations`、`framework_validation`、`framework_evidence`、`object_framework_executor` | 4 + 6 + 5 + 7 + 4 + 1 项通过；冻结回调、历史输入、项目评价、无旧目录回退及导出目录联接边界                                                                                                 | `f1-callback-integration-tests.log`                                                                                                                                                                                                                                                                        |
| `executor_tools::tests`、`task_callback` lib 定向过滤                                                                                                   | 5 + 3 项通过；两组包含 1 项重复执行的回调用例；未扩大到全套测试                                                                                                                            | `f1-callback-executor-tests.log`、`f1-callback-callback-tests.log`                                                                                                                                                                                                                                         |
| 回调批次编译、格式、编码及结构检查                                                                                                                      | 桌面 tests 编译通过，14.21 秒；migrated_session / executor_contract 示例编译通过，6.93 秒，未执行；625 个源码、18 个未变动历史例外、0 个违规；21 个源码 UTF-8 无 BOM、格式及 diff 检查通过 | `f1-callback-desktop-check.log`、`f1-callback-examples-check.log`、`f1-callback-effective-lines.log`、`f1-callback-format-check.log`、`f1-callback-source-check.log`、`f1-callback-diff-check.log`                                                                                                         |
| `project_codex_storage`、`project_framework_storage`、`project_storage_links`、`project_workspace_authority` 定向目标                                   | 5 + 5 + 6 + 3 项通过；启动、回调、作业、执行器及验收/完成拒绝非本任务工作区；2 个子进程辅助入口被忽略                                                                                      | `f1-workspace-authority-tests.log`                                                                                                                                                                                                                                                                         |
| `task_finish::tests`、`validation::task_gate`、`validation::task_integration_tests` lib 定向过滤                                                        | 6 + 4 + 6 项通过；工作区归属批次共 35 项定向用例通过                                                                                                                                       | `f1-workspace-task_finish-tests.log`、`f1-workspace-validation-task_gate.log`、`f1-workspace-validation-task_integration_tests.log`                                                                                                                                                                        |
| `cargo check --locked -p beaver-desktop --tests`                                                                                                        | 工作区归属改动后编译通过，9.45 秒                                                                                                                                                          | `f1-workspace-desktop-check.log`                                                                                                                                                                                                                                                                           |
| `project_workspace_readers`、`project_framework_storage` 定向目标                                                                                       | 2 + 6 项通过；资源列表/文本/原始字节/预览/打开目录拒绝其他任务或项目副本，审批拒绝外来相同字节；1 个子进程辅助入口被忽略                                                                   | `f1-workspace-readers-tests.log`                                                                                                                                                                                                                                                                           |
| `task_resources::tests`、`assets::tests`、`reveal::tests` 及 `asset_delivery_integrity` 定向目标                                                        | 2 + 5 + 1 + 4 项通过；读取及审批批次共 20 项定向用例通过                                                                                                                                   | `f1-readers-task_resources-tests.log`、`f1-readers-assets-tests.log`、`f1-readers-reveal-tests.log`、`f1-readers-delivery-integrity.log`                                                                                                                                                                   |
| 读取及审批批次编译                                                                                                                                      | `cargo check --locked -p beaver-desktop --tests` 通过，8.33 秒；`cargo check --locked -p beaver-core --example reveal_contract` 通过，6.21 秒；未执行示例                                  | `f1-readers-desktop-check.log`、`f1-readers-example-check.log`                                                                                                                                                                                                                                             |
| 工作区归属、读取及审批改动的格式与结构检查                                                                                                              | 23 个源码的定向 `rustfmt --check` 通过，`skip_children=true`；627 个源码、18 个未变动历史例外、0 个违规                                                                                    | `f1-workspace-format-check.log`、`f1-workspace-effective-lines.log`、`effective-code-lines.json`                                                                                                                                                                                                           |
| 本轮编码及空白检查                                                                                                                                      | 23 个源码及 2 份计划/记录共 25 个文件严格 UTF-8 解码、无 BOM、无尾随空白；限定范围 `git diff --check` 通过，未跟踪文件由显式扫描覆盖                                                       | `f1-workspace-source-check.log`、`f1-workspace-diff-check.log`                                                                                                                                                                                                                                             |
| `validation::project_evidence_tests` 与 `validation::release_tests` lib 定向过滤                                                                        | 5 + 5 项通过：项目证据隔离、关闭搬动后读取、历史反馈、后台句柄寿命、冻结发布范围及篡改拒绝                                                                                                 | `f1-evidence-project-tests.log`、`f1-evidence-release-tests.log`                                                                                                                                                                                                                                           |
| `validation::approval_tests`、`feedback_context_tests`、`task_integration_tests`、`jobs::tests`、`task_gate` lib 定向过滤                               | 4 + 1 + 6 + 3 + 4 项通过：原有审批、修复、覆盖率、worker 和任务门槛行为                                                                                                                    | `f1-evidence-approval_tests.log`、`f1-evidence-feedback_context_tests.log`、`f1-evidence-task_integration_tests.log`、`f1-evidence-jobs-tests.log`、`f1-evidence-task_gate.log`                                                                                                                            |
| `project_storage_links` 的 `active_evidence_handles` 定向过滤                                                                                           | 1 项通过：活动句柄拒绝证据父目录及 run 目录联接，外部目标未改变                                                                                                                            | `f1-evidence-links-tests.log`                                                                                                                                                                                                                                                                              |
| 验收证据批次编译、格式及结构检查                                                                                                                        | 桌面 tests 编译通过，9.50 秒；受影响 `validation_contract` 示例编译通过，5.41 秒，未运行；29 个源码定向格式检查通过；628 个源码、18 个未变动历史例外、0 个违规                             | `f1-evidence-desktop-check.log`、`f1-evidence-example-check.log`、`f1-evidence-format.log`、`f1-evidence-structure.log`                                                                                                                                                                                    |
| `game_export::` lib 定向过滤                                                                                                                            | 9 项通过：独立 scratch、项目搬动后的冻结导出、忽略实时预设及场景修改、缺失或损坏 blob 拒绝且不回退宿主、旧绝对模板及导出目标约束                                                           | `f1-export-tests.log`                                                                                                                                                                                                                                                                                      |
| 游戏导出批次编译、格式及结构检查                                                                                                                        | 桌面 tests 编译通过，2.92 秒；`export_contract` 示例编译通过，4.91 秒，未运行；4 个源码定向格式检查通过；629 个源码、18 个未变动历史例外、0 个违规                                         | `f1-export-desktop-check.log`、`f1-export-example-check.log`、`f1-export-format.log`、`f1-export-structure.log`                                                                                                                                                                                            |
| `asset_contract` 定向目标                                                                                                                               | 24 项通过：项目搬动后截图/反馈/模型输入保留、外来/损坏/缺失路径拒绝、保留清理和目录联接，以及原观察与提交行为                                                                              | `f1-observer-tests.log`                                                                                                                                                                                                                                                                                    |
| `executor_tools::tests::`、`project_codex_storage`、`project_workspace_authority`                                                                       | 5 + 5 + 3 项通过，1 个子进程辅助入口被忽略；本批共 37 项通过                                                                                                                               | `f1-observer-executor-tests.log`、`f1-observer-path-tests.log`                                                                                                                                                                                                                                             |
| 观察截图批次编译、格式及结构检查                                                                                                                        | 桌面 tests 编译通过，9.10 秒；15 个源码定向格式检查通过；632 个源码、18 个未变动历史例外、0 个违规                                                                                         | `f1-observer-desktop-check.log`、`f1-observer-format.log`、`f1-observer-structure.log`                                                                                                                                                                                                                     |
| `asset_contract` 定向目标                                                                                                                               | 30 项通过：新增检查点保存/搬动/后续恢复、外来目录及非法恢复路径拒绝、尝试和未启动任务保留、旧绝对路径兼容及目录联接用例                                                                    | `f1-checkpoint-tests.log`                                                                                                                                                                                                                                                                                  |
| `blender_session::tests::` lib 定向过滤、`asset_work` 定向目标                                                                                          | 1 + 4 项通过；检查点批次共 35 项通过，未启动 Blender                                                                                                                                       | `f1-checkpoint-session-tests.log`、`f1-checkpoint-work-tests.log`                                                                                                                                                                                                                                          |
| 检查点批次编译、格式及结构检查                                                                                                                          | 桌面 tests 编译通过，13.04 秒；11 个源码定向格式检查通过；634 个源码、18 个未变动历史例外、0 个违规                                                                                        | `f1-checkpoint-desktop-check.log`、`f1-checkpoint-format-check.log`、`f1-checkpoint-structure.log`                                                                                                                                                                                                         |
| 项目相对工作区的 10 个集成测试目标                                                                                                                      | 39 项通过、2 个子进程辅助入口忽略；覆盖项目任务写入规范相对位置、非法记录拒绝、旧宿主兼容、项目搬动后任务/资源/HOME 重新解析，以及 steering 与自动修复子任务                               | `f1-workspace-relative-project-tests.log`                                                                                                                                                                                                                                                                  |
| `task_create::tests::`、`task_plan_tests::`、`documents::tests::`、`validation::task_integration_tests::`、`codex_home::tests::`                        | 1 + 4 + 2 + 6 + 1 项通过；只运行相对工作区改动的直接依赖用例                                                                                                                               | `f1-workspace-relative-lib-task_create-tests.log`、`f1-workspace-relative-lib-task_plan_tests.log`、`f1-workspace-relative-lib-documents-tests.log`、`f1-workspace-relative-lib-validation-task_integration_tests.log`、`f1-workspace-relative-lib-codex_home-tests.log`                                   |
| `cargo check --locked -p beaver-desktop --tests`                                                                                                        | 项目相对工作区改动后桌面 tests 编译通过，8.79 秒                                                                                                                                           | `f1-workspace-relative-desktop-check.log`                                                                                                                                                                                                                                                                  |
| 项目相对工作区批次的格式及结构检查                                                                                                                      | 16 个 Rust 文件定向 `rustfmt --check` 通过，`skip_children=true`；635 个源码、18 个未变动历史例外、0 个违规                                                                                | `f1-workspace-relative-format-check.log`、`f1-workspace-relative-structure.log`、`effective-code-lines.json`                                                                                                                                                                                               |

上述编译和测试命令均通过 `rtk` 或 `rtk proxy` 在同一 PowerShell 工具链环境执行。存储和归档早期已有 22 项定向用例通过；宿主边界批次执行了 13 项相关用例。历史引用批次通过 15 项，会话索引批次只重跑受影响的迁移报告目标，共 19 项通过，未扩大到全套测试。PowerShell 将 Cargo 的进度 stderr 包装为 `NativeCommandError` 文本，通过日志对应退出码为 0，并包含 `Finished`。

早期 `f1-project-storage-tests.log` 的组合命令在链接测试夹具处失败，不能将整个日志标为通过。修正后链接和拒绝目标单独重跑通过，证据见对应日志。归属测试最初作为集成测试访问 crate 私有成员而编译失败；现改为 crate 内单元测试，未扩大生产 API 的可见性，并已编译运行通过。

文件检查批次首次在测试辅助函数处编译失败：`file_hash` 返回可选摘要，夹具未解包。修复后有 1 项路径用例失败，原因是夹具将空路径拼接出的尾分隔符误用为兄弟目录前缀；已改为直接构造兄弟目录路径。两个失败记录保留于 `f1-files-tests.log`、`f1-files-tests-pass.log`；最终通过证据为 `f1-files-tests-verified.log`，没有放宽生产路径校验。

运行时句柄批次初次组合命令退出 101：8 项 runtime/storage 用例通过，新增链接夹具把含 `/content` 的混合分隔符路径交给 `cmd /C mklink`，被解释为开关。改为逐组件拼接原生路径后，链接及拒绝目标共 7 项通过；没有放宽产品路径校验。另有 7 项 Files/Store 单元用例通过，本批共验证 22 项相关用例，未重跑已经通过且未再修改的目标。首次失败日志保留，不能将其整体标为通过。

归属用例覆盖 701 条事件、尚在 WAL 中的内容、两个项目、没有任务的项目验收、SQL/JSON 身份矛盾、不可恢复的旧回执、错误 flow revision 键和归档修改后的拒绝。新增用例覆盖两个项目共用 blob、已退役 flow 的冻结版本、baseline 历史、未完成 operation、错误/跨项目/缺失引用、冻结定义漂移、有效归档内的缺失或错误命名 blob，以及无效字段正文脱敏。各检查前后比较归档文件清单与摘要，确认未改写归档。临时夹具复制时没有并发写入；此结果不能证明线上生产迁移已经完成。

主要有效行数：存储生命周期 125、数据库校验 117、布局检查 121、共享 DDL 62、归属规则 191、归属报告 173、引用检查 149、字段映射 141、内容检查 124。归属测试为 315 行，引用与内容测试为 268 行，各自聚合共享离线归档夹具的行为用例；没有修改历史超限 baseline。

本批有效行数：宿主执行快照 83、快照测试 152、调度器 343、调度测试 110、验收 jobs 148、验收边界测试 118、验收 service 130、桌面任务运行时 144、桌面验收运行时 212。只读独立核验未发现本批接口的锁重入或宿主配置回退问题；此结论不覆盖生产分库后的多项目调度。

文件检查批次有效行数：文件清单 193、文件引用校验 158、字段映射 72、文件用例 334，集成后的归属报告 183。测试模块围绕同一离线归档夹具保持单一职责；未修改历史超限 baseline。独立只读核验补充确认了生成配置、凭据环境及会话索引的迁移边界，未将目录归属当作可直接迁移的证明。

历史引用批次有效行数：内容检查 162、文件字段映射 104、共享文件用例 336、历史用例 261。历史用例使用真实输入冻结和归档流程，原目录不可用后仍能检查，并确认没有改写归档或泄漏载荷。独立只读核验未发现本批序列化字段或冻结 blob 语义问题；不覆盖后续转换、启用和生产路由。

会话索引批次有效行数：索引检查 190、路径解析 114、索引用例 374、集成后的归属报告 192、共享文件用例 338、crate 模块注册 107。测试文件聚合同一归档索引的行为用例，保持单一职责；没有更新历史超限 baseline。独立只读核验未发现本批阻塞问题。WAL 夹具冻结期间没有并发写入，不能将此证据视为在线快照或生产迁移的验证。

源文件引用批次有效行数：文件引用 175、字段映射 118、内容检查 236、共享文件用例 340、源文件用例 255。新用例覆盖原路径不可用、源内容后来改变或删除、blob 损坏和报告脱敏，并确认归档保持不变。

运行时句柄批次有效行数：Store 199、数据库校验 125、项目存储 139、共享运行时 39、Files 420、模块注册 108、存储用例 161、运行时用例 194、链接用例 94。Files 继续集中负责快照及文件事务，未增加其他项目路径职责；未修改行数 baseline。独立只读核验未发现锁生命周期或关闭/重新关联阻塞问题，该核验与上述实际测试分别记录，均不代表生产路由已接通。

文件句柄适配批次共通过 58 项定向用例，未执行全量测试。新增项目任务用例从真实创建、计划完成、子任务合入和功能包采纳入口验证冻结内容；后续任务不读取父任务未合入草稿，对话回退保留修改前后内容，下一子任务读取已合入结果。资料用例在关闭重开后从项目 blob 恢复中断 Journal；未恢复操作阻止任务、资料、资源和截图写入。旧宿主入口的用例同时保留原行为。独立只读核验未发现本批句柄传递的阻塞问题；不覆盖仍待适配的证据目录和生产路由。

本批有效行数：Files 427、任务创建 323、任务计划 269、任务关联 319、功能包任务 259、资料 174、资源 346、资产提交 92；项目任务用例 257、项目资源用例 150、目录联接用例 116。各模块仍围绕原有职责；未修改历史行数 baseline。

HOME 批次有效行数：Files 443、HOME 准备 411、启动准备 99、项目 HOME 用例 213、目录联接用例 138。该批独立只读核验确认了 HOME 与工作副本的隔离，同时指出启动信任旧任务记录的绝对 `workspace` 字段。后续工作区归属批次补上标准目录校验，本轮相对位置批次又让新项目任务保存规范相对位置，并验证搬动后重新解析；早期 HOME 测试本身仍不能单独作为整个启动路径的隔离证明。

回调批次共 35 次定向用例通过（34 项不同用例）。独立只读核验未发现句柄传递、冻结内容及导出路径的阻塞问题；未新增并发捕获提交或异步任务执行中关闭项目的专门用例。新项目任务的工作区记录已在后续批次改为相对位置；历史记录转换和桌面旧存储入口仍是待处理边界，不把本批验证视为生产项目路由已经完成。

工作区归属批次通过 35 项定向用例，后续读取及审批批次通过 20 项；两批包含重复执行的框架用例，不合计为 55 项不同测试。项目模式拒绝同字节的其他任务/项目副本，审批失败不修改任务或工作流，恢复正确工作区后同一请求可正常批准。独立只读检索补齐了审批入口；旧迁移模块继续使用历史归档规则。未运行全流程、真实引擎或原生窗口验收。

本轮有效行数：Files 463、执行器 484、任务完成 466、HOME 准备 412、资源 352、资源读取 180、打开目录 165、交付审批 197；项目 HOME 用例 258、框架用例 231、工作区归属用例 139、资源读取用例 100。上述文件均在 500 行以内，保持各自既有职责；没有更新历史超限 baseline。

验收证据批次共 29 项定向用例通过，未运行真实引擎或全流程。首次项目证据测试有 2 项夹具失败：Windows 规范路径带 `\\?\` 前缀、反馈冻结要求目标工作区已存在。分别调整期望路径及创建夹具工作区后，该目标 5 项全部通过，没有放宽产品校验。该批移动用例只证明证据及冻结内容可读取；本轮相对位置批次另行证明新项目任务工作副本和 HOME 准备可从新根解析，仍不涵盖项目登记及完整会话索引的重定位。service 用例未入队真实引擎任务，只验证后台持有文件句柄与项目锁的生命周期。独立只读核验未发现遗漏的验收句柄重建或调用入口。

验收批次有效行数：Files 470、证据 repository 178、service 130、operations 234、code 183、release 222、反馈冻结 73；项目证据用例 208、共享夹具 137、发布用例 181、目录联接用例 208。所有本批修改文件在 500 行以内，未修改历史超限 baseline。29 个源码及 2 份计划/记录通过严格 UTF-8、无 BOM、无尾随空白检查，包含未跟踪的新测试文件；限定范围 diff 检查通过，证据为 `f1-evidence-source-check.log`、`f1-evidence-diff-check.log`。

游戏导出批次有效行数：导出准备 424、项目导出用例 190、桌面游戏运行时 201、导出示例 126。9 项定向用例未调用真实 Godot 导出；生产项目路由及导出回执归属仍待整体接入。首次桌面编译发现句柄局部变量加到了相邻播放入口，已移回导出入口并重新编译通过；播放入口保持原有逻辑。未构建或启动新 EXE。

观察截图批次有效行数：Files 422、任务路径 87、截图引用 241、资产 agent 208、桌面预览 149、项目截图用例 248、目录联接用例 63。37 项定向测试及桌面编译通过，未运行引擎或 EXE 验收；该批搬动用例证明冻结截图、反馈及后续模型图像可读取。本轮相对位置批次另行覆盖新项目任务工作副本的重解析；检查点与完整会话索引仍由各自证据和后续迁移处理。独立只读核验提出按历史记录路径清理的建议；对照改动前实现后未采纳，继续只删除任务拥有的规范文件，避免扩大删除权限。

检查点批次有效行数：任务路径 139、检查点保存/保留 109、Blender 会话 382、会话注册表 90、调度 worker 130；项目检查点用例 177、保留用例 126、目录联接用例 98。首次新增用例失败源于 Windows 修改文件时间需要写权限，以及反馈种子尚未应用时 checkpoint 为空；仅修正夹具和断言，随后 30 项资产协议用例全部通过。另有 5 项相关会话/制造用例通过，没有调用真实引擎或扩展到全流程。

独立只读核验提出的路径别名建议未采纳：现有入口比较规范目标后返回经过布局检查的标准路径，Client 已先规范化响应，并不沿调用方别名继续读取。会话并发领取仍由现有调度器及 Sessions 预留管理，本批未改变该机制；多项目路由需要统一核验 Store 与 Files 的来源和领取身份，不能仅凭 Session 持有 Files 宣称调度切换完成。11 个源码和 2 份计划/记录纳入严格 UTF-8、BOM、尾随空白及限定范围 diff 检查，证据为 `f1-checkpoint-source-check.log`、`f1-checkpoint-diff-check.log`。

项目相对工作区批次证明，新建项目任务保存 `.beaver/workspaces/<task-id>`，关闭并搬动项目后可由新项目根恢复任务工作副本、资源读取和 Codex HOME 准备。该证据不涵盖生产 Store/Files 路由、宿主项目登记重定位或完整 Codex 会话索引转换；历史绝对工作区记录仍需在迁移副本中显式转换。本批没有运行全流程、真实 Godot/Blender 或 Beaver.exe 验收。

项目相对工作区批次有效行数：任务路径 148、任务创建 323、任务计划 270、资料 174、任务完成 184、启动准备 101、HOME 准备 412；项目任务用例 273、工作区归属用例 161、交付共享夹具 118、项目资源用例 160、框架用例 231、存储链接用例 208、项目 HOME 用例 301、工作区读取用例 146、验收工作区用例 82。全部低于 500 行并保持既有职责；没有更新历史超限 baseline。

生产创建与首批路由批次接入以下边界：

- `create_project` 写入 Godot 模板和生成项目 ID 后，先调用 `ProjectStore::initialize` 创建 `.beaver`、清单和 `project.sqlite`，并把项目实体写入项目本地 Store；只有这些步骤成功后才登记宿主 Store。初始化未完成的项目不会成为可打开的宿主登记。
- 桌面 `project.create` 创建完成后立即通过统一的 `ProjectStorageRouter` 打开项目；`project.npr.install` 和所有 `workflow.*` 操作同样要求已登记的项目本地存储成功打开，不再静默退回宿主 Store。
- NPR 项目元数据更新先写项目本地权威 Store，成功后再更新临时宿主登记副本；运行时与实体项目 ID 不一致时拒绝写入。
- 桌面 `task.create` 现在要求目标项目已由 `ProjectStorageRouter` 打开，使用项目运行时的 Store 和 Files 创建任务，工作区记录为 `.beaver/workspaces/<task-id>`，并在成功后写入任务路由索引；目标项目未打开时明确失败，不再写入宿主 Store 或宿主 `workspaces/`。
- 定向验证通过：项目创建用例 1 项、`project_storage_routing` 集成目标 9 项、`data_dispatch::tests::create_project_task_*` 2 项、`beaver-desktop --tests` 编译检查。未运行完整验收或启动 Beaver.exe。
- `objectFramework.status` 的项目级读取边界已补齐：已打开项目从项目 Store 返回 `detected` 状态、项目标识、固定 capability/blocker；即使宿主同 ID 记录指向其他路径，也不会读取宿主记录。已登记但未打开的本地项目明确返回“项目本地存储未打开，禁止回退到宿主存储”。新增 2 项路由测试通过。
- `logs.query` 的项目级读取边界已补齐：当请求携带 `projectId` 且项目已打开时，调用日志查询只读取项目 Store；宿主 Store 中同项目 ID 的记录不会混入项目结果，直接查询宿主仍只返回宿主记录。新增 1 项路由隔离测试通过。
- `project.blueprint.save` 的项目级写入边界已补齐：已打开项目通过项目 Runtime 保存 blueprint，读取项目 Store 的当前项目版本并递增 revision；即使宿主 Store 中存在同 ID 的影子项目，保存也不会污染宿主记录。新增 1 项路由隔离测试通过。
- 自动验收修复任务的桌面路由已补齐：验证服务在状态变更回调中重新打开已登记项目并刷新 `ProjectStorageRouter.task_projects`，因此由项目 Store 创建的修复任务可以在调度器唤醒和 UI 事件通知前进入任务索引；核心验证层仍不依赖桌面路由器。新增 `validation_runtime` 定向回归覆盖“项目打开后追加任务，刷新后可按任务找到项目 Runtime”，并与两项反馈任务索引测试一起 3 项通过。
- 验证媒体的宿主回退边界已补齐：当验收 run 属于已登记且存在 `.beaver` 的项目时，未打开项目 Runtime 不再静默读取宿主同 ID 的 `validationRun` 或证据目录；明确返回项目本地存储未打开错误。没有 `.beaver` 的旧宿主项目仍保留媒体读取兼容路径。新增 2 项 `validation_runtime` 定向测试通过。
- Scheduler 成功收尾边界已补齐：项目 Runtime 领取的验证型任务在 worker 完成后只在项目 Store 写入 `completed` 状态、状态事件和 operation；宿主 Store 中同 ID 的影子任务保持 `queued`，且不产生项目任务的 operation。新增 `completed_project_task_finishes_only_in_project_store` 回归测试通过。
- `game.verifyExport` 调用日志的项目归属已补齐：该方法输入只有导出 `path` 时，`call_log_context` 在分发前复用 `GameStorage` 的导出路径选择逻辑解析项目，调用记录直接写入项目 Store，宿主 Store 不再出现同一条记录；已登记但未打开的本地项目会被打开而不是回退宿主；显式 `projectId` 与路径归属不一致时拒绝；路径不匹配任何项目 delivery 时保持宿主上下文。新增 `business_routing::tests::export` 4 项定向测试，`business_routing` 共 19 项通过。
- 验证型任务收尾的项目隔离已有回归：`task_completion` 的自动修复子任务、补充要求子任务、集成完成和交付覆盖记录只写入项目 Store，宿主 Store 中同 ID 影子任务和另一个项目中同 ID 任务均保持原状态，事件不落宿主，项目重开后子任务与事件仍可读。新增 `native/core/tests/project_validation_isolation.rs` 3 项通过。
- 任务数据分发路由已经收敛到共享选择器：删除 `data_dispatch_tasks.rs` 内重复的 `TaskStorageHandles`/`task_storage_handles`，并让共享 `TaskRuntimeHandles` 显式返回 `project_routed`。关联任务、任务设置、合并重试、接受/回滚、资源、事件、回调状态和 reveal 共 8 个入口使用相同的项目优先、关闭本地 Runtime 禁止宿主回退及旧宿主兼容规则；项目关联任务成功后继续更新 Router 索引。定向验证为共享路由 3 项、关联任务 3 项、关闭 Runtime 禁止回退 1 项，均通过；`cargo check --locked -p beaver-desktop --tests`、`cargo fmt --all -- --check` 和有效行检查通过，结构结果为 660 个源码、18 个未变动历史文件、0 个违规。证据为 `f1-task-routing-*.log`。
- 项目 Runtime 的首次打开恢复已经集中到 `project_runtime_lifecycle`。`ProjectStorageRouter::open_registered_with` 在项目注册表锁内创建 Runtime，依序执行 `Journal::recover`、`Store::recover_tasks`、Validation 恢复和 setup 恢复，随后读取任务并一次性发布 Runtime 与任务索引；初始化、任务读取、任务 ID 归属检查或索引更新失败时，新 Runtime 会被关闭且不会被其他调用观察，后续可以重试。已经打开的 Runtime 不重复执行恢复。
- 动态打开调用面已经统一使用上述生命周期入口：启动宿主恢复、Scheduler 每轮发现、项目创建与导入、任务/Validation/Workflow 路由、GameStorage 项目解析和调用日志复制都会先完成恢复再取得 Runtime。状态聚合仍只遍历已经打开的 Runtime，不会因读取状态而隐式打开项目并绕过恢复顺序。定向验证通过核心路由 15 项、导入恢复 1 项、创建/导入调用日志复制 2 项、GameStorage 5 项、Workflow 1 项、Validation 路由刷新 1 项、导出调用日志 4 项和 Scheduler 动态发现 1 项；`cargo check --locked -p beaver-desktop --tests`、`cargo fmt --all -- --check` 及有效行检查通过，结构结果为 661 个源码、18 个未变动历史文件、0 个违规。证据为 `f1-runtime-recovery-*.log`。
- 状态聚合的宿主影子边界已补齐：`data_dispatch_state::state_operation` 仍只消费已经打开的 Runtime，但在合并宿主记录前会先确定“由项目存储拥有任务”的项目集合，即已打开 Runtime 的项目，以及宿主已登记且存在实际 `.beaver` 目录的项目。这些项目在宿主 Store 中的影子任务不再进入 `state` 响应；Runtime 关闭后项目任务不会静默回退到宿主影子记录，宿主登记的项目实体本身保持返回，没有 `.beaver` 的旧宿主项目和任务保持兼容，读取状态不会打开 Runtime，也不会绕过 journal、任务、Validation 和 setup 的恢复顺序。新增 2 项 `data_dispatch::tests::state_tests` 回归（登记未打开项目隐藏影子任务且不隐式打开、Runtime 关闭后不回退宿主影子任务），与原有 2 项一起 4 项通过；`cargo check --locked -p beaver-desktop --tests`、`cargo fmt --all -- --check` 和有效行检查通过，结构结果为 661 个源码、18 个未变动历史文件、0 个违规。真实生产分派 `data_dispatch::call("state")` 直接调用同一函数；完整 `Backend` 依赖 tauri `AppHandle` 构造 Scheduler 与验证服务，无法在单元测试中搭建，因此分派层只保留人工核对，不伪造回归。
- 关闭/重开生命周期已有回归：`project_runtime_lifecycle` 覆盖“Runtime 仍被持有时拒绝关闭、释放后关闭移除任务路由、`open_registered_local` 重开时再次执行恢复（运行中任务改为 `interrupted`）并重建任务索引、重复枚举不重复打开”；`state_tests` 覆盖“关闭后状态不含项目任务、重开后项目任务重新出现”。本批 `state_tests` 5 项与生命周期 2 项共 7 项通过，`cargo check --locked -p beaver-desktop --tests`、格式和有效行检查通过，结构结果为 661 个源码、18 个未变动历史文件、0 个违规。证据为 `f1-state-aggregation-*.log`。
- 项目注销的 Runtime 清理已补齐：`ProjectStorageRouter::close_unregistered` 对比宿主登记与已打开 Runtime，关闭登记已被移除的项目并删除其任务路由；仍被活动引用持有的 Runtime 保留并在返回值 `UnregisteredClose.retained` 中报告，下一轮再试。`project_runtime_lifecycle::open_registered_local` 现在先执行该清理再打开新登记的本地项目，因此 Scheduler 每轮发现、Validation 枚举和启动恢复都会让注销项目从任务枚举和任务路由中消失；已登记项目不受影响，注销不会删除项目本地 `.beaver`。业务 API 目前没有 `project.remove` 一类入口，本批只提供 Router 与生命周期层的语义，未新增 UI 或业务方法。新增核心路由 2 项（注销关闭与保留登记项目、活动引用保留并延后关闭）和桌面生命周期 1 项（注销后枚举与任务路由消失、`.beaver` 保留）；核心路由 17 项、桌面生命周期与状态 8 项、`task_runtime`/`game_storage`/`validation_runtime`/`workflow_runtime`/`business_routing` 33 项通过，`cargo check --locked -p beaver-desktop --tests`、格式和有效行检查通过，结构结果为 661 个源码、18 个未变动历史文件、0 个违规。证据为 `f1-unregister-*.log`。
- `objectFramework.status` 的路由事实已补齐：`object_framework_status::status_with_routing(store, project_id, routed)` 由桌面分派传入 `query_runtime_handles` 的 `project_routed`，只有通过 `ProjectStorageRouter` 取得项目自身 Store 且清单状态为 `detected` 时，响应的 `storage.routed` 才为 `true` 并去掉 `PROJECT_STORAGE_NOT_ROUTED` 阻塞；探测本身不打开 Runtime，`offline`/`legacy`/`invalid` 项目即使传入 `routed` 也保持 `false`。`objectsRead`/`manufactureRead`/`execution` 仍为 `false`，`OBJECT_QUERIES_UNAVAILABLE` 与 `OBJECT_FRAMEWORK_DISABLED` 保留。TS 契约 `objectFrameworkStatusSchema.storage.routed` 为带默认值 `false` 的布尔，旧响应兼容。核心状态 2 项、桌面 `business_routing::tests::storage` 7 项、`tests/object-framework.test.ts` 4 项通过，`npm run typecheck`、`cargo check --locked -p beaver-desktop --tests`、格式与有效行检查通过。证据为 `f1-status-routed-*.log`。

### 2026-09-21：注销后保留 Runtime 的调度边界

前一批注销清理只覆盖可立即关闭的 Runtime。活动引用阻止关闭时，Runtime 仍会出现在枚举中，原 Scheduler 会继续执行计划协调并领取其排队任务。本批为 `TaskRuntime` 增加 `draining` 状态，桌面 `project_runtime_lifecycle::scheduler_runtimes` 将已移除宿主登记但仍被持有的 Runtime 标记为 draining。该状态保留原 Store/Files、项目所有权、运行任务的并发占用和控制路由，但跳过计划协调及新任务领取；宿主中相同项目或任务 ID 的影子记录继续被抑制。

重新登记同一项目后，新枚举恢复正常领取，不重复恢复正在运行的任务。引用释放后沿用 Router 的延后关闭逻辑，移除任务索引并保留 `.beaver`。活动 worker 的收尾继续使用其已持有的原 Runtime。此处不新增注销业务 API，不覆盖 Runtime 完全关闭后的宿主孤立历史任务处置，也不改变 Validation 的枚举策略；不能把此结果解释为所有注销入口已经完成。

本批定向验证：新增核心 draining 用例 2 项、现有 Scheduler 用例 13 项、桌面生命周期用例 4 项全部通过。覆盖停止领取、保留宿主影子抑制、恢复领取、排队任务中断路由、停止父子计划协调、重新登记不重复恢复、释放后关闭和本地文件保留；现有 Scheduler 用例另覆盖活动任务在 Runtime 移出可见列表后仍在原 Store 收尾。尚未增加真实 worker 与宿主注销组合的端到端测试。`cargo check --locked -p beaver-desktop --tests`、`cargo fmt --all -- --check` 和 `npm run check:effective-lines` 通过；结构结果为 662 个源码、18 个未变动历史文件、0 个违规。改动源码通过严格 UTF-8、无 BOM、尾随空白及限定 diff 检查，未更新 baseline。证据为 `output/f1-draining-{core-tests,scheduler-tests,desktop-tests,desktop-check,format-check,structure}.log`。版本仍为 `0.1.19.36`，未启动 Beaver.exe 或运行完整验收。

### 2026-09-21：Validation 排空与活动验证存储持有

Validation 的 Storage 枚举现将注销后因活动引用保留的项目标记为 `draining`，队列领取与 code lane 的 coordinator 跳过这些项目，但保留其项目 ID 以抑制宿主影子验证记录。重新登记后的枚举恢复领取。领取返回原 Storage，worker 执行、进度保存、视觉比较和最终结果持久化均使用该句柄，避免执行或收尾时再次解析已注销项目，也确保原 Store/Files 在整个活动验证期间保持被持有。

`cargo test --locked -p beaver-core --lib validation::jobs` 的 4 项测试通过，新增项目本地 Store 回归覆盖 draining 禁止领取、宿主影子抑制、恢复领取，以及 resolver 已失效后仍使用原 Store 执行准备和保存结果。该用例未运行真实 Godot，也未证明注销与领取的原子互斥；用户注销 API、完全关闭后的宿主孤立记录、Validation 回调的任务路由刷新及真实 worker 组合仍需继续核对。编译与结构证据见 `output/f1-validation-lifecycle-{tests,check,structure}.log`。本批不发布可执行文件，版本保持 `0.1.19.36`。

### 2026-09-21：导入同 ID 副本的登记冲突

核对发现 `project.import` 在不同路径读到相同本地项目 ID 时，原实现会覆盖宿主登记；这会在已有 Runtime 仍指向原路径时形成登记与运行时不一致。现在先按清单 ID 检查宿主登记，在打开副本数据库或写宿主记录之前拒绝冲突，要求显式重新关联或派生身份。原登记路径离线也不能视作自动覆盖授权；同一路径重导入仍保留既有语义。

`cargo test --locked -p beaver-core --lib projects::` 4 项通过，新增回归覆盖原目录在线及移动后离线两种冲突，验证宿主记录、副本清单及历史任务保留。同路径重导入也通过。首轮新增测试使用错误清单文件名失败，改用 `layout::MANIFEST` 后重跑通过。`cargo check --locked -p beaver-desktop --tests`、Rust 格式和有效行数检查通过：664 个源码、18 个未变动历史文件、0 个违规。证据为 `output/f1-project-identity-{tests,check,structure}.log`。本批仅补齐冲突拒绝，显式重新关联、派生身份及其 API/UI 仍未实现，F1.4 保持未完成。

### 2026-09-21：显式项目重新关联 API

新增公开业务方法 `project.reassociate`，参数为 `id`、`expectedPath`、`path`。调用方必须提供当前宿主登记路径；服务端拒绝过期登记、目标身份不匹配、其他项目已占用的目录及应用内部数据目录。存在活动 Runtime 引用时拒绝切换；成功时保留宿主其他元数据，更新登记路径，关闭旧 Runtime 并清除旧任务路由，下一次正常打开仍执行既有恢复流程。移动后的项目可以重新关联，不改写本地项目实体、清单或历史任务，也不删除原目录。业务入口已接入命令目录及状态变更通知，UI 尚未接入。

独立审查发现先关闭 Runtime 再写宿主登记会在数据库写入失败时留下失效路由。本批改为持有注册表锁预检引用，宿主写入成功后才关闭和清理。3 项核心回归通过，覆盖旧路径并发检查、目标身份、活动引用拒绝、移动恢复、本地实体保留，以及 `PRAGMA query_only` 注入写失败后原 Runtime/路由保留和重试成功。桌面 tests 编译通过，保留现有未使用字段和可变性警告。测试及编译日志为 `output/f1-reassociation-{tests,check}.log`；格式与结构日志为 `output/f1-reassociation-{format,structure}.log`。本批未运行完整验收或交付可执行文件，F1.4 保持未完成。

### 2026-09-21：Runtime 关闭后的宿主历史任务隔离

Scheduler 现在只协调和领取仍有宿主项目登记的宿主任务，同时继续排除本地 Runtime 的影子记录。所有权过滤发生在父子计划协调之前，避免已注销项目的宿主历史父任务被自动完成或扩展。项目 Runtime 的 draining 与全局并发额度保持原有语义。桌面任务路由在保留 Runtime 完全关闭后拒绝回退到未登记项目的宿主任务；活动引用持有的原项目路由仍可用于收尾。历史任务和项目文件均保留。

核心 Scheduler 16 项、桌面 business routing 20 项定向测试通过；旧项目兼容夹具补齐真实登记，继续验证有效旧项目可使用宿主存储。领取测试提取到 `scheduler_claim_tests.rs`，共用原测试模块的运行时夹具；移除已不需要的调度测试超长文件例外，未更新 baseline。Rust 格式检查通过；有效行检查为 667 个源码、18 个未变动历史文件、0 个违规。证据为 `output/f1-orphan-{scheduler-tests,routing-tests,format,structure}.log`。

本批尚未实现公开注销 API、Validation 完全关闭后的孤立记录隔离，以及登记移除与领取之间的并发同步；不能据此宣布完整注销生命周期完成。F1 子项保持未勾选，未运行完整验收或构建发布包。

### 2026-09-21：Validation 宿主孤立记录隔离

Validation 的宿主领取与后台协调现在使用同一项目归属集合：只处理仍登记且不属于已枚举本地 Runtime 的项目；保留的 draining Runtime 仍抑制宿主影子记录。领取写回 running 前，在原 Store 锁内再次核对登记与记录状态。coverage、feedback 及自动修复均先过滤归属，避免孤立记录被改写或生成后续修复。已开始的验证继续持有原 Storage 收尾。

桌面验证存储 resolver 和宿主媒体回退均拒绝未登记项目；有效 legacy 项目仍允许宿主存储，本地 Runtime 保持优先。核心 jobs 定向测试 5 项、桌面 validation runtime 7 项通过，覆盖孤立 queued/failed run、coverage/feedback 保留、有效 legacy 协调/领取以及未登记媒体回退拒绝。Rust 格式、有效行和限定 diff 检查通过；源码有效行数为 jobs 225、coordinator 112、automatic repair 60、生命周期测试 144、桌面 runtime 499，未修改 baseline。证据为 `output/f1-validation-orphan-{tests,desktop,format,structure}.log`。

本批未实现公开注销 API 或枚举与登记移除之间的统一同步协议；现有检查不能证明并发注销生命周期已经完整闭合。F1 仍在进行中，未运行全流程验收或发布构建。

### 2026-09-21：领取写入前重验与 Runtime 清理登记锁

Scheduler 在最终 Store 锁内重读候选任务，重新核对 queued 状态、项目归属、waitingDelivery、依赖和迁移保留标记；宿主任务同时核对当前项目登记，再使用当前记录准备和写入领取结果。候选读取之后被中断、删除、转移或失去登记的任务不会按旧快照恢复运行，期间更新的任务内容也不会被旧快照覆盖。

`close_unregistered` 按项目注册表、宿主 Store、任务索引的固定顺序持锁，在同一临界区读取登记、关闭无活动引用的 Runtime 并清理路由。登记核对不再发生于获取注册表锁之前；任务索引锁失败也不会留下已关闭 Runtime 的部分结果。活动引用仍保留原 Runtime 供收尾。

Scheduler 定向测试 18 项、项目存储路由 17 项通过；新增用例覆盖选取后登记移除、任务状态/归属/依赖变化、任务删除及当前内容保留。Rust 格式和有效行检查通过：668 个源码、18 个未变动历史文件、0 个违规，未更新 baseline。证据为 `output/f1-claim-revalidation-tests.log`、`output/f1-close-registration-tests.log`、`output/f1-claim-revalidation-format.log` 和 `output/f1-claim-revalidation-structure.log`。

本批只收紧宿主 Store 内领取及 Runtime 清理边界。项目 Runtime 的 draining 仍是枚举快照，枚举后注销与本地领取之间尚需统一同步；公开注销 API 继续待实现，F1 保持未完成。未运行全流程验收或发布构建。

### 2026-09-21：Scheduler 项目登记实时同步

新增 `ProjectWorkGate`，使用共享宿主 Store 锁将项目登记变更与新工作准入串行化。桌面 Scheduler 枚举出的每个本地 Runtime 均绑定该项目的 gate；父子计划协调和最终领取分别重新获取 permit，并在项目 Store 写入完成前持有。注销前生成的非 draining 快照现在也会根据当前登记拒绝协调和领取。permit 在返回领取结果前释放，外部 worker 和控制操作仍使用原 Store 收尾，不持有登记锁。

新增回归验证旧枚举快照在注销后不改变 queued 任务或 waitingChildren 父任务、活动任务仍可定位控制、重新登记后恢复工作，以及已领取任务在注销后仍持有原 Store。独立 gate 用例验证 permit 持有期间登记锁不可写、旧 clone 每次重读登记。Scheduler 19 项、gate 1 项、桌面项目生命周期 4 项定向测试通过；Rust 格式、限定 diff 和严格 UTF-8/BOM/尾随空白检查通过。有效行检查为 670 个源码、18 个未变动历史文件、0 个违规；gate 54 行、Scheduler runtime 60 行、调度操作 391 行、新回归 108 行、router 352 行、桌面生命周期 251 行，未更新 baseline。

证据为 `output/f1-scheduler-registration-gate-{tests,desktop,format,structure}.log` 和 `output/f1-project-work-gate-tests.log`。Validation 尚未接入该实时同步机制，公开注销 API 仍待实现；不能将 Scheduler 的定向证明扩大为完整注销生命周期完成。下一步为 Validation 协调和领取接入同一 gate，再实现注销业务入口。未构建发布包或运行全流程验收。

### 2026-09-21：Validation 项目登记实时同步

Validation 的项目 Storage 现绑定同一 `ProjectWorkGate`。候选扫描、最终领取和协调均按宿主登记锁、项目 Store 锁的顺序获取锁，写入完成前保持 permit；旧的非 draining 枚举快照在登记移除后不能继续领取或协调。宿主兼容存储使用默认 gate，继续在自身 Store 锁内核对登记与本地影子归属，避免重入锁。

已领取验证继续使用原 Store/Files 执行和保存，工具解析与 changed 回调均在 permit 释放后执行。新增回归覆盖旧快照拒绝、重新登记恢复协调和领取、领取后注销仍保存到原项目，以及宿主影子保持 queued；工具解析同时检查宿主与项目 Store 可独立获取锁。桌面验证测试提取到独立文件，保留原测试模块路径，避免生产文件超过有效行限制。

核心 Validation jobs 6 项、桌面 validation runtime 7 项定向测试通过；Rust 格式、有效行、限定 diff 和严格 UTF-8/BOM/尾随空白检查通过，未更新 baseline。证据为 `output/f1-validation-registration-gate-{tests,desktop,format,structure}.log`。独立只读审查未发现本批锁顺序或 permit 生命周期问题。公开项目注销 API 仍待实现，F1 保持进行中；未构建发布包或运行全流程验收。

### 2026-09-21：公开项目注销 API

新增 `project.unregister {id, expectedPath}`，仅删除宿主项目登记，返回 `id`、原 `path` 和 `draining`。接口按注册表、宿主 Store、任务索引顺序持锁，核对当前登记路径后先提交宿主删除；失败时保留原 Runtime 与任务路由。宿主锁同时排斥 Scheduler/Validation 的新工作准入。无活动引用时关闭 Runtime 并清理路由，有引用时保留原存储供收尾，由后续枚举继续清理。离线项目无需访问磁盘即可注销，项目文件、本地实体和宿主历史记录均不删除。

桌面 catalog、参数验证、mutation 通知和 Scheduler wake 已接通。核对公开入口时发现登记操作的调用日志此前会要求项目 Runtime，导致离线重新关联无法到达业务入口，也会额外持有 Runtime 引用；`project.reassociate` 与 `project.unregister` 现将登记操作日志写到宿主，保留项目 ID 上下文及身份冲突检查，不打开或持有项目 Runtime。

核心注销 3 项及重新关联 3 项回归通过，覆盖旧路径拒绝、宿主写失败、活动工作收尾、释放后的路由清理、文件/历史保留和离线兼容项目；桌面登记日志路由 1 项通过，验证离线可达且不阻止 Runtime 关闭。桌面 tests 编译、Rust 格式和有效行检查通过。证据为 `output/f1-project-unregister-{core,desktop,routing,format,structure}.log`。独立只读审查未发现本批正确性问题。F1 仍需用户界面、离线诊断、派生身份、迁移入口与剩余权威存储核对；未运行原生全流程验收或发布构建。

## 6. 尚未完成与下一入口

1. F1.1：新建空白项目、首批 `project.npr.install`、`workflow.*` 和 `task.create` 生产路由已经接入。Scheduler 已完成任务领取/收尾的运行时归属传递、项目 Runtime 优先、宿主影子任务抑制、运行中 Runtime 移除后的原 Store 收尾、RuntimeSource 失败清理、未知任务不匹配 Runtime，以及会话关闭不依赖 Runtime fallback 的定向边界修正；验证服务的自动修复任务也已在状态变更时刷新桌面任务索引。Scheduler RuntimeSource 现在会在每次枚举前自动打开所有已登记的本地 `.beaver` 项目，因此运行期间新导入的项目可以进入同一个全局调度器，旧的宿主项目仍保持兼容。动态首次打开会先完成 journal、任务、Validation 和 setup 恢复，再发布 Runtime 与任务索引。任务数据分发、验证、回调、任务框架、控制、资产、调用日志和事件入口已经使用共享项目路由边界。状态聚合已排除本地项目的宿主影子任务且不隐式打开项目；关闭/重开已有恢复回归；宿主登记移除后的 Runtime 关闭与任务路由清理已进入枚举入口。仍需完成全局调度器和 worker 的其余多项目生命周期、面向用户的项目注销/重新关联业务入口、journal 的剩余权威边界、导出回执及剩余直接 SQL 的权威 Store 核对。共享句柄、项目快照、上述文件入口、验收证据、游戏导出准备、观察截图、检查点、执行/读取工作区归属校验和新项目任务的相对工作区记录已适配。
2. F1.2：完整归档文件清单、已知实体/blob/文件/会话索引路径检查，恢复副本内的项目分库转换（历史绝对工作区/检查点/截图记录改写、会话索引路径改写、未归属内容显式保留并写入回执），以及启用时宿主 `settings`/`toolSetup` 的再绑定和保留任务的 `migrationRetained` 隔离已实现；仍需补齐 Codex HOME 内宿主生成配置的重建、其余真实上下文内嵌引用、根元数据处理和保留项的显式转换/处置入口，不能据此宣告完成。
3. F1.3：`partition-projects` 提供分库、路径转换、逐文件核对和分库回执；`activate-import` 现能识别分库副本，`stage` 完成分区校验、宿主登记切换、凭据转换、宿主/分区任务中断和文件日志恢复，`activate` 在工具检测后写回执并移除 pending 标记，源数据只读核对，无双写。仍需桌面侧的迁移入口、启用后的路由接入回归和真实工具环境下的启用验证。
4. F1.4：显式重新关联 API 已接入；仍需离线诊断、派生独立身份及相应 UI 接入。

下一步在统一任务路由、首次打开恢复、状态聚合影子边界、关闭/重开回归和注销后 draining 边界之上，接入面向用户的项目注销/重新关联业务入口，并核对注销与真实 worker 收尾的组合、Runtime 完全关闭后的宿主孤立历史任务以及 Validation 生命周期；继续 journal 的剩余权威边界与迁移启用后的路由接入。状态读取入口继续只消费已打开 Runtime，不能自行打开项目而绕过 journal、任务、Validation 和 setup 的恢复顺序。保持单一宿主并发额度，不能按项目复制全额 Scheduler。任务写入、调度、运行时和恢复必须同步使用同一项目权威位置。普通项目导入已经覆盖恢复回归，迁移副本的桌面启用入口仍保持关闭。

本轮未构建或启动新 Beaver.exe，版本保持 `0.1.19.36`；未修改已确认 UI，未开发 NPR 小阶段或工具调用链。原生运行窗口、实际迁移启用及用户验收均未完成。开发阶段运行 Scheduler 定向测试 13 项（包含新增成功收尾用例）、`business_routing` 19 项、`project_validation_isolation` 3 项、迁移分库/启用定向用例 7 项（含新增启用 2 项）、`cargo check --locked -p beaver-core`、`cargo check --locked -p beaver-desktop`、格式、有效行数和 diff 检查，未运行全流程验收。本轮任务路由收敛另运行共享路由 3 项、关联任务 3 项、关闭 Runtime 禁止回退 1 项和 `beaver-desktop --tests` 编译检查，全部通过；首次打开恢复批次另运行核心路由 15 项及 8 组桌面定向目标，随后通过 `cargo check --locked -p beaver-desktop --tests`、格式检查、严格 UTF-8/BOM/尾随空白检查、限定 diff 检查和有效行检查。桌面编译保留仓库已有的 3 个未使用字段或可变性警告；有效行数检查为 661 个源码、18 个未变动历史文件、0 个违规，未修改 baseline。

## 2026-09-21：派生组装副本的可恢复显式启用

新增 `project_derivation_assembly::activate/inspect_activated`。启用前重新执行组装回执、准备来源摘要、最终绝对路径、文件清单、项目清单及数据库完整性核验；随后以独立的 `ASSEMBLY-ACTIVATION.json` 记录 `beaver-project-derivation-activation-v1` 回执，先创建并同步回执，再删除 `.beaver-migration-pending`。因此 pending 删除是副本从不可见状态发布为普通项目的唯一边界，回执写入或删除标记失败均可重试，重复启用幂等且不覆盖既有回执。已启用副本仍要求准备来源未发生变化，移动后拒绝核验；启用不修改宿主登记，回执明确 `host_registration_changed: false`。5 项组装测试通过，覆盖成功启用、普通 `ProjectStore::open`、已启用核验、幂等调用及回执已写但 pending 尚存的恢复。宿主登记切换/冲突检查、独立 API/UI 和 F1 整体验收仍待完成。

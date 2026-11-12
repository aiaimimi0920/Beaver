# Beaver Cloudflare 中转测试服务

这是独立于旧 `protocol/` 和 `sites-overlay/` 的可部署 Worker + D1 实现。
实际消费者是 `client.mjs`：保持 Beaver headless Core 存活，将外部任务上下文发布到云端，
接收一条结构化命令，通过现有 `external.*` API 执行，再将 Core 回执写回云端。
不依赖 Slack，不调用付费模型 API，也不冒充 Dot 登录接口。

## 当前验收与后续计划

- [x] Worker/D1 持久化请求、不可变回复和执行回执；Beaver/Dot 双角色鉴权。
- [x] 提供 REST 与最小无状态 JSON MCP 接口，以及保持 Core 存活的多轮 CLI 消费者。
- [x] 部署独立测试资源，在全新空工程验证真实云端到真实 Core 的文件写入、读取与回执往返。
- [x] 测试服务 OAuth 授权码 + PKCE、浏览器授权页、短期令牌和撤销；已通过线上 HTTP 协议验证，见 [OAuth 使用说明](OAUTH.md)。
- [ ] 真实 Dot 侧连接授权和兼容性验证。服务端 OAuth 可用不代表真实 Dot 已完成登录。
- [ ] 真实 Dot 自动触发：当前没有事件推送或唤醒接口，MCP 工具本身不等于自动唤醒。
- [ ] Desktop 连接管理 UI、多 Dot 身份和多租户隔离；当前是单用户测试通道。

本轮验收只覆盖消息中转后端 slice，不表示整个 Dot 集成完成，也没有生成新的桌面 Beaver.exe。

## 已部署资源（2026-10-23）

- Worker / D1 名称：`beaver-dot-relay-test-20261023`
- 服务：`https://beaver-dot.aiaimimi.com`
- MCP：上述地址追加 `/mcp`
- D1 ID：`41455374-664e-43cb-8952-d471b90b3f96`
- 初始部署 ID（历史记录）：`626933a6c8c844b6b12b2f97cdacd335`

当前连接入口使用自定义域名，OAuth issuer 同步指向该域名。旧 workers.dev 入口暂时保留兼容，
不作为新连接地址；迁移后的 OAuth 验证证据见 [OAuth 使用说明](OAUTH.md)。

用户已批准新建测试服务；没有覆盖原有服务。部署仅包含中转实现，测试上传的是新建空工程的
合成任务上下文，不是用户现有项目源码或历史任务。后续使用 CLI 时会上传该轮完整
`external.context`（包含任务信息、工作区路径和指令），使用前应确认允许发送这些数据。

`wrangler.jsonc` 绑定上述独立数据库；`schema.sql` 初始化数据表。Worker 环境需要两个不同的
secret：`BEAVER_TOKEN_SHA256`、`DOT_TOKEN_SHA256`，值为独立随机 Bearer token 的 SHA-256。
未正确配置时拒绝请求。明文 token 不应提交仓库或出现在日志中。

## 协议

| 接口                             | 权限   | 用途                                           |
| -------------------------------- | ------ | ---------------------------------------------- |
| `POST /v1/exchanges`             | Beaver | 发布带 task/run/revision 的请求                |
| `GET /v1/exchanges`              | Dot    | 获取前 20 条待回复请求                         |
| `GET /v1/exchanges/:id`          | 双端   | 读取请求、回复和回执                           |
| `POST /v1/exchanges/:id/reply`   | Dot    | 提交一条不可变命令                             |
| `POST /v1/exchanges/:id/receipt` | Beaver | 写入不可变 Core 回执                           |
| `POST /mcp`                      | Dot    | `beaver_pending`、`beaver_get`、`beaver_reply` |

所有接口要求 `Authorization: Bearer <对应端 token>`。MCP 支持初始化协议版本
`2025-03-26` 和 `2025-06-18`；这是最小 JSON 请求/响应实现，不保证所有客户端兼容。
原有数据接口拒绝带 `Origin` 的浏览器请求。OAuth 授权页及同源确认表单另行处理。
数据消息体上限 512 KiB，OAuth 表单上限 8 KiB。

Dot 回复格式例如：

```json
{
  "method": "external.tool",
  "arguments": {
    "tool": "file.write",
    "arguments": {
      "path": "example.txt",
      "content": "hello",
      "expectedSha256": null
    }
  }
}
```

只允许 `external.submitPlan`、`external.tool`、`external.finish`；具体参数服从 Core 合同。
远端不能指定执行目标。适配器与本地 manifest 比较完整请求，并沿用原 task/run/revision/requestId；
不为旧回复刷新 fence。相同请求的重试幂等，不同内容占用同一 ID 返回冲突。
Core 继续负责权限、revision 和执行幂等。工具执行器属于受信任主体，**不是运行恶意代码的安全沙箱**。

## CLI 使用

要求 Node.js 22+、已构建 `beaver-headless.exe`。从仓库根目录运行：

```powershell
$env:BEAVER_RELAY_ORIGIN = 'https://beaver-dot.aiaimimi.com'
# 从本地受保护凭证来源设置 BEAVER_RELAY_TOKEN，不要在共享日志中打印。
$env:BEAVER_HEADLESS = 'C:/Users/Public/nas_home/Beaver/target/debug/beaver-headless.exe'
rtk node integrations/dot-bridge/cloudflare/client.mjs C:/absolute/new-run-directory C:/absolute/prompt.txt
```

目标目录必须不存在，父目录必须存在。CLI 新建空工程、外部 code 任务，每轮先保存本地 manifest，
再发布请求。Dot 侧需要独立消费者调用 MCP 读任务和回消息；CLI 不会自行启动 Dot。
每 3 秒轮询，最多 20 轮或 30 分钟，收到 finish 或失败回执后停止。
运行期间必须保持同一个 Core host 存活；shutdown/restart 会使旧 run 失效。
目前网络异常会停止，不支持后台自动恢复。执行后 ACK 丢失应以原 requestId 查询/重试，
不能创建新 ID 重放未知执行。数据库没有自动保留期或清理功能，不应发送生产敏感数据。

## 开发验证证据

`relay.test.mjs` 用真实 SQLite 验证 D1 SQL，以 Core stub 验证适配合同，不能替代真实 Core 测试。
`live-proof.mjs` 在实际线上服务中模拟 Dot HTTP 调用方，使用真实 Core：不调用 Dot 或模型。

```powershell
rtk node --test integrations/dot-bridge/cloudflare/relay.test.mjs
rtk cargo test --locked -p beaver-core external_tools::
rtk node integrations/dot-bridge/cloudflare/live-proof.mjs ORIGIN CREDENTIALS_JSON HEADLESS_EXE NEW_DIRECTORY
```

本机明文测试凭证只保存在仓库外的
`C:/Users/Public/nas_home/AI/GameEditor/linshi/beaver-dot-relay-20261023/credentials.json`，
字段 `beaver` 和 `dot` 分别对应两端。不要将其上传或复制到文档。

成功证据：同目录下 `live-02/proof.json`，明确记录 `actualDot: false`、`actualCore: true`。
两轮工具为 `file.write`、`file.read`，均返回 `succeeded`，内容为“Cloudflare 中转验证”，
并校验云端回执和本地文件。测试后中断任务并关闭 Core；未做完整游戏生成验收。
首轮失败证据保留在 `live-01`。失败定位到 Windows canonical drive prefix 不能单独进行
目录元数据查询，已修复 `external_tools_files.rs`，并增加 Windows 路径回归测试；仍检查完整根目录及祖先。

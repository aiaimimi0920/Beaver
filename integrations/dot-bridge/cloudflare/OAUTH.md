# OAuth 测试连接

## 已实现的用户流程

客户端发现 MCP 授权元数据，带 PKCE S256、state 和 resource 打开授权页。
用户检查客户端名称、精确回调地址和权限，输入独立的服务所有者密钥，然后选择授权或拒绝。
授权后客户端以一次性授权码及原 code_verifier 换取 1 小时访问令牌，并访问 `/mcp`。
撤销接口会立即使该令牌失效。拒绝授权不会签发授权码。

这是单所有者测试服务的 OAuth 登录，不是 OpenAI 账号登录、不收集 OpenAI 密码，
不提供用户注册、组织管理或多租户隔离。`beaver:agent` 能读取本测试通道的全部任务并提交执行命令，
因此只能授权受信任的客户端。OAuth 令牌不能使用 Beaver 的任务发布/回执写入接口。
原有两端长期 token 暂时兼容，不受 OAuth 撤销接口影响。

## 连接参数

服务 origin：`https://beaver-dot.aiaimimi.com`

| 参数                                | 值                                                   |
| ----------------------------------- | ---------------------------------------------------- |
| MCP URL / resource                  | origin + `/mcp`                                      |
| 授权服务器发现                      | origin + `/.well-known/oauth-authorization-server`   |
| 资源发现                            | origin + `/.well-known/oauth-protected-resource/mcp` |
| 授权端点                            | origin + `/oauth/authorize`                          |
| Token 端点                          | origin + `/oauth/token`                              |
| 撤销端点                            | origin + `/oauth/revoke`                             |
| ChatGPT 预注册 client_id            | `beaver-chatgpt`                                     |
| client secret / token endpoint auth | 无，public client，`none`                            |
| scope                               | `beaver:agent`                                       |
| grant / response type               | `authorization_code` / `code`                        |
| PKCE                                | 仅 `S256`                                            |

当前采用预注册客户端，不实现 DCR 或 CIMD，不宣称支持它们。
已按官方文档预设稳定回调 `https://chatgpt.com/connector_platform_oauth_redirect`，
但真实连接时必须核对管理页给出的**精确回调地址**；若不同，需要修改
`OAUTH_CLIENTS_JSON` 中对应客户端的精确 allowlist 后部署，不能使用通配符。
真实 Dot 是否提供对应 OAuth 连接配置、是否采用相同回调，尚未验证。
授权响应包含 issuer 标识 `iss`；客户端应验证它及 state。

首次验证使用的所有者密钥保存在本机仓库外：
`C:/Users/Public/nas_home/AI/GameEditor/linshi/beaver-dot-relay-20261023/oauth-owner.json` 的 `owner` 字段。
只在本服务 HTTPS 授权页使用，不要发给 Dot、提交仓库或贴入聊天。
它是高熵随机密钥，不是便于猜测的普通密码。当前没有密码账号系统或登录限流，
不能自行将其换成弱密码后公开使用。

## 实现及恢复边界

- `oauth-common.mjs`：配置、表单边界、PKCE、令牌检查。
- `oauth-consent.mjs`：授权页、同源确认、浏览器 cookie 绑定及所有者校验。
- `oauth.mjs`：发现、授权码兑换、令牌撤销及路由。
- `oauth-schema.sql`：只新增三个表，不覆盖既有 exchanges。
- 必需配置：`OAUTH_ISSUER`、`OAUTH_CLIENTS_JSON`、secret `OAUTH_OWNER_KEY_SHA256`。
  Worker 只存所有者密钥和 bearer token 的哈希；授权码、OAuth 令牌也只存哈希。
- 授权页使用 HttpOnly/Secure/SameSite cookie、禁止 iframe 和外部资源；确认需同源 Origin。
- 授权流程 5 分钟过期；授权码 60 秒过期，原子消费，不能重复兑换。
- 令牌固定 resource/scope/client、1 小时过期，不支持 refresh_token。到期重新授权。
- `POST /oauth/revoke` 为表单：`client_id` 与 `token`；持有者只能撤销匹配客户端的令牌。
- 移除客户端配置立即阻止其令牌访问；轮换所有者密钥不会自动撤销已签发令牌。
- 没有自动清理过期记录、生产级流量限制或后台 Dot 唤醒；当前仍仅供独立测试服务使用。
- 测试客户端 `beaver-relay-proof` 回调为 `https://example.com/beaver-oauth-proof`。
  验证程序从不访问此地址，只读取 303 的 Location；不要在日常连接中使用它。

## 验证

2026-10-23 使用地址迁移为 `https://beaver-dot.aiaimimi.com`，线上 issuer 与本地配置已同步。
旧 workers.dev 入口暂时保留，但不再作为连接配置或 OAuth issuer 使用。
已有客户端应改用新 MCP URL 并重新授权；数据库、任务记录和所有者密钥保持不变。
新域名 HTTPS 与完整 OAuth HTTP 流程证据：
`C:/Users/Public/nas_home/AI/GameEditor/linshi/beaver-dot-relay-20261023/oauth-custom-domain-proof-01.json`。
此验证同样不包含真实 Dot 登录。以下部署 ID 和旧证据属于迁移前的历史记录。

2026-10-23 部署 ID：`556934ad6ebf4faab944e8082dcbb8f5`。
本地 `oauth.test.mjs` 与原 `relay.test.mjs` 覆盖授权、角色隔离、过期、错误 audience、
错误密钥、撤销、PKCE、授权码重放及原消息通道。

线上证据：
`C:/Users/Public/nas_home/AI/GameEditor/linshi/beaver-dot-relay-20261023/oauth-proof-01.json`。
执行了发现、授权页/表单、拒绝、跨站拒绝、错误回调拒绝、PKCE、重复兑换拒绝、
MCP tools/list、角色隔离和撤销。使用 HTTP 模拟客户端，没有真实浏览器交互验收，
没有真实 Dot 参与；证据保留 `actualDot: false`。

复现（输出文件必须不存在）：

```powershell
rtk node --test integrations/dot-bridge/cloudflare/relay.test.mjs integrations/dot-bridge/cloudflare/oauth.test.mjs
rtk node integrations/dot-bridge/cloudflare/oauth-live.mjs ORIGIN OWNER_CREDENTIALS_JSON NEW_PROOF_FILE
```

参考已联网核实的 [MCP 授权规范](https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization)
和 [OpenAI 认证接入说明](https://developers.openai.com/plugins/build/auth)。实现是测试用途的有限子集，
没有通过正式 OAuth 合规认证。下一步是实际客户端配置与手动授权，不能把 HTTP 模拟通过等同真实 Dot 接通。

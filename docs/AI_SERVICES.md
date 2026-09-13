# AI 服务接入

## 两种服务来源，一个本机执行位置

自配模式的六个能力有各自 Base URL、模型和 Key；平台模式共用平台 Base URL/令牌，模型仍按能力选择。媒体路径目前共用能力级路径设置。切换服务来源不会切换成远程桌面或云 Godot。

编码、review 和翻译要求 Responses API；图像和语音使用对应的兼容媒体端点。不要把供应商的网页 URL 或只提供 Chat Completions 的地址当作 Responses API。

Base URL 例如 `https://provider.example/v1`；不能内嵌用户名、密码、查询或 fragment。明文 HTTP 仅允许 localhost/127.0.0.1/IPv6 loopback。Key 保存时留空表示保留已有值；本机免鉴权服务可以不填 Key。

## 从本机 Codex 补齐

自配模式下点击“从本机 Codex 补齐”，显式保存当前设置，并读取当前进程 `CODEX_HOME`（未指定时为用户 `.codex`）中的 API 地址、模型和静态 API Key。只补齐编程、审查、翻译的空白项；不同服务地址、已有模型、已有 Key 和当前输入的新 Key 均不被来源配置覆盖。图像、语音、音乐和平台凭据不自动套用文本服务。

支持 `config.toml` 的模型与 provider、`CODEX_PROFILE` 或配置内 profile，以及 provider 声明的 `env_key`、显式 provider bearer key、`OPENAI_API_KEY` 环境变量和 `auth.json` 的 `OPENAI_API_KEY`。这些是显式静态凭据导入，不是完整 Codex 配置加载器；不复制 `auth.json.tokens` 中的 OAuth 登录信息，不执行凭据命令，不复制个人 MCP、skills、会话或历史。字段语义参考 [Codex 官方配置参考](https://learn.chatgpt.com/docs/config-file/config-reference)。命令动态刷新、其他凭据管理器和额外供应商 headers 需要手动配置兼容网关。

Key 只经主进程写入系统安全存储加密的凭据槽；渲染器收到 `hasKey`，不会收到导入的明文。解析错误不回显配置原文。应用启动不会静默扫描个人配置，也没有内置共享测试 Key。当前开发机按照用户明确请求单独补齐了本机设置，不随发布包传播。

## Codex

Beaver 通过 app-server JSONL 传递目标、停止条件、相对素材路径及归一化问题区域。它自行读取图片、调用工具并编排执行。版本实测基于本机 Codex CLI 0.144.5；升级应先运行 `npm run verify:codex`。

所有任务都有专属 HOME，不覆盖已有 profile 或自定义的 `codex-gd`。内置 Godot/Blender 制作 skills 和 beaver_media MCP；可选启用固定版本 Godot MCP 与 Blender MCP。外部 Blender MCP 仍依赖 uvx 和 Blender add-on server；不启用也可以让 Codex 使用 Blender CLI。

## 媒体 MCP

| 工具 | 参数 | 默认协议 |
| --- | --- | --- |
| `generate_image` | prompt、output、可选 references（最多 5 份） | `POST /images/generations` JSON；带参考用 multipart `/images/edits` |
| `generate_speech` | prompt、output、可选 voice | `POST /audio/speech`，input、model、voice、response_format=wav |
| `generate_music` | prompt、output | **必须显式配置 route**；POST JSON model/prompt |
| `translate_text` | prompt | POST `/responses`；将供应商输出交回 Codex 处理 |

自定义 image route 不会被带参考图分支偷偷改写；自定义服务必须接受对应 multipart 输入。若供应商编辑接口与生成接口是另一组不同路径，应由适配网关统一，或在接入时明确调整路径。

媒体响应支持二进制，或：

```json
{"data":[{"b64_json":"BASE64_BYTES"}]}
```

```json
{"data":[{"url":"https://provider.example/output.wav"}]}
```

异步任务 ID、轮询、供应商专有回调没有被伪装成成功。此类服务需要一个真实适配网关，或后续专用 adapter。输出文件不能已经存在；生成新文件后由 Codex 决定如何替换游戏中的旧素材。生成失败不写入假的占位结果。

## 平台后台尚缺什么

客户端没有内置虚假的账户或余额。要运营平台额度，需要另外部署身份认证、密钥发行、余额/预扣/结算、重试幂等、供应商路由、支付与退款以及滥用控制。当前可配置真实兼容网关，不代表这些运营设施已经实现。

## 验证的真实含义

当前测试使用本机固定 HTTP 应答验证两种能力路由、MCP 请求、媒体落盘和真实 Codex 会话协议。它们**不证明真实供应商账单、真实图像/音乐质量或一次对话完成完整游戏**。接入用户选定的真实服务后，需要从目标任务到素材、试玩、回退、再次修改做完整验收。

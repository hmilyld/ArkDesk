# network-tools 插件 — 开发约定（AGENTS）

> 面向 AI/开发者的本地约定。功能说明见同目录 `README.md`；项目级文档只做引用。

## 工具

`plugin.json` 声明工具（同一插件、前后端同处）：

- `http-client`「接口测试」→ `frontend/views/HttpClient.vue`
- `http-interceptor`「请求拦截」→ `frontend/views/Interceptor.vue`（独立工具，不与接口测试同页耦合）

## 后端

- 命令一律定义在 `backend/mod.rs`（`build.rs` 只扫描该文件，构建期自动登记），
  函数名 = 前端调用名，带 `network_tools_` 前缀、返回 `Result<T, AppError>`。
- 「接口测试」无自有命令：HTTP 走框架 `http_send`，数据走 `@/core/db`。
- 「请求拦截」命令是对 `backend/intercept/` 的薄封装（见下）。
- `backend/migrations.rs` 提供迁移（scope = `network-tools`）；`build.rs` 需发现
  `backend/mod.rs` 才会聚合 `migrations.rs`，故该文件必须保留。

## 请求拦截后端（intercept/）

| 文件              | 职责                                                                           |
| ----------------- | ------------------------------------------------------------------------------ |
| `mod.rs`          | 生命周期（start/stop/status/configure）、事件批量发送、命令实现                |
| `dto.rs`          | DTO（camelCase）+ `ProxyConfig`/`ProxyStatus`/`FlowRecord`/`Ws*`/`CaInfo`      |
| `state.rs`        | 共享状态：流量环形缓冲、WS 记录、配置、事件出口（`Outbox`）                    |
| `handler.rs`      | `HttpHandler`/`WebSocketHandler`：Tee 抓体、请求-响应配对、WS 只读             |
| `body.rs`         | `TeeBody`（透传 + 复制，不破坏流式）、按 `Content-Encoding` 解压、文本化       |
| `ca.rs`           | 根证书生成/加载/导出/信息（rcgen + ring，存 `app_data_dir/network-tools`）     |
| `system_proxy.rs` | 一键系统代理：macOS `networksetup`（osascript 提权）/ Windows HKCU；含备份还原 |

关键约定：

- **绑定与生命周期**：`TcpListener::bind(("127.0.0.1", port))` 自行绑定（端口占用返回明确错误）；`with_graceful_shutdown(oneshot)` 停止；start/stop 幂等。
- **请求-响应配对**：hudsucker 对每个请求克隆一份 handler，`handle_request`/`handle_response`/`handle_error` 同一实例 → 用实例内 `pending: Option<PendingFlow>` 配对（不依赖 `client_addr`）；仅 HTTP/1.1，**不启用 http2**。
- **抓体**：`TeeBody` 边转发边复制到 `BodySink`（上限 `maxBodyKb`）；响应体流结束触发 `on_end` 回调落库。**不改转发字节与头部**。
- **事件**：批量（约 200ms）发 `network-tools://flow` / `network-tools://ws`（插件私有，绕过 `core/events`）；前端用 `@tauri-apps/api/event` 的 `listen` 订阅。
- **解压**：`gzip/deflate/br/zstd` 解压**副本**供展示，不影响转发；字符集用 `encoding_rs`。
- **二进制重放**：`network_tools_flow_body_temp` 把捕获体写入缓存目录返回路径，前端设为 `binary` 体类型。
- **系统代理**：`network_tools_system_proxy_enable/disable/status`；设置前把原代理配置备份到
  `app_data_dir/network-tools/system-proxy.json`，还原时按备份恢复；`stop()` 会自动还原。
  macOS 经 osascript 弹框提权，Windows 走 HKCU 注册表（免管理员）。
  `frontend/setup.ts` 启动自检：若发现系统代理处于托管态但代理未运行（多为上次强杀残留），
  自动还原，避免网络被指向死端口。
- **限制**：无上游代理 / 不支持 mTLS / 不启用 HTTP/2；证书固定的 App 不可拦截；
  忽略系统代理的应用（部分 Electron/Java/游戏）抓不到；真正「全流量」需 TUN 透明代理（未实现）。

> 依赖属本地层（`hudsucker` + `http-body-util` + `flate2`/`brotli`/`zstd`），
> 注意 hudsucker 会间接引入 `aws-lc-sys`（见 `LOCAL.md`）。

## 前端

- `frontend/views/HttpClient.vue`：单工作区（左集合/历史，右请求 + 响应）。
- 数据层 composable（`frontend/composables/`）：`useSend` / `useCollections` / `useHistory` / `useEnvironments` / `useDraft`。
- 共享模型：`frontend/shared.ts`（`HttpRequestSpec` / `HttpResponseView` / 设置默认值 / `buildSendOptions`）。
  发送页与未来拦截页共用同一模型与组件。
- 纯逻辑：`frontend/curl.ts`（cURL 互转）、`frontend/variables.ts`（变量替换）、`frontend/transfer.ts`（自有 JSON 导入导出格式）；
  **不得**引入 Tauri 依赖，以保证 vitest（node 环境）可测。格式化错误请用 `frontend/error.ts`（可依赖 `@/core/errors`）。
- 设置单例：`frontend/settings-store.ts` 的 `httpSettings`（发送页与设置页共用，勿各自 `useToolSettings`）。
- 组件（`frontend/components/`）一律 props 驱动、无业务状态，拦截页复用：
  `KeyValueEditor` / `BodyEditor` / `AuthEditor` / `RequestTabs` / `ResponsePanel` / `CollectionsSidebar` / `CurlDialog`。
- 请求拦截前端：`views/Interceptor.vue` + `components/intercept/`（`FlowList`/`FlowDetail`/`WsDetail`/`CaDialog`/`PortDialog`）
  - `composables/useInterceptor.ts`（状态/命令/事件订阅）；纯逻辑在 `intercept-shared.ts`（过滤、`flowToSpec`、`flowToResponseView`）。
- 启动钩子：`frontend/setup.ts`（框架启动期调用，见「系统代理」自检）。
- 公共 UI 助手集中在 `frontend/shared.ts`：`httpMethodClass` / `statusBadgeVariant` / `urlPath` / `copyToClipboard`，勿在组件内重复实现。

## 约定要点

- **Cookie**：无自动 cookie jar（`cookieMode: 'none'`）。Cookie 编辑器为唯一真源；若手动写了 `Cookie` 头，
  发送时与编辑器内容合并（`buildSendOptions`）。响应 `Set-Cookie` 仅展示、可一键复制回编辑器。
- **认证**：发送时合成 Authorization/API Key；若手写同名头，以手写优先并给出警示。
- **Content-Type**：显式请求头优先，否则按 Body 类型自动带；multipart 的 Content-Type 由框架生成（忽略用户覆盖）。
- **变量**：仅解析启用字段；存在未解析变量时**阻止发送**。
- **历史**：存发送时已替换变量的请求快照；响应体按阈值截断存储。

## 设置项

`frontend/shared.ts` 的 `HttpSettings`，经 `useToolSettings` 存于 `tools.network-tools`（单例见 `settings-store.ts`）。

- 接口测试：`timeoutMs` / `followRedirects` / `verifySsl` / `maxResponseKb` / `historyEnabled` /
  `historyLimit` / `proxyMode` / `proxyUrl` / `defaultUserAgent` / `activeEnvironmentId`
- 请求拦截：`interceptorPort`（0 = 首次启动时询问）/ `interceptorRecordBodies` /
  `interceptorMaxBodyKb` / `interceptorMaxFlows` / `interceptorMaxWsFrames` /
  `interceptorAutoSystemProxy`

## 校验

```bash
pnpm lint && pnpm build                          # 前端
pnpm fmt:rs && pnpm lint:rs && pnpm test:rs      # Rust（intercept 引擎 + 迁移聚合）
pnpm test                                        # vitest（curl / 变量 / 导入导出 / 拦截纯逻辑）
```

端到端冒烟（ignored，需网络/回环）：

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib proxy_ -- --ignored
```
